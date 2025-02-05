use std::env;
use std::sync::{LazyLock, Mutex};

#[cfg(debug_assertions)]
pub const DEFAULT_LOG_LEVEL_STR: &str = "debug";
#[cfg(not(debug_assertions))]
pub const DEFAULT_LOG_LEVEL_STR: &str = "info";
pub const LOG_DIR: &'static str = "logs/";
pub static IO_LOCK: Mutex<()> = Mutex::new(());
pub const MAX_CONCURRENT_CONNECTIONS: u8 = 5;
pub static AUTH: LazyLock<String> = LazyLock::new(|| {
    get_env("AUTH")
});
pub static PORT: LazyLock<u16> = LazyLock::new(|| {
    get_env("PORT")
        .parse()
        .expect("PORT must be a valid u16")
});
pub static IP_CONFIG_PATH: LazyLock<String> = LazyLock::new(|| {
    get_env("IP_CONFIG_PATH")
});
pub static POST_IP_PATH: LazyLock<Option<String>> = LazyLock::new(|| {
    env::var("POST_IP_PATH").ok()
});

fn get_env(key: &'static str) -> String {
    env::var(key)
        .expect(&format!("Environment variable {} not set", key))
}
