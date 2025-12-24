use bytes::Bytes;

pub struct CmdReq {
    pub id: String,
    pub data: Bytes,
}

pub enum CmdResp {
    Success,
    Failure { message: String },
    Unknown { log_id: usize },
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let cmd = CmdReq {
            id: "cmd_789".to_string(),
            data: Bytes::from(b"network_data".to_vec()),
        };
        // 克隆 Bytes 是零拷贝（仅复制引用计数）
        let cmd_clone = CmdReq {
            id: cmd.id.clone(),
            data: cmd.data.clone(),
        };
        assert_eq!(cmd.id, cmd_clone.id);
        assert_eq!(cmd.data, cmd_clone.data);
    }
}


