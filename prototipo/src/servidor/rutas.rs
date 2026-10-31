use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::io::Read;
use uuid::Uuid;

use crate::paquetes::MetadatosPaquete;
use crate::servidor::auth;
use crate::servidor::errores::{ApiError, ApiResult};
use crate::servidor::middleware::{AuthUser, require_role};
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
        .route("/api/v1/health", get(health))
        // auth
        .route("/api/v1/auth/token", post(auth_token))
        .route("/api/v1/auth/revoke", post(auth_revoke))
        .route("/api/v1/users/me", get(users_me))
        .route("/api/v1/users", post(users_create))
        // namespaces
        .route("/api/v1/namespaces", post(ns_create))
        .route("/api/v1/namespaces/{name}", get(ns_info))
        .route("/api/v1/namespaces/{name}/members", post(ns_add_member))
        .route("/api/v1/namespaces/{name}/packages", get(ns_packages))
        // packages
        .route("/api/v1/packages/{namespace}/{name}", get(info_paquete))
        .route(
            "/api/v1/packages/{namespace}/{name}/versions",
            get(info_versiones),
        )
        .route(
            "/api/v1/packages/{namespace}/{name}/{version}",
            get(info_version),
        )
        .route(
            "/api/v1/packages/{namespace}/{name}/{version}/download",
            get(descargar),
        )
        .route(
            "/api/v1/packages/{namespace}/{name}/{version}/yank",
            post(yank_version),
        )
        .route("/api/v1/search", get(buscar))
        // publish (single-step, R1-compatible alias)
        .route("/api/v1/publish", post(publicar))
        .route("/api/v1/packages", post(publicar))
        // keys
        .route("/api/v1/keys", get(listar_claves))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status":"ok"})))
}

// ---------- auth ----------

#[derive(Deserialize)]
struct LoginReq {
    username: String,
    password: String,
}

async fn auth_token(
    State(state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> ApiResult<impl IntoResponse> {
    let uid = auth::autenticar(&state.pool, &req.username, &req.password)
        .await
        .map_err(ApiError::unauthorized)?;
    let token = auth::emitir_token(&state.pool, uid, 30)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(erros_ok(json!({"token": token, "token_type": "Bearer"})))
}

fn erros_ok(v: serde_json::Value) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(json!({"success":true,"data":v,"error":null})),
    )
}

async fn auth_revoke(
    State(state): State<AppState>,
    user: AuthUser,
    headers: axum::http::HeaderMap,
) -> ApiResult<impl IntoResponse> {
    // revoke the token presented
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");
    let _ = user;
    auth::revocar_token(&state.pool, token)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok((
        StatusCode::OK,
        Json(json!({"success":true,"data":null,"error":null})),
    ))
}

async fn users_me(State(state): State<AppState>, user: AuthUser) -> ApiResult<impl IntoResponse> {
    let row: Option<(String,)> = sqlx::query_as("SELECT username FROM users WHERE id=$1")
        .bind(user.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let username = row.map(|r| r.0).unwrap_or_default();
    Ok(erros_ok(
        json!({"id": user.user_id, "username": username, "role": user.role}),
    ))
}

#[derive(Deserialize)]
struct CreateUserReq {
    username: String,
    password: String,
    #[serde(default = "default_role")]
    role: String,
}

fn default_role() -> String {
    "reader".to_string()
}

async fn users_create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateUserReq>,
) -> ApiResult<impl IntoResponse> {
    require_role(&user, &["admin"])?;
    if req.password.len() < 8 {
        return Err(ApiError::bad_request(
            "INVALID_INPUT",
            "contraseña demasiado corta",
        ));
    }
    if !matches!(req.role.as_str(), "admin" | "reader" | "publisher") {
        return Err(ApiError::bad_request("INVALID_INPUT", "rol inválido"));
    }
    // Only admin can create admin (already admin)
    let id = auth::crear_usuario(&state.pool, &req.username, &req.password, &req.role)
        .await
        .map_err(|e| {
            if e.contains("unique") || e.contains("duplicate") {
                ApiError::conflict("USER_EXISTS", "usuario ya existe")
            } else {
                ApiError::internal(e)
            }
        })?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"success":true,"data":{"id":id},"error":null})),
    ))
}

// ---------- namespaces ----------

#[derive(Deserialize)]
struct NsCreateReq {
    name: String,
}

