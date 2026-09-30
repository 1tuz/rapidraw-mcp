use anyhow::Result;
use rapidraw_mcp::{Config, RapidRawClient, RapidRawMcp};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // stdout is reserved for MCP JSON-RPC — log only to stderr.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let config = Config::from_env();
    tracing::info!(
        api_url = %config.api_url,
        token_set = config.api_token.is_some(),
        timeout_secs = config.timeout.as_secs(),
        "starting rapidraw-mcp"
    );

    let client = RapidRawClient::new(&config)?;
    let server = RapidRawMcp::new(client);

    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("MCP serve error: {e:?}");
    })?;

    service.waiting().await?;
    Ok(())
}
