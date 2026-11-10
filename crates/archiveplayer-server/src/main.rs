use archiveplayer_core::Service;
use archiveplayer_server::{Config, app_state, build_router};
use std::error::Error;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("archiveplayer_server=info,tower_http=info")),
        )
        .init();

    let config = Config::from_env().unwrap_or_else(|error| {
        eprintln!("Configuration error: {error}");
        std::process::exit(2);
    });

    let index = config.static_dir.join("index.html");
    if !index.is_file() {
        eprintln!(
            "Static frontend not found at {}. Run `pnpm build` first or set ARCHIVEPLAYER_STATIC_DIR.",
            index.display()
        );
        std::process::exit(2);
    }

    let service = Service::new(config.data_dir.clone())?;
    tracing::info!(
        "ArchivePlayer server {} using data directory {}",
        env!("CARGO_PKG_VERSION"),
        service.data_dir.display()
    );

    let listener = TcpListener::bind(config.bind).await?;
    tracing::info!("listening on http://{}", config.bind);
    tracing::info!("serving frontend from {}", config.static_dir.display());

    axum::serve(listener, build_router(app_state(service, config)))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::warn!("could not install SIGTERM handler: {error}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutting down");
}
