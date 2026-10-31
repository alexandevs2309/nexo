//! Pruebas E2E del registro (requieren PostgreSQL aislado).
//!
//! Ejecutar de forma explícita y segura:
//!   DATABASE_URL="postgres://user:pass@127.0.0.1:5432/dbname" cargo test --test registry_integration -- --ignored --nocapture
//!
//! Condiciones de aislamiento (validadas por el test):
//! - host local: localhost, 127.0.0.1 o ::1
//! - base de datos no del sistema (postgres, template0, template1)
//!
//! El test crea un esquema temporal propio, ejecuta migraciones en él y lo elimina
//! al finalizar (incluso ante fallo o panic) mediante RAII.

use basilisco::paquetes;
use sqlx::postgres::PgConnectOptions;
use std::io::{Read, Write};
use std::process::{Child, Command};
use std::str::FromStr;
use std::time::Duration;

struct TestGuard {
    child: Option<Child>,
    schema: String,
    db_url: String,
    artifact_dir: String,
    base_dir: String,
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        let _ = Command::new("psql")
            .arg(&self.db_url)
            .args([
                "-v",
                "ON_ERROR_STOP=1",
                "-c",
                &format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", self.schema),
            ])
            .status();
        let _ = std::fs::remove_dir_all(&self.artifact_dir);
        let _ = std::fs::remove_dir_all(&self.base_dir);
    }
}

fn is_isolated(url: &str) -> Result<(), String> {
    let opts =
        PgConnectOptions::from_str(url).map_err(|e| format!("DATABASE_URL inválida: {e}"))?;
    let host = opts.get_host();
    if host != "localhost" && host != "127.0.0.1" && host != "::1" {
        return Err(format!("host no local: {host}"));
    }
    let db = opts.get_database().unwrap_or("");
    if db.is_empty() || db == "postgres" || db == "template0" || db == "template1" {
        return Err(format!("base de datos no permitida: {db}"));
    }
    if opts.get_username().is_empty() {
        return Err("usuario vacío".to_string());
    }
    Ok(())
}

