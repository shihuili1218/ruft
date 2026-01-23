use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, RuftError>;

/// Raft error types classified by "how to handle"
#[derive(Debug)]
pub enum RuftError {
    /// Fatal error - process must exit, human intervention required
    /// Example: data corruption, unrecoverable state
    Fatal(String),

    /// Configuration error - wrong cluster setup, check config files
    /// Example: unknown member ID, invalid endpoint
    Configuration(String),

    /// Storage error - disk/persistence issues, may be temporary, can retry
    /// Example: disk full, write failed
    Storage(String),

    /// Network error - RPC/connection issues, temporary, wait for reconnect
    /// Example: connection timeout, peer unreachable
    Network(String),

    /// Invalid state - protocol logic, concurrent state issue, or shutdown
    /// Example: term mismatch, node shutting down
    InvalidState(String),

    /// IO error wrapper
    Io(io::Error),

    /// Serialization/deserialization error
    Serialization(String),
}

impl fmt::Display for RuftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuftError::Fatal(msg) => write!(f, "Fatal error: {}", msg),
            RuftError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            RuftError::Storage(msg) => write!(f, "Storage error: {}", msg),
            RuftError::Network(msg) => write!(f, "Network error: {}", msg),
            RuftError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            RuftError::Io(e) => write!(f, "IO error: {}", e),
            RuftError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for RuftError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RuftError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for RuftError {
    fn from(err: io::Error) -> Self {
        RuftError::Io(err)
    }
}