async fn ns_create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<NsCreateReq>,
) -> ApiResult<impl IntoResponse> {
    require_role(&user, &["admin", "publisher", "reader"])?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO namespaces (id, name, owner_id) VALUES ($1,$2,$3)")
        .bind(id)
        .bind(&req.name)
        .bind(user.user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                ApiError::conflict("NAMESPACE_EXISTS", "namespace ya existe")
            } else {
                ApiError::internal(e.to_string())
            }
        })?;
    // owner member
    let _ = sqlx::query(
        "INSERT INTO namespace_members (namespace_id, user_id, role) VALUES ($1,$2,'owner') ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(user.user_id)
    .execute(&state.pool)
    .await;
    Ok((
        StatusCode::CREATED,
        Json(json!({"success":true,"data":{"name":req.name},"error":null})),
    ))
}

async fn ns_info(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let row: Option<(Uuid, Uuid, chrono::DateTime<chrono::Utc>)> =
        sqlx::query_as("SELECT id, owner_id, created_at FROM namespaces WHERE name=$1")
            .bind(&name)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
    let (id, owner, created) =
        row.ok_or_else(|| ApiError::not_found("NAMESPACE_NOT_FOUND", "namespace no encontrado"))?;
    let pkgs: Vec<String> =
        sqlx::query_scalar("SELECT name FROM packages WHERE namespace=$1 ORDER BY name")
            .bind(&name)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();
    Ok(erros_ok(json!({
        "id": id,
        "name": name,
        "owner_id": owner,
        "created_at": created,
        "packages": pkgs
    })))
}

#[derive(Deserialize)]
struct MemberReq {
    user_id: Uuid,
    role: String,
}

async fn ns_add_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path(name): Path<String>,
    Json(req): Json<MemberReq>,
) -> ApiResult<impl IntoResponse> {
    // only owner or admin
    let ns: Option<(Uuid, Uuid)> =
        sqlx::query_as("SELECT id, owner_id FROM namespaces WHERE name=$1")
            .bind(&name)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
    let (ns_id, owner_id) =
        ns.ok_or_else(|| ApiError::not_found("NAMESPACE_NOT_FOUND", "namespace no encontrado"))?;
    if user.role != "admin" && user.user_id != owner_id {
        return Err(ApiError::forbidden("solo owner/admin"));
    }
    if !matches!(req.role.as_str(), "owner" | "publisher" | "reader") {
        return Err(ApiError::bad_request("INVALID_INPUT", "rol inválido"));
    }
    sqlx::query(
        "INSERT INTO namespace_members (namespace_id, user_id, role) VALUES ($1,$2,$3) \
         ON CONFLICT (namespace_id, user_id) DO UPDATE SET role=EXCLUDED.role",
    )
    .bind(ns_id)
    .bind(req.user_id)
    .bind(&req.role)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"success":true,"data":null,"error":null})),
    ))
}

async fn ns_packages(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM packages WHERE namespace=$1 ORDER BY name")
            .bind(&name)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(erros_ok(
        json!({"namespace": name, "packages": rows.into_iter().map(|r| r.0).collect::<Vec<_>>() }),
    ))
}

// ---------- packages ----------

async fn info_paquete(
    State(state): State<AppState>,
    Path((ns, name)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT v.version FROM versions v JOIN packages p ON p.id=v.package_id \
         WHERE p.namespace=$1 AND p.name=$2 AND v.yanked=false ORDER BY v.published_at",
    )
    .bind(&ns)
    .bind(&name)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    // ensure package exists
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM packages WHERE namespace=$1 AND name=$2)")
            .bind(&ns)
            .bind(&name)
            .fetch_one(&state.pool)
            .await
            .unwrap_or(false);
    if !exists {
        return Err(ApiError::not_found(
            "PACKAGE_NOT_FOUND",
            "paquete no encontrado",
        ));
    }
    Ok(erros_ok(
        json!({"namespace":ns,"name":name,"versions": rows.into_iter().map(|r|r.0).collect::<Vec<_>>() }),
    ))
}

async fn info_versiones(
    State(state): State<AppState>,
    Path((ns, name)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT v.version FROM versions v JOIN packages p ON p.id=v.package_id \
         WHERE p.namespace=$1 AND p.name=$2 ORDER BY v.published_at",
    )
    .bind(&ns)
    .bind(&name)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(erros_ok(
        json!({"namespace":ns,"name":name,"versions": rows.into_iter().map(|r|r.0).collect::<Vec<_>>() }),
    ))
}

