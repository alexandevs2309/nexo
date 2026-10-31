use axum::serve;
use sqlx::PgPool;
use tokio::net::TcpListener;

use crate::servidor::auth;
use crate::servidor::config::ConfigServidor;
use crate::servidor::db::{crear_pool, migrar};
use crate::servidor::rutas::{AppState, router};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = ConfigServidor::from_env()?;
    let pool: PgPool = crear_pool(&cfg.database_url).await?;
    migrar(&pool).await?;

    std::fs::create_dir_all(&cfg.artifacts_dir)?;

    // Bootstrap seguro del primer administrador (solo si no existe ninguno).
    if let (Ok(user), Ok(pass)) = (
        std::env::var("BASILISCO_BOOTSTRAP_ADMIN_USERNAME"),
        std::env::var("BASILISCO_BOOTSTRAP_ADMIN_PASSWORD"),
    ) {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role='admin'")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
        if count == 0 {
            match auth::crear_usuario(&pool, &user, &pass, "admin").await {
                Ok(_) => tracing::info!("administrador bootstrap creado: {}", user),
                Err(e) => tracing::warn!("bootstrap admin falló: {}", e),
            }
        }
    }

    // Identidad criptográfica del registro: generar clave si no existe.
    bootstrap_registry_key(&pool, &cfg).await;

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

async fn bootstrap_registry_key(pool: &PgPool, cfg: &ConfigServidor) {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM keys WHERE registry_id=$1")
        .bind(&cfg.registry_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    if count > 0 {
        return;
    }
    // generar par Ed25519
    use ed25519_dalek::SigningKey;
    let mut seed = [0u8; 32];
    let u1 = uuid::Uuid::new_v4();
    let u2 = uuid::Uuid::new_v4();
    seed[..16].copy_from_slice(u1.as_bytes());
    seed[16..].copy_from_slice(u2.as_bytes());
    let sk = SigningKey::from_bytes(&seed);
    let vk = sk.verifying_key();
    let key_id = "key-1".to_string();
    let _ = sqlx::query(
        "INSERT INTO keys (id, registry_id, key_id, public_key, valid_from) VALUES ($1,$2,$3,$4,now())",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(&cfg.registry_id)
    .bind(&key_id)
    .bind(vk.to_bytes().to_vec())
    .execute(pool)
    .await;
    // guardar clave privada en archivo (no en git)
    let path = std::path::PathBuf::from(&cfg.artifacts_dir).join("registry.key");
    let _ = std::fs::write(&path, sk.to_bytes());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    tracing::info!("clave de registro generada: {}", key_id);
}
