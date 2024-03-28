use std::env;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::process::Command;

use logger_utc::*;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    let port = env::var("PORT").expect("PORT must be set");
    let port: u16 = port.parse().expect("PORT must be a valid u16");
    let listen_address = format!("0.0.0.0:{port}");
    let ip_conf_path = "ip.conf";
    let auth = env::var("AUTH").expect("AUTH must be set");

    let listener = TcpListener::bind(&listen_address).await.unwrap();
    log_string(format!("Server listening on {}", listen_address));

    while let Ok((socket, _)) = listener.accept().await {
        tokio::spawn(handle_connection(socket, ip_conf_path, auth.parse().unwrap()));
    }
}

async fn handle_connection(mut socket: TcpStream, ip_config_path: &str, auth_token: String) {
    let mut buffer = [0; 1024];
    let mut ip = String::new();

    // Read the IP address and authentication token from the incoming request
    if let Ok(n) = socket.read(&mut buffer).await {
        let request = String::from_utf8_lossy(&buffer[..n]);

        if request.trim() == auth_token {
            log("Valid authentication token");
            ip = match socket.peer_addr() {
                Ok(ip_addr) => {
                    // Remove Port
                    let ip_addr = ip_addr.to_string().split(":").next().unwrap().to_string();
                    log_string(format!("Got client IP: {ip_addr}"));
                    ip_addr
                }
                Err(e) => {
                    eprintln!("Failed to get clients IP: {e}");
                    return;
                }
            }
        } else {
            log_to_dyn_file(
                "Invalid authentication",
                Some("logs/"),
                "invalid_ips.log")
                .unwrap();
            eprintln!("Invalid authentication token. Ignoring request.");
            return;
        }
    }

    // Read the existing IP from the configuration file
    let existing_ip = fs::read_to_string(&ip_config_path)
        .await
        .unwrap_or(String::new());
    let existing_ip = existing_ip.trim();

    let response;

    // Compare current and existing IPs, extract the existing IP to compare
    if ip != existing_ip {

        // Update the configuration file
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(ip_config_path)
            .unwrap();

        let mut writer = BufWriter::new(file);

        write!(writer, "{ip}").unwrap();

        let log_str = format!("New IP {ip} was written into config file. Old: {existing_ip}");

        log(&log_str);
        log_to_dyn_file(&log_str, Some("logs/"), "changed_ips.log").unwrap();

        response = format!("200 OK: New IP {ip} was written into config file. Old: {existing_ip}");
    } else {
        log_string(format!("No change in IP: New {ip} == old {existing_ip}"));
        response = format!("200 OK: No Change in IP: New {ip} == old {existing_ip}")
    }

    if let Ok(command) = env::var("POST_IP_PATH") {
        std::thread::spawn(move || {
            let mut command_dir = String::new();
            let mut dirs: Vec<_> = command.split("/").collect();
            dirs.remove(dirs.len() - 1);
            for dir in dirs {
                command_dir.push_str(dir);
                command_dir.push('/');
            }
            if let Err(err) = Command::new(format!("{command}"))
                .current_dir(format!("{command_dir}"))
                .arg(format!("{ip}"))
                .spawn() {
                log_to_dyn_file(
                    &format!("Error: {err}"),
                    Some("logs/"),
                    "post_ip_errs.log",
                ).unwrap();
            }
        });
    }

    // Respond to the client
    if let Err(err) = socket.write_all(format!("{response}").as_bytes()).await {
        let err = format!("Failed to respond to client: {}", err);
        log_to_dyn_file(&err, Some("logs/"), "err_res.log").unwrap();
        eprintln!("{err}");
    }
}
