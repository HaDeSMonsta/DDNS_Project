#[macro_export]
macro_rules! get_env {
    ($env_key:expr) => {
        match ::std::env::var($env_key) {
            Ok(var) => var,
            Err(_) => {
                ::tracing::error!("{} is not set", $env_key);
                ::std::process::exit(1);
            }
        }
    };
    ($env_key:expr, $parse_type:ty) => {
        {
            let val = match ::std::env::var($env_key) {
                Ok(var) => var,
                Err(_) => {
                    ::tracing::error!("{} is not set", $env_key);
                    ::std::process::exit(1);
                }
            };
            match val.parse::<$parse_type>() {
                Ok(parsed_val) => parsed_val,
                Err(_) => {
                    ::tracing::error!("{} is not set to a valid {}: {val}",
                        $env_key, ::core::stringify!($parse_type));
                    ::std::process::exit(1);
                }
            }
        }
    };
}
