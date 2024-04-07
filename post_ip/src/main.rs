use std::{env, error::Error};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};

use reqwest::Client;
use serde_json::{json, Value};

const BASE_URL: &'static str = "https://ccp.netcup.net/run/webservice/servers/endpoint.php?JSON";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().unwrap();

    let client = Client::new();
    let ip_conf_path = env::var("IP_CONFIG_PATH").unwrap();

    let api_key = env::var("API_KEY").unwrap();
    let api_pw = env::var("API_PW").unwrap();
    let cus_id = env::var("CUS_ID").unwrap();
    let domain_name = env::var("DOMAIN_NAME").unwrap();
    let cli_id = String::new();
    let new_ip = get_ip(ip_conf_path);
    let star_id = env::var("STAR_ID").unwrap();
    let at_id = env::var("AT_ID").unwrap();

    let login_payload = json!({
        "action": "login",
        "param": {
            "apikey": api_key,
            "apipassword": api_pw,
            "customernumber": cus_id
        }
    });

    let session_id = perform_request(&client, &login_payload).await?;

    let session_id = session_id["responsedata"]["apisessionid"].as_str();

    let dns_payload = json!({
        "action": "updateDnsRecords",
        "param": {
            "customernumber": cus_id,
            "apikey": api_key,
            "apisessionid": session_id,
            "clientrequestid": cli_id,
            "domainname": domain_name,
            "dnsrecordset": {
                "dnsrecords": [
                    {
                        "id": star_id,
                        "hostname": "*",
                        "type": "A",
                        "priority": "0",
                        "destination": new_ip,
                        "deleterecord": "FALSE",
                        "state": "yes"
                    },
                    {
                        "id": at_id,
                        "hostname": "@",
                        "type": "A",
                        "priority": "0",
                        "destination": new_ip,
                        "deleterecord": "FALSE",
                        "state": "yes"
                    }
                ]
            }
        }
    });

    let response = perform_request(&client, &dns_payload).await.unwrap();

    let was_successful = response["shortmessage"].as_str();
    assert_eq!(was_successful.unwrap().trim(), "DNS records successful updated");

    Ok(())
}

async fn perform_request(client: &Client, payload: &Value) -> Result<Value, reqwest::Error> {
    let response = client.post(BASE_URL)
                         .json(payload)
                         .send()
                         .await?;

    response.json().await
}

fn get_ip(path: String) -> String {
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .unwrap();

    let mut reader = BufReader::new(file);
    let mut ip = String::new();
    reader.read_line(&mut ip).unwrap();
    String::from(ip.trim())
}