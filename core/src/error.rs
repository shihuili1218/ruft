use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, RuftError>;

#[derive(Debug)]
pub enum RuftError {
    Io(io::Error),
    Rpc(tonic::Status),
    Storage(String),
    InvalidState(String),
    Serialization(String),
}

impl fmt::Display for RuftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuftError::Io(e) => write!(f, "IO error: {}", e),
            RuftError::Rpc(e) => write!(f, "RPC error: {}", e),
            RuftError::Storage(msg) => write!(f, "Storage error: {}", msg),
            RuftError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            RuftError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for RuftError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RuftError::Io(e) => Some(e),
            RuftError::Rpc(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for RuftError {
    fn from(err: io::Error) -> Self {
        RuftError::Io(err)
    }
}

impl From<tonic::Status> for RuftError {
    fn from(err: tonic::Status) -> Self {
        RuftError::Rpc(err)
    }
}
