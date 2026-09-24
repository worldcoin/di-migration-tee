use std::net::SocketAddr;

use thiserror::Error;

#[derive(Debug)]
pub struct Config {
    pub http_addr: SocketAddr,
    pub dynamodb_table_name: String,
    pub sqs_queue_url: String,
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
    #[error("SQS_QUEUE_URL is required")]
    MissingSqsQueueUrl,
    #[error("failed to read SQS_QUEUE_URL: {0}")]
    ReadSqsQueueUrl(std::env::VarError),
    #[error("SQS_QUEUE_URL must be an HTTP(S) URL with a queue path")]
    InvalidSqsQueueUrl,
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
        let sqs_queue_url = match std::env::var("SQS_QUEUE_URL") {
            Ok(value) if !value.is_empty() => value,
            Ok(_) | Err(std::env::VarError::NotPresent) => {
                return Err(ConfigError::MissingSqsQueueUrl);
            }
            Err(error) => return Err(ConfigError::ReadSqsQueueUrl(error)),
        };
        let queue_uri: axum::http::Uri = sqs_queue_url
            .parse()
            .map_err(|_| ConfigError::InvalidSqsQueueUrl)?;
        if !matches!(queue_uri.scheme_str(), Some("http" | "https"))
            || queue_uri.authority().is_none()
            || queue_uri.path() == "/"
        {
            return Err(ConfigError::InvalidSqsQueueUrl);
        }

        Ok(Self {
            http_addr,
            dynamodb_table_name,
            sqs_queue_url,
        })
    }
}
