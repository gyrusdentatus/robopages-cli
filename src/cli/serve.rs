use std::sync::Arc;

use actix_cors::Cors;
use actix_web::web;
use actix_web::App;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use camino::Utf8PathBuf;

use crate::book::openai;
use crate::book::Book;
use crate::runtime;

async fn not_found() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::NotFound().body("nope"))
}

async fn serve_pages_with_filter(
    book: web::Data<Arc<Book>>,
    _: HttpRequest,
    actix_web_lab::extract::Path((filter,)): actix_web_lab::extract::Path<(String,)>,
) -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(book.as_tools(Some(filter))))
}

async fn serve_pages(
    book: web::Data<Arc<Book>>,
    _: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(book.as_tools(None)))
}

async fn process_calls(
    book: web::Data<Arc<Book>>,
    _: HttpRequest,
    calls: web::Json<Vec<openai::Call>>,
) -> actix_web::Result<HttpResponse> {
    match runtime::execute(false, book.get_ref().clone(), calls.0).await {
        Ok(resp) => Ok(HttpResponse::Ok().json(resp)),
        Err(e) => Err(actix_web::error::ErrorBadRequest(e)),
    }
}

pub(crate) async fn serve(
    path: Utf8PathBuf,
    filter: Option<String>,
    address: String,
    lazy: bool,
    workers: usize,
) -> anyhow::Result<()> {
    if !address.contains("127.0.0.1:") && !address.contains("localhost:") {
        log::warn!("external address specified, this is an unsafe configuration as no authentication is provided");
    }

    let workers = if workers == 0 {
        std::thread::available_parallelism()?.into()
    } else {
        workers
    };

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

    log::info!(
        "serving {} pages on http://{address} with {workers} workers",
        book.size(),
    );

    // TODO: add minimal web ui
    HttpServer::new(move || {
        let cors = Cors::default().max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(book.clone()))
            .route("/process", web::post().to(process_calls))
            // TODO: is this is the best way to do this? can't find a clean way to have an optional path parameter
            .service(web::resource("/{filter}").route(web::get().to(serve_pages_with_filter)))
            .service(web::resource("/").route(web::get().to(serve_pages)))
            .default_service(web::route().to(not_found))
            .wrap(actix_web::middleware::Logger::default())
    })
    .bind(&address)
    .map_err(|e| anyhow!(e))?
    .workers(workers)
    .run()
    .await
    .map_err(|e| anyhow!(e))
}
