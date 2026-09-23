use std::net::SocketAddr;

use thiserror::Error;

#[derive(Debug)]
pub struct Config {
    pub http_addr: SocketAddr,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read HTTP_ADDR: {0}")]
    ReadHttpAddr(#[from] std::env::VarError),
    #[error("invalid HTTP_ADDR: {0}")]
    InvalidHttpAddr(String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let http_addr = match std::env::var("HTTP_ADDR") {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => "0.0.0.0:8080".to_owned(),
            Err(error) => return Err(error.into()),
        };
        let http_addr = http_addr
            .parse()
            .map_err(|error| ConfigError::InvalidHttpAddr(format!("{http_addr}: {error}")))?;

        Ok(Self { http_addr })
    }
}
