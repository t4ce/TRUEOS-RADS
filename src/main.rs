use anyhow::Context;
use trueos_rads::server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "trueos_rads=info,tower_http=info".into()),
        )
        .init();

    let workspace = std::env::current_dir().context("failed to read current directory")?;
    server::serve(workspace).await
}
