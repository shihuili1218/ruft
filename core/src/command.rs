use crate::endpoint::Endpoint;
use bytes::Bytes;

#[derive(Clone, Debug)]
pub struct CmdReq {
    pub id: String,
    pub data: Bytes,
}

/// Response to a command submission
#[derive(Clone, Debug)]
pub enum CmdResp {
    /// Command accepted and applied successfully
    Success {
        /// Optional data returned by the state machine
        data: Option<Bytes>,
    },
    /// Not the leader - client should retry with the leader
    NotLeader {
        /// Known leader endpoint, if any
        leader: Option<Endpoint>,
    },
    /// Command rejected with structured error
    Rejected {
        /// Error code for programmatic handling
        code: ErrorCode,
        /// Human-readable error message
        message: String,
    },
    /// Command is being replicated (async response)
    Pending {
        /// Log index where command was appended
        log_index: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    /// Cluster has no leader
    NoLeader,
    /// Not enough nodes to form quorum
    NoQuorum,
    /// Log is full or storage error
    StorageFull,
    /// Invalid command format
    InvalidCommand,
    /// Command timeout
    Timeout,
    /// Internal error
    Internal,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::NoLeader => write!(f, "NO_LEADER"),
            ErrorCode::NoQuorum => write!(f, "NO_QUORUM"),
            ErrorCode::StorageFull => write!(f, "STORAGE_FULL"),
            ErrorCode::InvalidCommand => write!(f, "INVALID_COMMAND"),
            ErrorCode::Timeout => write!(f, "TIMEOUT"),
            ErrorCode::Internal => write!(f, "INTERNAL"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_req_clone() {
        let cmd = CmdReq {
            id: "cmd_789".to_string(),
            data: Bytes::from(b"network_data".to_vec()),
        };
        // Bytes clone is zero-copy (reference counted)
        let cmd_clone = cmd.clone();
        assert_eq!(cmd.id, cmd_clone.id);
        assert_eq!(cmd.data, cmd_clone.data);
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::NoLeader.to_string(), "NO_LEADER");
        assert_eq!(ErrorCode::Timeout.to_string(), "TIMEOUT");
    }
}
