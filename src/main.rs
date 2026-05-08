use anyhow::{Context, bail};
use std::fs;
use std::path::Path;
use trueos_rads::server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let workspace = std::env::current_dir().context("failed to read current directory")?;
    load_local_env(&workspace)?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "trueos_rads=info,tower_http=info".into()),
        )
        .init();

    server::serve(workspace).await
}

fn load_local_env(workspace: &Path) -> anyhow::Result<()> {
    let path = workspace.join(".env.local");
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err).with_context(|| format!("failed to read {}", path.display())),
    };

    for (index, raw_line) in raw.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
        let Some((key, value)) = line.split_once('=') else {
            bail!("invalid .env.local line {}", index + 1);
        };
        let key = key.trim();
        if !is_env_key(key) {
            bail!("invalid .env.local key on line {}", index + 1);
        }
        if std::env::var_os(key).is_none() {
            let value = parse_env_value(value.trim());
            // Startup is still single-threaded here; shell-provided env wins above.
            unsafe {
                std::env::set_var(key, value);
            }
        }
    }

    Ok(())
}

fn parse_env_value(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return value[1..value.len() - 1].to_string();
        }
    }

    value.to_string()
}

fn is_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(ch) if ch == '_' || ch.is_ascii_alphabetic() => {}
        _ => return false,
    }

    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}
