mod macros;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use dotenvy::dotenv;
use serde::Deserialize;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::process::exit;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::fs;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{Level, debug, error, info, instrument, warn};
use tracing_subscriber::FmtSubscriber;
#[cfg(feature = "post_netcup")]
use {
    anyhow::{anyhow, bail},
    reqwest::Client,
    serde_json::json,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const FORWARDED_HEADER: &str = "x-forwarded-for";
const IP_CONF_PATH: LazyLock<String> =
    LazyLock::new(|| env::var("IP_CONF_PATH").unwrap_or_else(|_| String::from("/config/ip.conf")));

static AUTH: LazyLock<String> = LazyLock::new(|| get_env!("AUTH"));

#[cfg(feature = "post_netcup")]
const NC_URL: &str = "https://ccp.netcup.net/run/webservice/servers/endpoint.php?JSON";
#[cfg(feature = "post_netcup")]
static NC_API_KEY: LazyLock<String> = LazyLock::new(|| get_env!("NC_API_KEY"));
#[cfg(feature = "post_netcup")]
static NC_API_PW: LazyLock<String> = LazyLock::new(|| get_env!("NC_API_PW"));
#[cfg(feature = "post_netcup")]
static NC_CUS_ID: LazyLock<String> = LazyLock::new(|| get_env!("NC_CUS_ID"));
#[cfg(feature = "post_netcup")]
static NC_DOMAIN_NAME: LazyLock<String> = LazyLock::new(|| get_env!("NC_DOMAIN_NAME"));
#[cfg(feature = "post_netcup")]
static NC_STAR_ID: LazyLock<String> = LazyLock::new(|| get_env!("NC_STAR_ID"));
#[cfg(feature = "post_netcup")]
static NC_AT_ID: LazyLock<String> = LazyLock::new(|| get_env!("NC_AT_ID"));

#[derive(Clone)]
struct AppState {
    rw_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Deserialize)]
struct IpPayload {
    auth: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv();

    {
        const KEY: &str = "LOG_LEVEL";
        #[cfg(debug_assertions)]
        pub const DEFAULT_LOG_LEVEL: Level = Level::DEBUG;
        #[cfg(not(debug_assertions))]
        pub const DEFAULT_LOG_LEVEL: Level = Level::INFO;

        let lvl = match env::var(KEY) {
            Ok(lvl) => match lvl.parse() {
                Ok(lvl) => lvl,
                Err(e) => {
                    eprintln!("WARNING: {KEY} is set, but the value {lvl} is invalid: {e}");
                    exit(1);
                }
            },
            Err(_) => DEFAULT_LOG_LEVEL,
        };

        let fmt_sub = FmtSubscriber::builder().with_max_level(lvl).finish();

        tracing::subscriber::set_global_default(fmt_sub)
            .with_context(|| format!("Failed to set subscriber with lvl {lvl}"))?;
    }

    #[cfg(any(feature = "post_netcup"))]
    info!("Starting DDNS-Server v{VERSION} with \"post_netcup\"");
    #[cfg(not(any(feature = "post_netcup")))]
    info!("Starting DDNS-Server v{VERSION}");

    info!("Checking environment");

    let _ = *AUTH;
    let port = get_env!("PORT", "8080", u16);

    create_ip_file().await.context("Unable to create ip file")?;

    let listen_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);

    let state = AppState {
        rw_lock: Arc::new(Mutex::new(())),
    };

    let app = Router::new().route("/ip", post(post_ip)).with_state(state);

    let listener = TcpListener::bind(listen_address)
        .await
        .with_context(|| format!("Failed to bind port {port}"))?;
    info!("Listening on {listen_address}");

    axum::serve(listener, app)
        .await
        .context("Unable to serve")?;

    Ok(())
}

async fn create_ip_file() -> Result<()> {
    let ip_path = IP_CONF_PATH.to_string();
    let ip_path = Path::new(&ip_path);
    if let Some(path) = ip_path.parent() {
        if !path.to_string_lossy().trim().is_empty() {
            debug!("IP path {path:?} does not exist, creating");
            fs::create_dir_all(&path)
                .await
                .with_context(|| format!("Error creating IP path {path:?}"))?;
        } else {
            debug!("Parent empty");
        }
    }
    if !fs::try_exists(&ip_path)
        .await
        .with_context(|| format!("Unable to check if IP path {ip_path:?} exists"))?
    {
        fs::File::create(&ip_path)
            .await
            .with_context(|| format!("Error creating IP file {ip_path:?}"))?;
    }
    Ok(())
}