fn psql_available() -> bool {
    Command::new("psql")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn make_bpk() -> (Vec<u8>, String) {
    let content = b"lib demo\n";
    let manifest = r#"[format]
name = "BASILISCO_PACKAGE"
version = "1.0"

[package]
namespace = "demo"
name = "demo"
version = "0.1.0"
description = "demo"
license = "Apache-2.0"
authors = ["demo"]

[compatibility]
basilisco = ">=0.6.0"

[dependencies]

[[files]]
path = "lib.bsk"
size = 9
sha256 = "de0fcd91c7f11f05a86c414de64cb7f44e421b4c165475f46157ebeeb3e9efe9"
"#;
    let mut buf = Vec::new();
    {
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default();
        zip.start_file("BPK-MANIFEST.toml", opts).unwrap();
        zip.write_all(manifest.as_bytes()).unwrap();
        zip.start_file("payload/lib.bsk", opts).unwrap();
        zip.write_all(content).unwrap();
        zip.finish().unwrap();
    }
    let sha = paquetes::sha256_hex(&buf);
    (buf, sha)
}

#[test]
#[ignore = "requiere PostgreSQL aislado; ver cabecera del archivo"]
fn ciclo_publicacion_descarga_instalacion() {
    let db_url = match std::env::var("DATABASE_URL") {
        Ok(v) => v,
        Err(_) => panic!("DATABASE_URL no definida; no se puede ejecutar el E2E"),
    };
    is_isolated(&db_url).expect("DATABASE_URL no cumple condiciones de aislamiento");
    if !psql_available() {
        panic!("psql no disponible; instale postgresql-client");
    }

    let schema = format!("test_r6_{}", std::process::id());
    let status = Command::new("psql")
        .arg(&db_url)
        .args([
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", schema),
        ])
        .status()
        .expect("no se pudo ejecutar psql");
    assert!(status.success(), "no se pudo crear el esquema de prueba");

    let mut server_url = db_url.clone();
    let sep = if server_url.contains('?') { "&" } else { "?" };
    server_url.push_str(&format!("{}options=-c%20search_path%3D{}", sep, schema));

    let artifact_dir = format!("/tmp/bsk-e2e-art-{}", std::process::id());
    let base_dir = format!("/tmp/bsk-e2e-base-{}", std::process::id());
    let _ = std::fs::remove_dir_all(&artifact_dir);
    let _ = std::fs::remove_dir_all(&base_dir);
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::create_dir_all(&base_dir).unwrap();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let mut guard = TestGuard {
        child: None,
        schema: schema.clone(),
        db_url: db_url.clone(),
        artifact_dir: artifact_dir.clone(),
        base_dir: base_dir.clone(),
    };

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_basilisco-registry"));
    cmd.env("DATABASE_URL", &server_url);
    cmd.env("BASILISCO_REGISTRY_ID", "test-reg");
    cmd.env("BASILISCO_ARTIFACTS", &artifact_dir);
    cmd.env("BASILISCO_BIND", format!("127.0.0.1:{}", port));
    cmd.env("BASILISCO_BOOTSTRAP_ADMIN_USERNAME", "admin");
    cmd.env("BASILISCO_BOOTSTRAP_ADMIN_PASSWORD", "adminpass123");
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    guard.child = Some(cmd.spawn().expect("no se pudo arrancar el servidor"));

    let mut ready = false;
    for _ in 0..50 {
        if ureq::get(&format!("http://127.0.0.1:{}/health", port))
            .call()
            .is_ok()
        {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(ready, "servidor no arrancó");

    let base_url = format!("http://127.0.0.1:{}", port);

    // login
    let login = serde_json::json!({"username":"admin","password":"adminpass123"});
    let resp = ureq::post(&format!("{}/api/v1/auth/token", base_url))
        .header("Content-Type", "application/json")
        .send(login.to_string())
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&resp.into_body().read_to_string().unwrap()).unwrap();
    let token = v["data"]["token"].as_str().unwrap().to_string();

    // namespace
    let _ = ureq::post(&format!("{}/api/v1/namespaces", base_url))
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send(serde_json::json!({"name":"demo"}).to_string());

    let (buf, artifact_sha) = make_bpk();
    let b64 = base64_encode(&buf);

    // bootstrap de confianza fuera de banda
    let key_bytes = std::fs::read(format!("{}/registry.key", artifact_dir)).unwrap();
    let sk = ed25519_dalek::SigningKey::from_bytes(key_bytes.as_slice().try_into().unwrap());
    let vk = sk.verifying_key();
    let pk_hex: String = vk.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
    let key_id = "key-1".to_string();

    let base = std::path::Path::new(&base_dir);
    std::fs::create_dir_all(base).unwrap();
    let pk_bytes: Vec<u8> = (0..pk_hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&pk_hex[i..i + 2], 16).unwrap())
        .collect();
    let mut almacen = paquetes::AlmacenClaves::default();
    almacen
        .agregar_seguro("test-reg", &key_id, pk_bytes)
        .unwrap();
    almacen.guardar(base).unwrap();

    let pub_meta = serde_json::json!({
        "namespace":"demo","name":"demo","version":"0.1.0",
        "sha256":artifact_sha,
        "registry_id":"test-reg","key_id":key_id,
        "nonce":"nonce-e2e-full","expires_at":"2027-10-31T00:00:00Z",
        "published_at":"2026-10-31T00:00:00Z",
        "firma":"","clave":""
    });
    let pub_body = serde_json::json!({"meta":pub_meta,"artifact_b64":b64});
    let r = ureq::post(&format!("{}/api/v1/publish", base_url))
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send(pub_body.to_string())
        .unwrap();
    assert_eq!(r.status(), 201);
    println!("E2E: publicación OK");

    // download
    let dl = ureq::get(&format!(
        "{}/api/v1/packages/demo/demo/0.1.0/download",
        base_url
    ))
    .call()
    .unwrap();
    let mut buf2 = Vec::new();
    dl.into_body().as_reader().read_to_end(&mut buf2).unwrap();
    assert_eq!(paquetes::sha256_hex(&buf2), artifact_sha);
    println!("E2E: descarga OK");

    // install
    let meta_url = format!("{}/api/v1/packages/demo/demo/0.1.0", base_url);
    let art_url = format!("{}/api/v1/packages/demo/demo/0.1.0/download", base_url);
    let res = paquetes::instalar_paquete_remoto_completo(base, &meta_url, &art_url).unwrap();
    assert_eq!(res.nombre, "demo");
    assert!(res.verificado);
    println!("E2E: instalación verificada OK");

    // restart
    let _ = guard.child.as_mut().unwrap().kill();
    let _ = guard.child.as_mut().unwrap().wait();
    guard.child = None;
    std::thread::sleep(Duration::from_secs(1));
    let mut cmd2 = Command::new(env!("CARGO_BIN_EXE_basilisco-registry"));
    cmd2.env("DATABASE_URL", &server_url);
    cmd2.env("BASILISCO_REGISTRY_ID", "test-reg");
    cmd2.env("BASILISCO_ARTIFACTS", &artifact_dir);
    cmd2.env("BASILISCO_BIND", format!("127.0.0.1:{}", port));
    cmd2.env("BASILISCO_BOOTSTRAP_ADMIN_USERNAME", "admin");
    cmd2.env("BASILISCO_BOOTSTRAP_ADMIN_PASSWORD", "adminpass123");
    cmd2.stdout(std::process::Stdio::null());
    cmd2.stderr(std::process::Stdio::null());
    guard.child = Some(cmd2.spawn().unwrap());
    let mut ready = false;
    for _ in 0..50 {
        if ureq::get(&format!("http://127.0.0.1:{}/health", port))
            .call()
            .is_ok()
        {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(ready, "servidor no arrancó tras reinicio");
    let dl2 = ureq::get(&format!(
        "{}/api/v1/packages/demo/demo/0.1.0/download",
        base_url
    ))
    .call()
    .unwrap();
    let mut buf3 = Vec::new();
    dl2.into_body().as_reader().read_to_end(&mut buf3).unwrap();
    assert_eq!(paquetes::sha256_hex(&buf3), artifact_sha);
    println!("E2E: persistencia tras reinicio OK");
    // guard Drop handles cleanup
}

#[test]
#[ignore = "requiere PostgreSQL aislado; ver cabecera del archivo"]
fn concurrencia_misma_version() {
    let db_url = match std::env::var("DATABASE_URL") {
        Ok(v) => v,
        Err(_) => panic!("DATABASE_URL no definida; no se puede ejecutar el E2E"),
    };
    is_isolated(&db_url).expect("DATABASE_URL no cumple condiciones de aislamiento");
    if !psql_available() {
        panic!("psql no disponible; instale postgresql-client");
    }

    let schema = format!("test_r6_conc_{}", std::process::id());
    let status = Command::new("psql")
        .arg(&db_url)
        .args([
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", schema),
        ])
        .status()
        .expect("no se pudo ejecutar psql");
    assert!(status.success(), "no se pudo crear el esquema de prueba");

    let mut server_url = db_url.clone();
    let sep = if server_url.contains('?') { "&" } else { "?" };
    server_url.push_str(&format!("{}options=-c%20search_path%3D{}", sep, schema));

    let artifact_dir = format!("/tmp/bsk-e2e-conc-{}", std::process::id());
    let _ = std::fs::remove_dir_all(&artifact_dir);
    std::fs::create_dir_all(&artifact_dir).unwrap();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_basilisco-registry"));
    cmd.env("DATABASE_URL", &server_url);
    cmd.env("BASILISCO_REGISTRY_ID", "test-reg");
    cmd.env("BASILISCO_ARTIFACTS", &artifact_dir);
    cmd.env("BASILISCO_BIND", format!("127.0.0.1:{}", port));
    cmd.env("BASILISCO_BOOTSTRAP_ADMIN_USERNAME", "admin");
    cmd.env("BASILISCO_BOOTSTRAP_ADMIN_PASSWORD", "adminpass123");
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    let mut child = cmd.spawn().unwrap();

    let mut ready = false;
    for _ in 0..50 {
        if ureq::get(&format!("http://127.0.0.1:{}/health", port))
            .call()
            .is_ok()
        {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(ready);

    let base_url = format!("http://127.0.0.1:{}", port);
    let login = serde_json::json!({"username":"admin","password":"adminpass123"});
    let resp = ureq::post(&format!("{}/api/v1/auth/token", base_url))
        .header("Content-Type", "application/json")
        .send(login.to_string())
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&resp.into_body().read_to_string().unwrap()).unwrap();
    let token = v["data"]["token"].as_str().unwrap().to_string();

    let _ = ureq::post(&format!("{}/api/v1/namespaces", base_url))
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .send(serde_json::json!({"name":"demo"}).to_string());

    let content = b"lib demo\n";
    let manifest = r#"[format]
name = "BASILISCO_PACKAGE"
version = "1.0"

[package]
namespace = "demo"
name = "demo"
version = "0.1.0"
description = "demo"
license = "Apache-2.0"
authors = ["demo"]

[compatibility]
basilisco = ">=0.6.0"

[dependencies]

[[files]]
path = "lib.bsk"
size = 9
sha256 = "de0fcd91c7f11f05a86c414de64cb7f44e421b4c165475f46157ebeeb3e9efe9"
"#;
    let mut buf = Vec::new();
    {
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default();
        zip.start_file("BPK-MANIFEST.toml", opts).unwrap();
        zip.write_all(manifest.as_bytes()).unwrap();
        zip.start_file("payload/lib.bsk", opts).unwrap();
        zip.write_all(content).unwrap();
        zip.finish().unwrap();
    }
    let artifact_sha = paquetes::sha256_hex(&buf);
    let b64 = base64_encode(&buf);

    let key_bytes = std::fs::read(format!("{}/registry.key", artifact_dir)).unwrap();
    let sk = ed25519_dalek::SigningKey::from_bytes(key_bytes.as_slice().try_into().unwrap());
    let vk = sk.verifying_key();
    let _pk_hex: String = vk.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
    let key_id = "key-1".to_string();

    let pub_meta = serde_json::json!({
        "namespace":"demo","name":"demo","version":"0.1.0",
        "sha256":artifact_sha,
        "registry_id":"test-reg","key_id":key_id,
        "nonce":"nonce-conc","expires_at":"2027-10-31T00:00:00Z",
        "published_at":"2026-10-31T00:00:00Z",
        "firma":"","clave":""
    });
    let pub_body = serde_json::json!({"meta":pub_meta,"artifact_b64":b64});

    let url = format!("{}/api/v1/publish", base_url);
    let token1 = token.clone();
    let body1 = pub_body.to_string();
    let h1 = std::thread::spawn(move || {
        ureq::post(&url)
            .header("Authorization", &format!("Bearer {}", token1))
            .header("Content-Type", "application/json")
            .send(body1)
            .map(|r| r.status())
    });
    let mut pub_body2 = pub_body.clone();
    pub_body2["meta"]["nonce"] = serde_json::json!("nonce-conc-2");
    let url2 = format!("{}/api/v1/publish", base_url);
    let token2 = token.clone();
    let body2 = pub_body2.to_string();
    let h2 = std::thread::spawn(move || {
        ureq::post(&url2)
            .header("Authorization", &format!("Bearer {}", token2))
            .header("Content-Type", "application/json")
            .send(body2)
            .map(|r| r.status())
    });
    let r1 = h1.join().unwrap();
    let r2 = h2.join().unwrap();
    let successes = [
        r1.as_ref().map(|s| *s == 201).unwrap_or(false),
        r2.as_ref().map(|s| *s == 201).unwrap_or(false),
    ]
    .iter()
    .filter(|b| **b)
    .count();
    assert_eq!(
        successes, 1,
        "se esperaba exactamente una publicación exitosa: {:?} {:?}",
        r1, r2
    );

    let mut found = false;
    for entry in std::fs::read_dir(&artifact_dir).unwrap() {
        let e = entry.unwrap();
        if e.path().extension().map(|x| x == "bpk").unwrap_or(false) {
            found = true;
        }
    }
    assert!(found, "no se encontró artefacto publicado");

    let _ = child.kill();
    let _ = child.wait();

    // cleanup schema
    let _ = std::process::Command::new("psql")
        .arg(&db_url)
        .args([
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!(
                "DROP SCHEMA IF EXISTS \"test_r6_conc_{}\" CASCADE;",
                std::process::id()
            ),
        ])
        .status();
    let _ = std::fs::remove_dir_all(&artifact_dir);
}
