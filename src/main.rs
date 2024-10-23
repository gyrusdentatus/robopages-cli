#[macro_use]
extern crate anyhow;

use clap::Parser;
use cli::Args;

mod book;
mod cli;
mod runtime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if std::env::var_os("RUST_LOG").is_none() {
        // set `RUST_LOG=debug` to see debug logs
        // NOTE: actix_server is waaaay too verbose at the info level -.-
        std::env::set_var("RUST_LOG", "info,actix_server=warn");
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_module_path(false)
        .format_target(false)
        .init();

    let result = match args.command {
        cli::Command::Install { source, path } => cli::install(source, path).await,
        cli::Command::Create { name } => cli::create(name).await,
        cli::Command::View { path, filter } => cli::view(path, filter).await,
        cli::Command::Serve {
            path,
            filter,
            address,
            lazy,
            workers,
        } => cli::serve(path, filter, address, lazy, workers).await,
        cli::Command::Run {
            path,
            function,
            auto,
        } => cli::run(path, function, auto).await,
    };

    if let Err(e) = result {
        log::error!("{:?}", e);
        std::process::exit(1);
    }

    Ok(())
}
