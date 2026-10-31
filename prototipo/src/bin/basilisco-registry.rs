//! Entry point independiente del servidor de registro.

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    if let Err(error) = basilisco::servidor::main::run().await {
        tracing::error!(error = %error, "el servidor de registro terminó con error");
        std::process::exit(1);
    }
}
