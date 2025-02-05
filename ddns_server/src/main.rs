mod consts;

use consts::*;
use dotenv::dotenv;
use logger_utc::*;
use std::fs::OpenOptions;
use std::io::{BufWriter, ErrorKind, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::process::Command;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;
use std::{env, fs, thread};
use tracing::{debug, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

// Theory for failure: Panic when IP changes => No subtraction of arc
// Solution, start another thread with logic and join it
// If it panics we log the error, but continue

// TODO Change logging, we probably shouldn't log every connection with info, just changes
fn main() {
    dotenv().expect("No .env file found");
    {
        const KEY: &str = "LOG_LEVEL";
        let level = env::var(KEY)
            .unwrap_or(DEFAULT_LOG_LEVEL_STR.to_string())
            .parse::<Level>()
            .expect(&format!("{KEY} env var is set, but invalid: {:?}", env::var(KEY)));
        tracing::subscriber::set_global_default(
            FmtSubscriber::builder()
                .with_max_level(level)
                .finish()
        ).expect("Setting global default subscriber failed");
    }

    info!("Checking environment");

    let _ = *AUTH;
    let _ = *IP_CONFIG_PATH;

    match *POST_IP_PATH {
        Some(ref p) => info!("Post IP path set: {p}"),
        None => info!("Post IP path not set"),
    }

    let listen_address = format!("{}:{}", Ipv4Addr::UNSPECIFIED, *PORT);

    let listener = TcpListener::bind(&listen_address)
        .expect(&format!("Unable to bind {}", listen_address));
    info!("Started server on {listen_address}");

    let active_conns = Arc::new(AtomicU8::new(0));
    info!("Accepting up to {MAX_CONCURRENT_CONNECTIONS} connections");

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            warn!("Error accepting connection");
            continue;
        };

        if active_conns.load(Ordering::SeqCst) >= MAX_CONCURRENT_CONNECTIONS {
            warn!("Connection limit reached, rejecting");
            continue;
        }
        debug!(
            "Accepting new connection (currently running: {}): {stream:?}",
            active_conns.load(Ordering::SeqCst)
        );
        active_conns.fetch_add(1, Ordering::SeqCst);

        let active_conns = active_conns.clone();
        thread::spawn(move || {
            let handle = thread::spawn(move || {
                handle_connection(&mut stream);
                debug!("Connection closed: {stream:?}");
            });
            match handle.join() {
                Ok(_) => {}
                Err(e) => warn!("Error joining thread: {e:?}"),
            };
            active_conns.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

fn handle_connection(sock: &mut TcpStream) {
    let start = Instant::now();
    let max_run_secs = 5;
    let auth_token = &*AUTH;
    let ip_conf_path = &*IP_CONFIG_PATH;
    let client_ip = match sock.peer_addr() {
        Ok(addr) => {
            let ip = addr.ip().to_string();
            info!("Got client ip: {ip}");
            ip
        }
        Err(e) => {
            warn!("Unable to get client ({sock:?}) ip: {e}");
            return;
        }
    };

    //* This is using 100 % CPU of the assigned core, lets try to disable it
    // Ok, still the same problem, so what is causing it?
    let Ok(_) = sock.set_nonblocking(true) else {
        warn!("{client_ip}: Unable to set socket to non-blocking");
        return;
    };
    // */

    let mut tmp_buf = [0; 1024];
    let mut buf = vec![];

    let auth;

    'outer: loop {
        if start.elapsed().as_secs() > max_run_secs {
            info!("{client_ip}: Client reached timeout");
            return;
        }

        match sock.read(&mut tmp_buf) {
            Ok(n) => {
                debug!("{client_ip}: Read {n} bytes");
                for byte in &tmp_buf[..n] {
                    if *byte == b'\n' {
                        auth = String::from_utf8_lossy(&buf).trim().to_string();
                        debug!("{client_ip}: Reached end of msg for sock, auth: {auth}");
                        break 'outer;
                    }
                    buf.push(*byte);
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => {
                warn!("{client_ip}: Error reading from socket: {e}");
                return;
            }
        }
    }

    if auth != *auth_token {
        warn!("{client_ip}: Invalid auth token");
        return;
    }
    info!("{client_ip}: Authenticated");

    let response;
    let ip_changed;

    {
        debug!("{client_ip}: Locking IO");
        let _lock = *IO_LOCK.lock().unwrap();
        let Ok(existing_ip) = fs::read_to_string(&ip_conf_path) else {
            warn!("{client_ip}: Unable to read ip from file {ip_conf_path}");
            return;
        };
        let existing_ip = existing_ip.trim();
        ip_changed = client_ip != existing_ip;

        if ip_changed {
            let file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&ip_conf_path)
                .unwrap();

            let mut writer = BufWriter::new(file);

            match write!(writer, "{client_ip}") {
                Ok(_) => {}
                Err(e) => {
                    warn!("{client_ip}: Unable to write ne ip into file, returning: {e}");
                    return;
                }
            };

            let log_str = format!("New IP {client_ip} was written into config file, old: {existing_ip}");

            info!("{client_ip}: {log_str}");
            log_to_dyn_file(&log_str, Some(LOG_DIR), "changed_ips.log").unwrap();

            response = format!("New IP {client_ip} was written into config file. Old: {existing_ip}");
        } else {
            let log_str = format!("No Change in IP: New {client_ip} == old {existing_ip}");
            info!("{client_ip}: {log_str}");
            response = log_str;
        }
        debug!("{client_ip}: Unlocking IO");
    }

    if let Err(err) = sock.write_all(format!("{response}\n").as_bytes()) {
        warn!("{client_ip}: Unable to respond to client: {err}");
    }

    if !ip_changed {
        return;
    }

    let Some(command) = &*POST_IP_PATH else { return; };
    let command = command.clone();
    thread::spawn(move || {
        let mut cmd_dir = String::new();
        let mut dirs = command.split("/")
                              .collect::<Vec<_>>();
        dirs.remove(dirs.len() - 1);
        dirs.into_iter()
            .for_each(|dir| {
                cmd_dir.push_str(dir);
                cmd_dir.push_str("/");
            });
        debug!(
            "{client_ip}: Starting post_ip with {:?}",
            Command::new(&command)
                .current_dir(&cmd_dir)
        );
        let mut child = match Command::new(command)
            .current_dir(cmd_dir)
            .spawn() {
            Ok(child) => child,
            Err(e) => {
                warn!("{client_ip}: Unable to start post_ip: {e}");
                return;
            }
        };

        match child.wait() {
            Ok(code) => info!("{client_ip}: post_ip exited with {code}"),
            Err(e) => warn!("{client_ip}: post_ip failed with: {e}"),
        }
    });
}
