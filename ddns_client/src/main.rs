use std::{env, thread};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use tracing::{debug, info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() {
    {
        #[cfg(debug_assertions)]
        let level = Level::DEBUG;
        #[cfg(not(debug_assertions))]
        let level = Level::INFO;
        tracing::subscriber::set_global_default(
            FmtSubscriber::builder()
                .with_max_level(level)
                .finish()
        ).expect("Setting global default subscriber failed");
    }

    info!("Checking environment");

    let server_address = env::var("SERVER_ADDRESS")
        .expect("SERVER_ADDRESS must be set");
    let sleep_mins: u64 = env::var("SLEEP_MINS")
        .unwrap_or(String::from("15"))
        .parse()
        .expect("SLEEP_MINS should be convertable to u64");
    assert_ne!(sleep_mins, 0, "SLEEP_MINS must be > 0");

    let auth = env::var("AUTH")
        .expect("AUTH must be set");

    info!("Target address: {server_address}");

    loop {
        {
            info!("Connecting to server");
            let mut stream = TcpStream::connect(&server_address)
                .expect("Failed to connect to server");
            stream.set_nonblocking(true)
                  .expect("Failed to set non-blocking");

            debug!("Authenticating");

            stream.write_all(format!("{auth}\n").as_bytes())
                  .expect("Unable to write to server");

            let mut tmp_buf = [0; 1024];
            let mut buffer = vec![];

            let start = Instant::now();
            'outer: loop {
                if start.elapsed().as_secs() > 50 { panic!("Timeout"); }
                match stream.read(&mut tmp_buf) {
                    Ok(n) => {
                        debug!("Read {n} bytes");

                        for byte in &tmp_buf[..n] {
                            if *byte == b'\n' {
                                debug!("Reached end of response");
                                break 'outer;
                            }
                            buffer.push(*byte);
                        }
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
                    Err(e) => panic!("Failed to read from server: {e}"),
                }
            }

            let res = String::from_utf8_lossy(&buffer);
            info!("Response from server: {res}");
        }
        info!("Sleeping for {sleep_mins} minutes");
        thread::sleep(Duration::from_secs(sleep_mins * 60));
    }
}
