use cronbird::{
    adapters::{FilePersister, InMemoryStore},
    config::Config,
    domain::CallbackStore,
    http::{AuthMiddleware, build_router},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " #1");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let config = Config::from_env()?;
    config.validate()?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!(
        r#"
                         _   _       _
         ___ ___ ___ ___| |_|_|___ _| |
        |  _|  _| . |   | . | |  _| . |
        |___|_| |___|_|_|___|_|_| |___|

          v{}
        "#,
        VERSION
    );

    tracing::info!("cronbird {} starting", VERSION);
    tracing::debug!("Environment variables and configuration loaded");
    tracing::info!("Listen address: {}", config.listen_addr);
    tracing::info!("Allow dynamic identities: {}", config.allow_dynamic);
    tracing::info!(
        "Authentication: {}",
        if config.auth_token.is_some() {
            "enabled"
        } else {
            "disabled"
        }
    );
    tracing::info!(
        "Metrics authentication: {}",
        if config.protect_metrics && config.auth_token.is_some() {
            "enabled"
        } else if config.protect_metrics {
            "requested but no token set (metrics remain public)"
        } else {
            "disabled"
        }
    );
    tracing::info!("Persist path: {}", config.persist_path.display());
    tracing::info!("Persist interval: {}s", config.persist_interval);

    tracing::debug!("Initializing in-memory store and file persister");
    let store = InMemoryStore::new(config.allowed_identities());

    let persister = Arc::new(FilePersister::new(
        config.persist_path.clone(),
        config.persist_interval,
    ));

    tracing::info!("Loading initial state from persistence");
    match persister.load_initial_state().await {
        Ok(snapshot) => {
            if !snapshot.is_empty() {
                tracing::info!("Loading {} records from persist file", snapshot.len());
                store
                    .load_snapshot(snapshot)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to load snapshot: {}", e))?;
            } else {
                tracing::debug!("Persistence file is empty, starting with clean state");
            }
        }
        Err(e) => {
            tracing::warn!("Failed to load initial state: {}. Starting fresh.", e);
        }
    }

    tracing::debug!("Spawning background persistence loop");
    let persist_store = store.clone();
    let persist_handle = {
        let persister = Arc::clone(&persister);
        tokio::spawn(async move {
            persister.run_persist_loop(persist_store).await;
        })
    };

    tracing::debug!("Building HTTP router");
    let auth = AuthMiddleware::new(config.auth_token.clone());
    let app = build_router(store.clone(), auth, config.protect_metrics);

    tracing::debug!("Binding to {}", config.listen_addr);
    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", config.listen_addr, e))?;

    tracing::info!("Server listening on {}", config.listen_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(persister, store, persist_handle))
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    tracing::info!("Server shutdown complete");
    Ok(())
}

/// Handles shutdown signals and performs cleanup.
async fn shutdown_signal(
    persister: Arc<FilePersister>,
    store: InMemoryStore,
    persist_handle: tokio::task::JoinHandle<()>,
) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received SIGINT, shutting down gracefully");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down gracefully");
        },
    }

    tracing::debug!("Aborting background persistence loop task");
    persist_handle.abort();

    tracing::info!("Performing final state persistence before exit");
    match tokio::time::timeout(Duration::from_secs(5), persister.final_persist(store)).await {
        Ok(Ok(())) => tracing::info!("Final persist completed"),
        Ok(Err(e)) => tracing::error!("Final persist failed: {}", e),
        Err(_) => tracing::error!("Final persist timed out"),
    }
}
