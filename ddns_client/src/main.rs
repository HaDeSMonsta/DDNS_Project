mod macros;

use std::{sync::LazyLock, time::Duration};

use anyhow::{Context, Result, bail};
use dotenvy::dotenv;
use reqwest::Client;
use serde_json::json;
use tracing::{Level, debug, info, instrument, warn};
use tracing_subscriber::FmtSubscriber;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_SLEEP_MINS: &str = "15";
static SLEEP_SECS: LazyLock<u64> =
    LazyLock::new(|| get_env!("SLEEP_MINS", DEFAULT_SLEEP_MINS, u64) * 60);
static SLEEP_MINS: LazyLock<u64> =
    LazyLock::new(|| get_env!("SLEEP_MINS", DEFAULT_SLEEP_MINS, u64));

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv();
    {
        #[cfg(debug_assertions)]
        let level = Level::DEBUG;
        #[cfg(not(debug_assertions))]
        let level = Level::INFO;
        tracing::subscriber::set_global_default(
            FmtSubscriber::builder().with_max_level(level).finish(),
        )
        .expect("Setting global default subscriber failed");
    }

    info!("Starting DDNS-Client v{VERSION}");

    let server_address = get_env!("SERVER_ADDRESS");
    if *SLEEP_SECS == 0 {
        bail!("Sleep mins must be > 0");
    }
    info!("Delta between calls: {} mins", *SLEEP_MINS);

    let auth = get_env!("AUTH");

    info!("Target address: {server_address:?}");

    loop {
        mk_call(&server_address, &auth)
            .await
            .context("Failed to call Server")?;
        debug!("Sleeping for {} mins", *SLEEP_MINS);
        tokio::time::sleep(Duration::from_secs(*SLEEP_SECS)).await;
    }
}

#[instrument(skip(auth), level = "debug")]
async fn mk_call(target_url: &str, auth: &str) -> Result<()> {
    let client = Client::new();
    debug!("Created client");
    let payload = json!({"auth": &auth,});
    debug!("Created payload");

    let resp = client
        .post(target_url)
        .header("x-forwarded-for", "127.0.0.1")
        .json(&payload)
        .send()
        .await
        .context("Failed to send request")?;

    let resp_status = resp.status();
    let resp_text = resp
        .text()
        .await
        .context("Unablte to extract text from resp")?;

    if resp_status == 200 {
        info!("Call succeeded: {resp_text}");
    } else {
        warn!("Status != 200: {resp_status}; Msg: {resp_text}");
    }

    Ok(())
}
