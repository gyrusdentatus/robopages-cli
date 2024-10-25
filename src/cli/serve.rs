use std::collections::HashMap;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::web;
use actix_web::App;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use camino::Utf8PathBuf;

use crate::book::flavors::Flavor;
use crate::book::{
    flavors::{nerve, openai},
    Book,
};
use crate::runtime;

struct AppState {
    max_running_tasks: usize,
    book: Arc<Book>,
}

async fn not_found() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::NotFound().body("nope"))
}

async fn serve_pages_impl(
    state: web::Data<Arc<AppState>>,
    query: web::Query<HashMap<String, String>>,
    filter: Option<String>,
) -> actix_web::Result<HttpResponse> {
    let flavor = Flavor::from_map_or_default(&query)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    if flavor.is_nerve() {
        Ok(HttpResponse::Ok().json(state.book.as_tools::<nerve::FunctionGroup>(filter)))
    } else {
        Ok(HttpResponse::Ok().json(state.book.as_tools::<openai::Tool>(filter)))
    }
}

async fn serve_pages_with_filter(
    state: web::Data<Arc<AppState>>,
    query: web::Query<HashMap<String, String>>,
    actix_web_lab::extract::Path((filter,)): actix_web_lab::extract::Path<(String,)>,
) -> actix_web::Result<HttpResponse> {
    serve_pages_impl(state, query, Some(filter)).await
}

async fn serve_pages(
    state: web::Data<Arc<AppState>>,
    query: web::Query<HashMap<String, String>>,
) -> actix_web::Result<HttpResponse> {
    serve_pages_impl(state, query, None).await
}

async fn process_calls(
    state: web::Data<Arc<AppState>>,
    calls: web::Json<Vec<openai::Call>>,
) -> actix_web::Result<HttpResponse> {
    match runtime::execute(false, state.book.clone(), calls.0, state.max_running_tasks).await {
        Ok(resp) => Ok(HttpResponse::Ok().json(resp)),
        Err(e) => Err(actix_web::error::ErrorBadRequest(e)),
    }
}

pub(crate) async fn serve(
    path: Utf8PathBuf,
    filter: Option<String>,
    address: String,
    lazy: bool,
    max_running_tasks: usize,
) -> anyhow::Result<()> {
    if !address.contains("127.0.0.1:") && !address.contains("localhost:") {
        log::warn!("external address specified, this is an unsafe configuration as no authentication is provided");
    }

    let book = Arc::new(Book::from_path(path, filter)?);
    if !lazy {
        for page in book.pages.values() {
            for (func_name, func) in page.functions.iter() {
                if let Some(container) = &func.container {
                    log::info!("pre building container for function {} ...", func_name);
                    container.source.resolve().await?;
                }
            }
        }
    }

    let max_running_tasks = if max_running_tasks == 0 {
        std::thread::available_parallelism()?.into()
    } else {
        max_running_tasks
    };

    log::info!(
        "serving {} pages on http://{address} with {max_running_tasks} max running tasks",
        book.size(),
    );

    let app_state = Arc::new(AppState {
        max_running_tasks,
        book,
    });

    HttpServer::new(move || {
        let cors = Cors::default().max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(app_state.clone()))
            .route("/process", web::post().to(process_calls))
            // TODO: is this is the best way to do this? can't find a clean way to have an optional path parameter
            .service(web::resource("/{filter}").route(web::get().to(serve_pages_with_filter)))
            .service(web::resource("/").route(web::get().to(serve_pages)))
            .default_service(web::route().to(not_found))
            .wrap(actix_web::middleware::Logger::default())
    })
    .bind(&address)
    .map_err(|e| anyhow!(e))?
    .run()
    .await
    .map_err(|e| anyhow!(e))
}
