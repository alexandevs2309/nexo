use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usuario {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadatosPublicacion {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub registry_id: String,
    pub key_id: String,
    pub nonce: String,
    pub expires_at: String,
    pub firma: String,
    pub clave: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespuestaPaquete {
    pub namespace: String,
    pub name: String,
    pub versions: Vec<String>,
}
