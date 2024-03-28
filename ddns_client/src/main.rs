use std::env;
use std::time::Duration;
use logger_utc::log;
use logger_utc::log_string;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let server_address = env::var("SERVER_ADDRESS").expect("SERVER_ADDRESS must be set");
    let sleep_mins = env::var("SLEEP_MINS").unwrap_or(String::from("15"));
    let sleep_mins: u64 = sleep_mins.parse().expect("SLEEP_MINS should be convertable to u64");
    if sleep_mins == 0 { panic!("SLEEP_MINS must be > 0") }

    let auth = env::var("AUTH").expect("AUTH must be set");
    log_string(format!("Address: {server_address}"));

    loop {
        {
            let mut stream = TcpStream::connect(&server_address).await?;

            // Write the authentication token to the server
            stream.write_all(auth.as_bytes()).await?;

            log("Sent identification to the server");

            // Read response
            let mut buffer = [0; 1024];
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                // 0 bytes received => connection closed
                log("Connection closed");
            } else {
                let response = String::from_utf8(buffer[0..n].to_vec())
                    .expect("Failed to convert Server response bytes to String");
                log_string(format!("Server response: {response}"));
            }
        }
        log_string(format!("Sleeping for {} seconds", sleep_mins * 60));
        tokio::time::sleep(Duration::from_secs(sleep_mins * 60)).await;
    }
}
