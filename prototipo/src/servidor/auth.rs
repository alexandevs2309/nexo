use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub fn hash_password(password: &str) -> Result<String, String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub async fn crear_usuario(
    pool: &PgPool,
    username: &str,
    password: &str,
    role: &str,
) -> Result<Uuid, String> {
    let id = Uuid::new_v4();
    let hash = hash_password(password)?;
    sqlx::query("INSERT INTO users (id, username, password_hash, role) VALUES ($1,$2,$3,$4)")
        .bind(id)
        .bind(username)
        .bind(hash)
        .bind(role)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(id)
}

pub async fn autenticar(pool: &PgPool, username: &str, password: &str) -> Result<Uuid, String> {
    let row: Option<(Uuid, String, bool)> =
        sqlx::query_as("SELECT id, password_hash, disabled FROM users WHERE username=$1")
            .bind(username)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
    let (id, hash, disabled) = row.ok_or_else(|| "credenciales inválidas".to_string())?;
    if disabled {
        return Err("usuario deshabilitado".to_string());
    }
    if !verify_password(password, &hash) {
        return Err("credenciales inválidas".to_string());
    }
    Ok(id)
}

pub async fn emitir_token(pool: &PgPool, user_id: Uuid, ttl_days: i64) -> Result<String, String> {
    let token = Uuid::new_v4().to_string() + &Uuid::new_v4().to_string().replace('-', "");
    let hash = token_hash(&token);
    let id = Uuid::new_v4();
    let expires = chrono::Utc::now() + chrono::Duration::days(ttl_days);
    sqlx::query(
        "INSERT INTO access_tokens (id, user_id, token_hash, expires_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(user_id)
    .bind(hash)
    .bind(expires)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(token)
}

pub async fn validar_token(pool: &PgPool, token: &str) -> Result<(Uuid, String), String> {
    let hash = token_hash(token);
    #[allow(clippy::type_complexity)]
    let row: Option<(
        Uuid,
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        bool,
    )> = sqlx::query_as(
        "SELECT t.user_id, u.role, t.expires_at, t.revoked_at, u.disabled \
             FROM access_tokens t JOIN users u ON u.id=t.user_id WHERE t.token_hash=$1",
    )
    .bind(hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let (uid, role, expires, revoked, disabled) =
        row.ok_or_else(|| "token inválido".to_string())?;
    if disabled {
        return Err("usuario deshabilitado".to_string());
    }
    if revoked.is_some() {
        return Err("token revocado".to_string());
    }
    if chrono::Utc::now() > expires {
        return Err("token expirado".to_string());
    }
    Ok((uid, role))
}

pub async fn revocar_token(pool: &PgPool, token: &str) -> Result<(), String> {
    let hash = token_hash(token);
    sqlx::query(
        "UPDATE access_tokens SET revoked_at=now() WHERE token_hash=$1 AND revoked_at IS NULL",
    )
    .bind(hash)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