async fn info_version(
    State(state): State<AppState>,
    Path((ns, name, version)): Path<(String, String, String)>,
) -> ApiResult<impl IntoResponse> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, String, String, i64, bool, String)>(
        "SELECT v.sha256, v.registry_id, v.key_id, v.signature, v.nonce, v.expires_at, v.artifact_path, v.clave, v.artifact_size, v.yanked, v.published_at \
         FROM versions v JOIN packages p ON p.id=v.package_id WHERE p.namespace=$1 AND p.name=$2 AND v.version=$3",
    )
    .bind(&ns).bind(&name).bind(&version)
    .fetch_optional(&state.pool).await.map_err(|e| ApiError::internal(e.to_string()))?;
    let (sha, rid, kid, sig, nonce, exp, _path, clave, size, yanked, published) =
        row.ok_or_else(|| ApiError::not_found("VERSION_NOT_FOUND", "versión no encontrada"))?;
    // dependencies
    let deps: serde_json::Value = sqlx::query_scalar(
        "SELECT v.dependencies FROM versions v JOIN packages p ON p.id=v.package_id WHERE p.namespace=$1 AND p.name=$2 AND v.version=$3",
    )
    .bind(&ns).bind(&name).bind(&version)
    .fetch_one(&state.pool).await.unwrap_or(serde_json::json!({}));
    let meta = MetadatosPaquete {
        namespace: ns.clone(),
        name: name.clone(),
        version: version.clone(),
        sha256: sha.clone(),
        published_at: published.clone(),
        registry_id: rid.clone(),
        key_id: kid.clone(),
        nonce: nonce.clone(),
        expires_at: exp.clone(),
        firma: Some(sig.clone()),
        clave: if clave.is_empty() {
            None
        } else {
            Some(clave.clone())
        },
    };
    let data = json!({
        "namespace": ns,
        "name": name,
        "version": version,
        "sha256": sha,
        "registry_id": rid,
        "key_id": kid,
        "nonce": nonce,
        "expires_at": exp,
        "signature": sig.clone(),
        "firma": sig.clone(),
        "clave": if clave.is_empty() { None } else { Some(clave.clone()) },
        "published_at": published.clone(),
        "size": size,
        "yanked": yanked,
        "dependencies": deps,
        "artifact": {
            "filename": format!("{}-{}.bpk", name, version),
            "size": size,
            "sha256": sha,
            "media_type": "application/vnd.basilisco.package"
        },
        "meta": meta,
    });
    Ok(erros_ok(data))
}

async fn descargar(
    State(state): State<AppState>,
    Path((ns, name, version)): Path<(String, String, String)>,
) -> ApiResult<impl IntoResponse> {
    let row: Option<(String, bool)> = sqlx::query_as(
        "SELECT v.artifact_path, v.yanked FROM versions v JOIN packages p ON p.id=v.package_id WHERE p.namespace=$1 AND p.name=$2 AND v.version=$3",
    )
    .bind(&ns).bind(&name).bind(&version)
    .fetch_optional(&state.pool).await.map_err(|e| ApiError::internal(e.to_string()))?;
    let (path, yanked) =
        row.ok_or_else(|| ApiError::not_found("VERSION_NOT_FOUND", "versión no encontrada"))?;
    if yanked {
        // allow download but could warn; keep allowed
    }
    let bytes = std::fs::read(&path)
        .map_err(|_| ApiError::not_found("ARTIFACT_NOT_FOUND", "artefacto no encontrado"))?;
    let filename = format!("{}-{}.bpk", name, version);
    let resp = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        bytes,
    );
    Ok(resp)
}

