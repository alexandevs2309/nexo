//! Pruebas E2E del registro (requieren PostgreSQL y el binario basilisco-registry).
//! Ejecutar con: DATABASE_URL=... cargo test --test registry_integration -- --ignored

use basilisco::paquetes;
use std::io::{Read, Write};
use std::process::{Child, Command};
use std::time::Duration;

fn start_server(port: u16, artifact_dir: &str) -> Child {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_basilisco-registry"));
    cmd.env("BASILISCO_BIND", format!("127.0.0.1:{}", port));
    cmd.env("BASILISCO_ARTIFACTS", artifact_dir);
    cmd.env("BASILISCO_BOOTSTRAP_ADMIN_USERNAME", "admin");
    cmd.env("BASILISCO_BOOTSTRAP_ADMIN_PASSWORD", "adminpass123");
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    cmd.spawn().expect("no se pudo arrancar el servidor")
}

#[test]
#[ignore = "requiere PostgreSQL (DATABASE_URL)"]
fn ciclo_publicacion_descarga_instalacion() {
    if std::env::var("DATABASE_URL").is_err() {
        return;
    }
    unsafe {
        std::env::set_var("BASILISCO_REGISTRY_ID", "test-reg");
    }

    let artifact_dir = format!("/tmp/opencode/reg-artifacts-e2e-{}", std::process::id());
    let _ = std::fs::remove_dir_all(&artifact_dir);
    std::fs::create_dir_all(&artifact_dir).unwrap();

    // reset schema
    let psql = std::process::Command::new("psql")
        .args([
            "-h",
            "127.0.0.1",
            "-U",
            "basilisco",
            "-d",
            "basilisco",
            "-c",
            "DROP SCHEMA public CASCADE; CREATE SCHEMA public;",
        ])
        .env("PGPASSWORD", "basilisco_dev_password")
        .output();
    let _ = psql;

    let port = (20000 + (std::process::id() as u16 % 10000)) as u16;
    let mut child = start_server(port, &artifact_dir);
    // wait for health
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

    // BPK
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

    // Bootstrap de confianza fuera de banda: leer la clave pública del archivo local.
    let key_bytes = std::fs::read(format!("{}/registry.key", artifact_dir)).unwrap();
    let sk = ed25519_dalek::SigningKey::from_bytes(key_bytes.as_slice().try_into().unwrap());
    let vk = sk.verifying_key();
    let pk_hex: String = vk.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
    let key_id = "key-1".to_string();

    // trust store
    let base_dir = format!("/tmp/opencode/e2e-client-{}", std::process::id());
    let base = std::path::Path::new(&base_dir);
    let _ = std::fs::remove_dir_all(base);
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

    // install
    let meta_url = format!("{}/api/v1/packages/demo/demo/0.1.0", base_url);
    let art_url = format!("{}/api/v1/packages/demo/demo/0.1.0/download", base_url);
    let res = paquetes::instalar_paquete_remoto_completo(base, &meta_url, &art_url).unwrap();
    assert_eq!(res.nombre, "demo");
    assert!(res.verificado);

    // Restart: matar y volver a arrancar, verificar descarga persistente.
    let _ = child.kill();
    let _ = child.wait();
    std::thread::sleep(Duration::from_secs(1));
    let mut child2 = start_server(port, &artifact_dir);
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

    let _ = child2.kill();
    let _ = child2.wait();
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}
