use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let path_arg = std::env::args()
        .nth(1)
        .context("no heapdump path provided")?;
    let path = PathBuf::from(path_arg);
    let heap = heapdump_analyzer::parse(&path)?;
    dbg!(heap);
    Ok(())
}
