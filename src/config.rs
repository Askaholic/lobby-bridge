use lazy_static::lazy_static;
use std::env::{self, VarError};
use std::str::FromStr;

lazy_static! {
    pub static ref BIND_HOST: String = get_env("BIND_HOST", "localhost");
    pub static ref BIND_PORT: u16 = get_env("BIND_PORT", 8003u16);
    pub static ref LOBBY_HOST: String = get_env("LOBBY_HOST", "localhost");
    pub static ref LOBBY_PORT: u16 = get_env("LOBBY_PORT", 8002u16);

    // Conveniences derived from other statics
    pub static ref BIND_ADDR: String = format!("{}:{}", &*BIND_HOST, *BIND_PORT);
    pub static ref LOBBY_ADDR: String = format!("{}:{}", &*LOBBY_HOST, *LOBBY_PORT);
}

/// Trigger initialization of all config variables. Does nothing if it has already been called.
pub fn init() {
    let _ = *LOBBY_HOST;
    let _ = *LOBBY_PORT;

    let _ = *LOBBY_ADDR;
}

fn get_env<T: FromStr>(key: &'static str, default: impl Into<T>) -> T {
    env::var("LOBBY_PORT")
        .map_err(|e| match e {
            VarError::NotUnicode(_) => panic!("Env variable {} is not valid unicode", key),
            e => e,
        })
        .map(|s| {
            s.parse().unwrap_or_else(|_| {
                panic!(
                    "Env variable {} cannot be parsed as type {}",
                    key,
                    std::any::type_name::<T>()
                )
            })
        })
        .unwrap_or(default.into())
}
