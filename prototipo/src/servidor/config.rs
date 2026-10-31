use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ConfigServidor {
    pub bind: String,
    pub database_url: String,
    pub artifacts_dir: String,
    pub registry_id: String,
}

impl ConfigServidor {
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL es obligatorio para iniciar el registro".to_string())?;
        let artifacts_dir =
            env::var("BASILISCO_ARTIFACTS").unwrap_or_else(|_| "./artifacts".to_string());
        let artifacts_dir = PathBuf::from(artifacts_dir);
        if artifacts_dir.as_os_str().is_empty() {
            return Err("BASILISCO_ARTIFACTS no puede estar vacío".into());
        }
        Ok(Self {
            bind: env::var("BASILISCO_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            database_url,
            artifacts_dir: artifacts_dir.to_string_lossy().into_owned(),
            registry_id: env::var("BASILISCO_REGISTRY_ID")
                .map_err(|_| "BASILISCO_REGISTRY_ID es obligatorio".to_string())?,
        })
    }
}
