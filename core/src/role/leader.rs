pub mod leader {
    use crate::command::{CmdReq, CmdResp};
    use crate::node::node::Node;
    use crate::role::state::State;

    pub async  fn append_entry(node: &mut Node, command: CmdReq) -> CmdResp {
        let asset_result = {
            let guard = node.state.read().await;
            match &*guard {
                State::Electing { .. } => Some(CmdResp::Failure { message: String::from("Electing") }),
                State::Leading { term: _, .. } => None,
                State::Following { term, leader } => Some(CmdResp::Failure {
                    message: format!("Following, leader[{}]: {}", term, leader),
                }),
                State::Learning { term, leader } => Some(CmdResp::Failure {
                    message: format!("Learning, leader[{}]: {}", term, leader),
                }),
            }
        };
        if let Some(resp) = asset_result {
            return resp;
        }

        todo!()
    }

    pub fn replicate_log(node: &mut Node) -> Result<(), String> {
        // Leader 特有的逻辑
        todo!()
    }
}