async fn yank_version(
    State(state): State<AppState>,
    user: AuthUser,
    Path((ns, name, version)): Path<(String, String, String)>,
) -> ApiResult<impl IntoResponse> {
    // authorize: admin or namespace owner/publisher
    let ns_owner: Option<Uuid> =
        sqlx::query_scalar("SELECT owner_id FROM namespaces WHERE name=$1")
            .bind(&ns)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
    let is_member: bool = if let Some(oid) = ns_owner {
        if user.user_id == oid || user.role == "admin" {
            true
        } else {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM namespace_members nm JOIN namespaces n ON n.id=nm.namespace_id WHERE n.name=$1 AND nm.user_id=$2 AND nm.role IN ('owner','publisher'))",
            )
            .bind(&ns).bind(user.user_id).fetch_one(&state.pool).await.unwrap_or(false)
        }
    } else {
        user.role == "admin"
    };
    if !is_member {
        return Err(ApiError::forbidden("sin permiso para yank"));
    }
    let res = sqlx::query(
        "UPDATE versions SET yanked=true WHERE package_id=(SELECT id FROM packages WHERE namespace=$1 AND name=$2) AND version=$3",
    )
    .bind(&ns).bind(&name).bind(&version)
    .execute(&state.pool).await.map_err(|e| ApiError::internal(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found(
            "VERSION_NOT_FOUND",
            "versión no encontrada",
        ));
    }
    let _ = sqlx::query("INSERT INTO audit (id, action, actor, detail) VALUES ($1,'yank',$2,$3)")
        .bind(Uuid::new_v4())
        .bind(user.user_id)
        .bind(json!({"namespace":ns,"name":name,"version":version}))
        .execute(&state.pool)
        .await;
    Ok((
        StatusCode::OK,
        Json(json!({"success":true,"data":null,"error":null})),
    ))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
}
fn default_limit() -> i64 {
    20
}

async fn buscar(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<impl IntoResponse> {
    let pattern = format!("%{}%", q.q.clone().unwrap_or_default());
    let limit = q.limit.clamp(1, 100);
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT namespace, name, description FROM packages WHERE namespace ILIKE $1 OR name ILIKE $1 OR description ILIKE $1 ORDER BY name LIMIT $2",
    )
    .bind(&pattern).bind(limit)
    .fetch_all(&state.pool).await.map_err(|e| ApiError::internal(e.to_string()))?;
    let items: Vec<_> = rows
        .into_iter()
        .map(|(ns, name, desc)| {
            json!({
                "namespace": ns, "name": name, "description": desc.unwrap_or_default()
            })
        })
        .collect();
    Ok(erros_ok(json!({"results": items})))
}

// ---------- publish ----------

#[derive(serde::Deserialize)]
struct PublishRequest {
    meta: MetadatosPublicacion,
    artifact_b64: String,
}

impl PublishRequest {
    fn artifact_decoded(&self) -> Result<Vec<u8>, String> {
        base64::engine::general_purpose::STANDARD
            .decode(&self.artifact_b64)
            .map_err(|e| e.to_string())
    }
}

