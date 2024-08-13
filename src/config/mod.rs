use log::{debug, error, info, log_enabled, trace, warn, Level};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Location {
    pub proxy_pass: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub max_tcp_buffer_size: usize,
    pub location: std::collections::HashMap<String, Location>,
}

impl Config {
    pub fn new(path: &str) -> Self {
        debug!(
            "Try to read config from PWD: {}, path: {}",
            std::env::current_dir().unwrap().display(),
            path
        );

        let raw_config = std::fs::read_to_string(path);

        if raw_config.is_err() {
            panic!(
                "Failed to parse config.toml, Error: {}",
                raw_config.err().unwrap()
            );
        }

        toml::from_str(raw_config.unwrap().as_str()).unwrap()
    }
}
