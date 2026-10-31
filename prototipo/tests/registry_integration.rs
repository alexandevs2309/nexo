//! Pruebas de integración del registro (requieren PostgreSQL).
//! Ejecutar con: DATABASE_URL=... cargo test --test registry_integration -- --ignored

use basilisco::servidor::main as srv;
use std::net::TcpListener;

#[tokio::test]
#[ignore = "requiere PostgreSQL (DATABASE_URL)"]
async fn health_endpoint() {
    // Verifica que el servidor arranca y responde /health.
    let Ok(db) = std::env::var("DATABASE_URL") else {
        return;
    };
    unsafe { std::env::set_var("BASILISCO_REGISTRY_ID", "test-reg") };
    unsafe { std::env::set_var("DATABASE_URL", db) };
    // bind a free port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    unsafe { std::env::set_var("BASILISCO_BIND", addr.to_string()) };
    unsafe { std::env::set_var("BASILISCO_ARTIFACTS", "/tmp/opencode/reg-artifacts") };
    let _ = std::fs::create_dir_all("/tmp/opencode/reg-artifacts");

    // run server in background
    tokio::spawn(async move {
        let _ = srv::run().await;
    });
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let resp = ureq::get(&format!("http://{}/health", addr)).call();
    assert!(resp.is_ok());
}
