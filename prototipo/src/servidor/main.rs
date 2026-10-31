use axum::serve;
use sqlx::PgPool;
use tokio::net::TcpListener;

use crate::servidor::config::ConfigServidor;
use crate::servidor::db::{crear_pool, migrar};
use crate::servidor::rutas::{AppState, router};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = ConfigServidor::from_env()?;
    let pool: PgPool = crear_pool(&cfg.database_url).await?;
    migrar(&pool).await?;

    std::fs::create_dir_all(&cfg.artifacts_dir)?;

    let state = AppState {
        pool,
        artifacts_dir: cfg.artifacts_dir.clone(),
        registry_id: cfg.registry_id,
    };
    let app = router(state);
    let listener = TcpListener::bind(&cfg.bind).await?;
    tracing::info!("servidor escuchando en {}", cfg.bind);
    serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