async fn publicar(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<PublishRequest>,
) -> ApiResult<impl IntoResponse> {
    require_role(&user, &["admin", "publisher"])?;
    let bytes = req
        .artifact_decoded()
        .map_err(|e| ApiError::bad_request("INVALID_PACKAGE", e))?;

    // Validate BPK
    crate::cliente_remoto::validar_bpk(&bytes)
        .map_err(|e| ApiError::bad_request("INVALID_PACKAGE", e))?;

    // size limit 50MB
    if bytes.len() > 50 * 1024 * 1024 {
        return Err(ApiError::bad_request(
            "INVALID_PACKAGE",
            "artefacto demasiado grande",
        ));
    }
    if bytes.is_empty() {
        return Err(ApiError::bad_request("INVALID_PACKAGE", "artefacto vacío"));
    }

    let sha = crate::paquetes::sha256_hex(&bytes);
    if sha != req.meta.sha256 {
        return Err(ApiError::bad_request("INVALID_HASH", "hash inválido"));
    }

    // verify signature if provided (R3): firma requerida
    if req.meta.firma.is_empty() || req.meta.clave.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_SIGNATURE",
            "firma requerida",
        ));
    }
    let firma = req.meta.firma.clone();
    let clave_bytes =
        hex_decode(&req.meta.clave).map_err(|e| ApiError::bad_request("INVALID_SIGNATURE", e))?;
    if clave_bytes.len() != 32 {
        return Err(ApiError::bad_request("INVALID_SIGNATURE", "clave inválida"));
    }
    let mp = crate::paquetes::MetadatosPaquete {
        namespace: req.meta.namespace.clone(),
        name: req.meta.name.clone(),
        version: req.meta.version.clone(),
        sha256: req.meta.sha256.clone(),
        published_at: req.meta.published_at.clone(),
        registry_id: req.meta.registry_id.clone(),
        key_id: req.meta.key_id.clone(),
        nonce: req.meta.nonce.clone(),
        expires_at: req.meta.expires_at.clone(),
        firma: Some(firma.clone()),
        clave: Some(req.meta.clave.clone()),
    };
    // published_at comes from meta but MetadatosPaquete uses published_at field; use provided? MetadatosPublicacion has no published_at. Keep placeholder; signature canonical uses published_at. Need match client. Use req.meta? no field. We'll use the same value client used? Client sets published_at. Server MetadatosPublicacion lacks it. TODO: add field. For now verify with placeholder would fail. Instead skip strict published_at?
    // To keep verification working, override published_at in mp is wrong. Add field to MetadatosPublicacion instead.
    crate::paquetes::verificar_firma_metadatos(&mp, &firma, &clave_bytes)
        .map_err(|e| ApiError::bad_request("INVALID_SIGNATURE", e))?;

    // validate registry_id matches
    if req.meta.registry_id != state.registry_id {
        return Err(ApiError::bad_request(
            "INVALID_METADATA",
            "registry_id no coincide",
        ));
    }

    // validate expiry not in past
    // expires_at is string; best-effort parse RFC3339
    if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(&req.meta.expires_at)
        && exp < chrono::Utc::now()
    {
        return Err(ApiError::bad_request(
            "INVALID_METADATA",
            "publicación expirada",
        ));
    }

    // namespace authorization
    authorize_namespace(&state, &user, &req.meta.namespace).await?;

    // parse manifest identity
    validate_manifest_identity(&bytes, &req.meta)?;

    // duplicate version check + package upsert in transaction
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // upsert package
    let pkg_id: Uuid = match sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO packages (id, namespace, name, description) VALUES ($1,$2,$3,$4) \
         ON CONFLICT (namespace, name) DO UPDATE SET name=EXCLUDED.name RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(&req.meta.namespace)
    .bind(&req.meta.name)
    .bind("") // description not in meta; could extend
    .fetch_one(&mut *tx)
    .await
    {
        Ok(id) => id,
        Err(_) => {
            // fallback select
            sqlx::query_scalar("SELECT id FROM packages WHERE namespace=$1 AND name=$2")
                .bind(&req.meta.namespace)
                .bind(&req.meta.name)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?
        }
    };

    // duplicate version
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM versions WHERE package_id=$1 AND version=$2)",
    )
    .bind(pkg_id)
    .bind(&req.meta.version)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;
    if exists {
        let _ = tx.rollback().await;
        return Err(ApiError::conflict(
            "VERSION_ALREADY_EXISTS",
            "versión duplicada",
        ));
    }

    // nonce uniqueness (DB constraint) - also check
    let nonce_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM versions WHERE nonce=$1)")
            .bind(&req.meta.nonce)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
    if nonce_exists {
        let _ = tx.rollback().await;
        return Err(ApiError::conflict(
            "VERSION_ALREADY_EXISTS",
            "nonce repetido",
        ));
    }

    // store artifact atomically
    let artifact_path = format!(
        "{}/{}_{}_{}.bpk",
        state.artifacts_dir, req.meta.namespace, req.meta.name, req.meta.version
    );
    let tmp_path = format!("{}.tmp", artifact_path);
    if std::fs::write(&tmp_path, &bytes).is_err() {
        let _ = tx.rollback().await;
        return Err(ApiError::internal("error de almacenamiento"));
    }
    if std::fs::rename(&tmp_path, &artifact_path).is_err() {
        let _ = tx.rollback().await;
        let _ = std::fs::remove_file(&tmp_path);
        return Err(ApiError::internal("error de almacenamiento"));
    }

    let ver_id = Uuid::new_v4();
    let deps = serde_json::json!({});
    let res = sqlx::query(
        "INSERT INTO versions (id, package_id, version, sha256, registry_id, key_id, signature, nonce, expires_at, artifact_path, artifact_size, dependencies, published_by, clave, published_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
    )
    .bind(ver_id)
    .bind(pkg_id)
    .bind(&req.meta.version)
    .bind(&req.meta.sha256)
    .bind(&req.meta.registry_id)
    .bind(&req.meta.key_id)
    .bind(&req.meta.firma)
    .bind(&req.meta.nonce)
    .bind(&req.meta.expires_at)
    .bind(&artifact_path)
    .bind(bytes.len() as i64)
    .bind(deps)
    .bind(user.user_id)
    .bind(&req.meta.clave)
    .bind(&req.meta.published_at)
    .execute(&mut *tx)
    .await;
    if res.is_err() {
        let _ = tx.rollback().await;
        let _ = std::fs::remove_file(&artifact_path);
        return Err(ApiError::internal("error de persistencia"));
    }

    let _ = sqlx::query(
        "INSERT INTO audit (id, action, actor, detail) VALUES ($1,'publish',$2,$3)",
    )
    .bind(Uuid::new_v4())
    .bind(user.user_id)
    .bind(json!({"namespace":req.meta.namespace,"name":req.meta.name,"version":req.meta.version}))
    .execute(&mut *tx)
    .await;

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"success":true,"data":{"status":"publicado"},"error":null})),
    ))
}

