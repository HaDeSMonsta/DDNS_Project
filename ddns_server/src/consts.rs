use std::env;
use std::sync::{LazyLock, Mutex};

pub const LOG_DIR: &'static str = "logs/";
pub const IO_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
pub const MAX_CONCURRENT_CONNECTIONS: u8 = 5;
pub const AUTH: LazyLock<String> = LazyLock::new(|| {
    get_env("AUTH")
});
pub const PORT: LazyLock<u16> = LazyLock::new(|| {
    get_env("PORT")
        .parse()
        .expect("PORT must be a valid u16")
});
pub const IP_CONFIG_PATH: LazyLock<String> = LazyLock::new(|| {
    get_env("IP_CONFIG_PATH")
});
pub const POST_IP_PATH: LazyLock<Option<String>> = LazyLock::new(|| {
    env::var("POST_IP_PATH").ok()
});

fn get_env(key: &'static str) -> String {
    env::var(key)
        .expect(&format!("Environment variable {} not set", key))
}