#[instrument(skip(state), level = "trace")]
async fn post_ip(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<IpPayload>,
) -> impl IntoResponse {
    if payload.auth != *AUTH {
        warn!("Invalid password detected: {:?}", payload.auth);
        tokio::time::sleep(Duration::from_secs(5)).await;
        return (
            StatusCode::FORBIDDEN,
            String::from("No entrance with that password"),
        );
    }

    let client_ip = match headers.get(FORWARDED_HEADER) {
        Some(ip) => match ip.to_str() {
            Ok(ip_str) => ip_str,
            Err(e) => {
                let err = format!("{FORWARDED_HEADER:?} is set, but can't be parsed to str");
                error!(?e, "{err}");
                return (StatusCode::PRECONDITION_FAILED, err);
            }
        },
        None => {
            let err = format!("{FORWARDED_HEADER} is not set");
            warn!("{err}");
            return (StatusCode::PRECONDITION_FAILED, err);
        }
    };
    debug!(?client_ip);

    let _guard = state.rw_lock.lock().await;

    let stored_ip = {
        const IP_ERR_MSG: &str = "Can't read existing IP";
        let mut attempt = 0u8;
        loop {
            match fs::read_to_string(&*IP_CONF_PATH).await {
                Ok(ip) => break ip,
                Err(e) => {
                    warn!("Can't read {:?}: {e:?}", IP_CONF_PATH.as_str());

                    match fs::try_exists(&*IP_CONF_PATH).await {
                        Ok(true) => error!("File exists, but we cannot read"),
                        Ok(false) if attempt == 0 => {
                            attempt += 1;
                            warn!(
                                "{:?} does not exist, creating again...",
                                IP_CONF_PATH.as_str()
                            );
                            match create_ip_file().await {
                                Ok(_) => {
                                    info!("Recreated {:?}, retrying...", IP_CONF_PATH.as_str());
                                    continue;
                                }
                                Err(e) => {
                                    error!("Failed to recreate {:?}: {e:?}", IP_CONF_PATH.as_str())
                                }
                            }
                        }
                        Ok(false) => {
                            error!(
                                "Second try and {:?} still doesn't exist",
                                IP_CONF_PATH.as_str()
                            )
                        }
                        Err(e) => error!(
                            "Somehow, we cannot even check if {:?} exists: {e:?}",
                            IP_CONF_PATH.as_str()
                        ),
                    }

                    return (StatusCode::INTERNAL_SERVER_ERROR, String::from(IP_ERR_MSG));
                }
            };
        }
    };

    if client_ip == stored_ip {
        debug!("Client IP {client_ip:?} == stored ip {stored_ip:?}");
        return (StatusCode::OK, format!("No change in IP: {stored_ip:?}"));
    }

    match fs::write(&*IP_CONF_PATH, &client_ip).await {
        Ok(_) => debug!("Wrote new ip {client_ip:?} to {:?}", IP_CONF_PATH.as_str()),
        Err(e) => {
            error!(
                ?e,
                "Unable to write IP {client_ip:?} to file {:?}",
                IP_CONF_PATH.as_str()
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Can't write new IP {client_ip:?} to file"),
            );
        }
    };

    #[cfg(not(feature = "post_netcup"))]
    {
        debug!("Post IP deactivated");
        return (StatusCode::OK, format!("New IP {client_ip:?} written"));
    }

    #[cfg(any(feature = "post_netcup"))]
    match execute_ip_change(&client_ip).await {
        Ok(_) => {
            info!("Successfully posted new IP");
            (
                StatusCode::OK,
                format!("New IP {client_ip:?} written and posted"),
            )
        }
        Err(e) => {
            error!("Post IP failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Posting IP failed (write succeeded)"),
            )
        }
    }
}

#[cfg(feature = "post_netcup")]
#[instrument]
async fn execute_ip_change(ip: &str) -> Result<()> {
    let client = Client::new();
    let login_payload = json!({
        "action": "login",
        "param": {
            "apikey": *NC_API_KEY,
            "apipassword": *NC_API_PW,
            "customernumber": *NC_CUS_ID,
        },
    });

    let session_id = client
        .post(NC_URL)
        .json(&login_payload)
        .send()
        .await
        .context("Unable to login")?
        .json::<serde_json::Value>()
        .await
        .context("Unable to parse login response")?["responsedata"]["apisessionid"]
        .as_str()
        .ok_or(anyhow!("No session id available"))?
        .to_string();

    let dns_payload = json!({
        "action": "updateDnsRecords",
        "param": {
            "customernumber": *NC_CUS_ID,
            "apikey": *NC_API_KEY,
            "apisessionid": session_id,
            "clientrequestid": "",
            "domainname": *NC_DOMAIN_NAME,
            "dnsrecordset": {
                "dnsrecords": [
                    {
                        "id": *NC_STAR_ID,
                        "hostname": "*",
                        "type": "A",
                        "priority": "0",
                        "destination": ip,
                        "deleterecord": "FALSE",
                        "state": "yes",
                    },
                    {
                        "id": *NC_AT_ID,
                        "hostname": "@",
                        "type": "A",
                        "priority": "0",
                        "destination": ip,
                        "deleterecord": "FALSE",
                        "state": "yes",
                    },
                ],
            },
        },
    });

    let dns_response = client
        .post(NC_URL)
        .json(&dns_payload)
        .send()
        .await
        .context("Unable to update dns")?
        .json::<serde_json::Value>()
        .await
        .context("Unable to parse dns response")?["shortmessage"]
        .as_str()
        .ok_or(anyhow!("No shortmessage in dns response"))?
        .to_string();

    if dns_response.trim() != "DNS records successful updated" {
        bail!("Invalid response received: {dns_response}");
    }

    Ok(())
}
