use std::net::SocketAddr;

use thiserror::Error;

#[derive(Debug)]
pub struct Config {
    pub http_addr: SocketAddr,
    pub dynamodb_table_name: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read HTTP_ADDR: {0}")]
    ReadHttpAddr(#[from] std::env::VarError),
    #[error("invalid HTTP_ADDR: {0}")]
    InvalidHttpAddr(String),
    #[error("DYNAMODB_TABLE_NAME is required")]
    MissingDynamodbTableName,
    #[error("failed to read DYNAMODB_TABLE_NAME: {0}")]
    ReadDynamodbTableName(std::env::VarError),
    #[error(
        "DYNAMODB_TABLE_NAME must be 3-255 ASCII letters, digits, underscores, hyphens, or dots"
    )]
    InvalidDynamodbTableName,
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
        let dynamodb_table_name = match std::env::var("DYNAMODB_TABLE_NAME") {
            Ok(value) if !value.trim().is_empty() => value,
            Ok(_) | Err(std::env::VarError::NotPresent) => {
                return Err(ConfigError::MissingDynamodbTableName);
            }
            Err(error) => return Err(ConfigError::ReadDynamodbTableName(error)),
        };
        if !(3..=255).contains(&dynamodb_table_name.len())
            || !dynamodb_table_name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err(ConfigError::InvalidDynamodbTableName);
        }

        Ok(Self {
            http_addr,
            dynamodb_table_name,
        })
    }
}
