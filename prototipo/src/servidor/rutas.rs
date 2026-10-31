use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::paquetes::MetadatosPaquete;
use crate::servidor::modelo::MetadatosPublicacion;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub artifacts_dir: String,
    pub registry_id: String,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/packages/:namespace/:name", get(info_paquete))
        .route(
            "/api/v1/packages/:namespace/:name/:version",
            get(info_version),
        )
        .route(
            "/api/v1/packages/:namespace/:name/:version/download",
            get(descargar),
        )
        .route("/api/v1/publish", post(publicar))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status":"ok"})))
}

async fn info_paquete(
    State(state): State<AppState>,
    Path((ns, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT version FROM versions v JOIN packages p ON p.id=v.package_id WHERE p.namespace=$1 AND p.name=$2 AND v.yanked=false ORDER BY v.created_at",
    )
    .bind(&ns)
    .bind(&name)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();
    (
        StatusCode::OK,
        Json(json!({"namespace":ns,"name":name,"versions":rows})),
    )
}

async fn info_version(
    State(state): State<AppState>,
    Path((ns, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT v.sha256, v.registry_id, v.key_id, v.signature, v.nonce, v.expires_at FROM versions v JOIN packages p ON p.id=v.package_id WHERE p.namespace=$1 AND p.name=$2 AND v.version=$3",
    )
    .bind(&ns)
    .bind(&name)
    .bind(&version)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    match row {
        Some((sha, rid, kid, sig, nonce, exp)) => {
            let meta = MetadatosPaquete {
                namespace: ns.clone(),
                name: name.clone(),
                version: version.clone(),
                sha256: sha,
                published_at: "2026-10-31T00:00:00Z".to_string(),
                registry_id: rid,
                key_id: kid,
                nonce,
                expires_at: exp,
                firma: Some(sig),
                clave: None,
            };
            (StatusCode::OK, Json(serde_json::to_value(meta).unwrap()))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error":"no encontrado"})),
        ),
    }
}

async fn descargar(
    State(state): State<AppState>,
    Path((ns, name, version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let path: Option<String> = sqlx::query_scalar(
        "SELECT v.artifact_path FROM versions v JOIN packages p ON p.id=v.package_id WHERE p.namespace=$1 AND p.name=$2 AND v.version=$3",
    )
    .bind(&ns)
    .bind(&name)
    .bind(&version)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    match path {
        Some(p) => {
            let bytes = std::fs::read(&p).unwrap_or_default();
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
                bytes,
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "no encontrado").into_response(),
    }
}

async fn publicar(
    State(state): State<AppState>,
    Json(req): Json<PublishRequest>,
) -> impl IntoResponse {
    // Validar BPK
    if let Err(e) = crate::cliente_remoto::validar_bpk(&req.artifact_b64_decoded()) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": e})));
    }
    // Verificar hash
    let sha = crate::paquetes::sha256_hex(&req.artifact_b64_decoded());
    if sha != req.meta.sha256 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"hash inválido"})),
        );
    }

    // Crear/obtener package
    let pkg_id = Uuid::new_v4();
    let _ = sqlx::query(
        "INSERT INTO packages (id, namespace, name) VALUES ($1, $2, $3) ON CONFLICT (namespace, name) DO NOTHING",
    )
    .bind(pkg_id)
    .bind(&req.meta.namespace)
    .bind(&req.meta.name)
    .execute(&state.pool)
    .await;

    let pkg_id: Uuid = sqlx::query_scalar("SELECT id FROM packages WHERE namespace=$1 AND name=$2")
        .bind(&req.meta.namespace)
        .bind(&req.meta.name)
        .fetch_one(&state.pool)
        .await
        .unwrap();

    // Verificar versión duplicada
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM versions WHERE package_id=$1 AND version=$2)",
    )
    .bind(pkg_id)
    .bind(&req.meta.version)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);
    if exists {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error":"versión duplicada"})),
        );
    }

    // Guardar artefacto
    let artifact_path = format!(
        "{}/{}_{}_{}.bpk",
        state.artifacts_dir, req.meta.namespace, req.meta.name, req.meta.version
    );
    if std::fs::write(&artifact_path, req.artifact_b64_decoded()).is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"error de almacenamiento"})),
        );
    }

    // Insertar versión
    let ver_id = Uuid::new_v4();
    let res = sqlx::query(
        "INSERT INTO versions (id, package_id, version, sha256, registry_id, key_id, signature, nonce, expires_at, artifact_path) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(ver_id)
    .bind(pkg_id)
    .bind(&req.meta.version)
    .bind(&req.meta.sha256)
    .bind(&req.meta.registry_id)
    .bind(&req.meta.key_id)
    .bind(req.meta.firma.clone())
    .bind(&req.meta.nonce)
    .bind(&req.meta.expires_at)
    .bind(&artifact_path)
    .execute(&state.pool)
    .await;

    if res.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"error de persistencia"})),
        );
    }

    // Audit
    let _ =
        sqlx::query("INSERT INTO audit (id, action, actor, detail) VALUES ($1,'publish',NULL,$2)")
            .bind(Uuid::new_v4())
            .bind(format!(
                "{}/{}@{}",
                req.meta.namespace, req.meta.name, req.meta.version
            ))
            .execute(&state.pool)
            .await;

    (StatusCode::CREATED, Json(json!({"status":"publicado"})))
}

#[derive(serde::Deserialize)]
struct PublishRequest {
    meta: MetadatosPublicacion,
    artifact_b64: String,
}

impl PublishRequest {
    fn artifact_b64_decoded(&self) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(&self.artifact_b64)
            .unwrap_or_default()
    }
}