async fn authorize_namespace(state: &AppState, user: &AuthUser, ns: &str) -> ApiResult<()> {
    if user.role == "admin" {
        // ensure namespace exists? create implicitly? require exists
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM namespaces WHERE name=$1)")
                .bind(ns)
                .fetch_one(&state.pool)
                .await
                .unwrap_or(false);
        if !exists {
            return Err(ApiError::not_found(
                "NAMESPACE_NOT_FOUND",
                "namespace no encontrado",
            ));
        }
        return Ok(());
    }
    let ns_row: Option<(Uuid, Uuid)> =
        sqlx::query_as("SELECT id, owner_id FROM namespaces WHERE name=$1")
            .bind(ns)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;
    let (ns_id, owner_id) = ns_row
        .ok_or_else(|| ApiError::not_found("NAMESPACE_NOT_FOUND", "namespace no encontrado"))?;
    if user.user_id == owner_id {
        return Ok(());
    }
    let is_pub: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM namespace_members WHERE namespace_id=$1 AND user_id=$2 AND role IN ('owner','publisher'))",
    )
    .bind(ns_id).bind(user.user_id).fetch_one(&state.pool).await.unwrap_or(false);
    if !is_pub {
        return Err(ApiError::forbidden("sin permiso en el namespace"));
    }
    Ok(())
}

fn validate_manifest_identity(bytes: &[u8], meta: &MetadatosPublicacion) -> ApiResult<()> {
    // reuse cliente_remoto validar already called; parse manifest
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|_| ApiError::bad_request("INVALID_PACKAGE", "zip inválido"))?;
    let mut manifest = String::new();
    for i in 0..archive.len() {
        let mut f = archive
            .by_index(i)
            .map_err(|_| ApiError::bad_request("INVALID_PACKAGE", "zip inválido"))?;
        if f.name().map(|n| n == "BPK-MANIFEST.toml").unwrap_or(false) {
            f.read_to_string(&mut manifest)
                .map_err(|_| ApiError::bad_request("INVALID_PACKAGE", "manifiesto ilegible"))?;
            break;
        }
    }
    if manifest.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_MANIFEST",
            "falta manifiesto",
        ));
    }
    // check name/version appear
    let mut m_name = None;
    let mut m_version = None;
    for line in manifest.lines() {
        let l = line.trim();
        if l.starts_with("name") && l.contains('=') {
            m_name = Some(
                l.split('=')
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .to_string(),
            );
        }
        if l.starts_with("version") && l.contains('=') {
            m_version = Some(
                l.split('=')
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .to_string(),
            );
        }
    }
    if let Some(n) = m_name
        && n != meta.name
    {
        return Err(ApiError::bad_request(
            "INVALID_MANIFEST",
            "nombre no coincide",
        ));
    }
    if let Some(v) = m_version
        && v != meta.version
    {
        return Err(ApiError::bad_request(
            "INVALID_MANIFEST",
            "versión no coincide",
        ));
    }
    // namespace not in manifest typically; skip
    Ok(())
}

// ---------- keys ----------

async fn listar_claves(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, String, Vec<u8>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>)> =
        sqlx::query_as(
            "SELECT registry_id, key_id, public_key, valid_from, valid_until, revoked_at FROM keys WHERE registry_id=$1",
        )
        .bind(&state.registry_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let items: Vec<_> = rows.into_iter().map(|(rid, kid, pk, vf, vu, rv)| {
        let hex: String = pk.iter().map(|b| format!("{:02x}", b)).collect();
        json!({"registry_id": rid, "key_id": kid, "public_key": hex, "valid_from": vf, "valid_until": vu, "revoked_at": rv})
    }).collect();
    Ok(erros_ok(json!({"keys": items})))
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("hex inválido".to_string());
    }
    let mut out = Vec::new();
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "hex inválido".to_string())?;
        out.push(byte);
    }
    Ok(out)
}
