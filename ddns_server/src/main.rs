mod macros;

#[cfg(feature = "post_netcup")]
use anyhow::anyhow;
use anyhow::{bail, Context, Result};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use dotenvy::dotenv;
#[cfg(feature = "post_netcup")]
use reqwest::Client;
use serde::Deserialize;
#[cfg(feature = "post_netcup")]
use serde_json::json;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::exit;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::fs;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

const FORWARDED_HEADER: &str = "x-forwarded-for";
const IP_CONF_PATH: &str = "/config/ip.conf";

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

#[derive(Deserialize)]
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

        let fmt_sub = FmtSubscriber::builder()
            .with_max_level(lvl)
            .finish();

        tracing::subscriber::set_global_default(fmt_sub)
            .with_context(|| format!("Failed to set subscriber with lvl {lvl}"))?;
    }

    info!("Checking environment");

    let _ = *AUTH;

    #[cfg(any(feature = "post_netcup"))]
    info!("Post IP activated");
    #[cfg(not(any(feature = "post_netcup")))]
    info!("Post IP not activated");

    match fs::try_exists(IP_CONF_PATH).await {
        Ok(true) => debug!("IP Conf exists"),
        Ok(false) => {
            info!("{IP_CONF_PATH} does not exist, will create");
            if let Err(e) = fs::File::create(IP_CONF_PATH).await {
                error!("Unable to create {IP_CONF_PATH}: {e}");
                exit(1);
            };
        }
        Err(e) => {
            error!("Can't check if {IP_CONF_PATH} exists: {e}");
            exit(1);
        }
    }

    let port = get_env!("PORT", u16);
    let listen_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);

    let state = AppState {
        rw_lock: Arc::new(Mutex::new(())),
    };

    let app = Router::new()
        .route("/ip", post(post_ip))
        .with_state(state);

    let listener = TcpListener::bind(listen_address)
        .await
        .with_context(|| format!("Failed to bind port {port}"))?;
    info!("Listening on {listen_address}");

    axum::serve(listener, app)
        .await
        .context("Unable to serve")?;

    Ok(())
}

// Why tf does the order matter? If ConnectInfo/headers are below json, this won't compile
async fn post_ip(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<IpPayload>,
) -> impl IntoResponse {
    if payload.auth != *AUTH {
        warn!("Invalid password detected: {}", payload.auth);
        tokio::time::sleep(Duration::from_secs(5)).await;
        return (StatusCode::FORBIDDEN, String::from("No entrance with that password"));
    }

    let client_ip = match headers.get(FORWARDED_HEADER) {
        Some(ip) => ip,
        None => {
            let err = format!("{FORWARDED_HEADER} is not set");
            error!("{err}");
            return (StatusCode::INTERNAL_SERVER_ERROR, err);
        }
    };
    debug!(?client_ip);

    let _guard = state.rw_lock.lock().await;

    let ip = match fs::read_to_string(IP_CONF_PATH).await {
        Ok(ip) => ip,
        Err(e) => {
            error!("Can't read {IP_CONF_PATH}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, String::from("Can't read existing IP"));
        }
    };
    let ip = ip.trim();

    if client_ip != ip {
        debug!("Client IP ({client_ip:?}) == ip ({ip})");
        return (StatusCode::OK, format!("No change in IP: {ip}"));
    }

    match fs::write(IP_CONF_PATH, &ip).await {
        Ok(_) => debug!("Wrote new ip {ip} to {IP_CONF_PATH}"),
        Err(e) => {
            error!("Unable to write IP {ip} to file {IP_CONF_PATH}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Can't write new IP {ip} to file: {e}"),
            );
        }
    };

    #[cfg(not(feature = "post_netcup"))]
    {
        debug!("Post IP deactivated");
        return (StatusCode::OK, format!("New IP ({ip}) written (no post_ip detected)"));
    }

    #[cfg(any(feature = "post_netcup"))]
    match execute_ip_change(&ip).await {
        Ok(_) => {
            info!("Successfully posted new IP");
            (StatusCode::OK, format!("New IP ({ip}) posted"))
        }
        Err(e) => {
            error!("Post IP failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, String::from("Posting IP failed"))
        }
    }
}

#[cfg(feature = "post_netcup")]
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

    let session_id = client.post(NC_URL)
                           .json(&login_payload)
                           .send()
                           .await
                           .context("Unable to login")?
        .json::<serde_json::Value>()
        .await
        .context("Unable to parse login response")?
        ["responsedata"]
        ["apisessionid"]
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

    let dns_response = client.post(NC_URL)
                             .json(&dns_payload)
                             .send()
                             .await
                             .context("Unable to update dns")?
        .json::<serde_json::Value>()
        .await
        .context("Unable to parse dns response")?
        ["shortmessage"]
        .as_str()
        .ok_or(anyhow!("No shortmessage in dns response"))?
        .to_string();

    if dns_response.trim() != "DNS records successful updated" {
        bail!("Invalid response received: {dns_response}");
    }

    Ok(())
}
