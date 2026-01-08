// TODO: This module needs to be redesigned for the new typestate architecture
// The command processing logic has been moved to RaftNode::emit()

/*
pub mod leader {
    use crate::command::{CmdReq, CmdResp};
    use crate::node::node::Node;

    pub async fn append_entry(node: &Node, command: CmdReq) -> CmdResp {
        // TODO: Implement with new architecture
        todo!()
    }

    pub fn replicate_log(node: &mut Node) -> Result<(), String> {
        // Leader-specific replication logic
        todo!()
    }
}
*/
