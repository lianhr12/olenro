//! CLI errors

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid argument: {0}")]
    InvalidArg(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Operation failed: {0}")]
    Failed(String),
}

pub type Result<T> = std::result::Result<T, CliError>;
