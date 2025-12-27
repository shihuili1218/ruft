use crate::Config;
use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;
use crate::node::meta::MetaHolder;
use crate::repeat_timer::{RepeatTimer, RepeatTimerHandle};
use crate::role::state::State;
use crate::rpc::client::{LocalClient, RaftRpcClient, RemoteClient};
use crate::rpc::server::run_server;
use crate::rpc::{init_local_client, init_remote_client, run_server, RaftRpcClient};
use rand::random_range;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Mutex;

struct Timer {
    send_heartbeat_timer: RepeatTimerHandle,
    wait_heartbeat_timer: RepeatTimerHandle,
    elect_timer: RepeatTimerHandle,
}

impl Timer {
    fn restart_wait_hb(&self) {
        self.send_heartbeat_timer.stop();
        self.wait_heartbeat_timer.stop();
        self.elect_timer.stop();
        self.wait_heartbeat_timer.restart();
    }

    fn restart_send_hb(&self) {
        self.send_heartbeat_timer.stop();
        self.wait_heartbeat_timer.stop();
        self.elect_timer.stop();
        self.send_heartbeat_timer.restart();
    }

    fn restart_elect(&self) {
        self.send_heartbeat_timer.stop();
        self.wait_heartbeat_timer.stop();
        self.elect_timer.stop();
        self.elect_timer.restart();
    }
}

pub(crate) struct Node {
    meta: Mutex<MetaHolder>,
    me: Endpoint,
    config: Config,

    pub state: Mutex<State>,
    timer: OnceLock<RepeatTimerHandle>,
}

impl Node {
    pub fn new(me: Endpoint, config: Config) -> Self {
        let meta = MetaHolder::new(&config);
        Node {
            meta: Mutex::new(meta),
            me,
            config,
            state: Mutex::new(State::Electing),
            timer: OnceLock::new(),
        }
    }

    pub fn start(self: &Arc<Self>) {
        self.start_timer();
        self.start_rpc();
    }

    fn elect_leader(&self) {
        if let State::Electing() = self.state.into_inner() {}
    }

    fn send_heartbeat(&self) {}

    pub fn update_member(&self, _endpoints: Vec<Endpoint>) {
        // TODO: 实现更新成员逻辑
    }

    pub fn emit(&self, cmd: CmdReq) -> CmdResp {
        let state = self.state.lock().unwrap();
        match &*state {
            State::Electing => CmdResp::Failure { message: String::from("Electing") },
            State::Leading { term: _, leader } => leader.append_entry(cmd),
            State::Following { term, follower } => CmdResp::Failure {
                message: format!("Following, leader[{}]: {}", term, follower.leader),
            },
            State::Learning { term, learner } => CmdResp::Failure {
                message: format!("Learning, leader[{}]: {}", term, learner.leader),
            },
        }
    }
}

impl Node {
    fn start_rpc(&self) {
        self.init_server();
        self.init_client();
    }

    fn init_server(self: &Self) {
        tokio::spawn(async move {
            let _rpc_server_handle = run_server(self).await;
        });
        // if let Ok(rt) = tokio::runtime::Runtime::new() {
        //     rt.block_on(async {
        //         let _rpc_server_handle = run_server(self).await;
        //     });
        // }
    }

    fn init_client(self: &mut Self) {
        let d_ = self.meta.get_mut().members().iter().map(|e| {
            let is_me = *e == self.me;
            let zz = if is_me {
                let result = init_local_client(self).expect("init local client failed");
            } else {
                let client = tokio::spawn(async move { init_remote_client(e).await.expect("init remote client failed") });
            };
        });
    }
}

impl Node {
    fn start_timer(self: &Arc<Self>) {
        self.init_timer();
        if let Some(timer) = self.timer.get() {
            timer.restart();
        }
    }
    fn init_timer(self: &Arc<Self>) {
        let node_for_delay = self.clone();
        let node_for_task = self.clone();

        let timer = RepeatTimer::new(
            "raft_timer".to_string(),
            Box::new(move || {
                let state = node_for_delay.state.lock().unwrap();
                match &*state {
                    State::Electing => Duration::from_millis(random_range(100..300)),
                    State::Following { .. } => Duration::from_millis(node_for_delay.config.heartbeat_interval_millis + 50),
                    State::Leading { .. } => Duration::from_millis(node_for_delay.config.heartbeat_interval_millis),
                    State::Learning { .. } => Duration::from_millis(node_for_delay.config.heartbeat_interval_millis + 50),
                }
            }),
            Box::new(move || {
                let state = node_for_task.state.lock().unwrap();
                match &*state {
                    // elect leader
                    State::Electing => node_for_task.elect_leader(),
                    // wait heartbeat timeout
                    State::Following { .. } => *state = State::Electing,
                    // send heartbeat interval ends
                    State::Leading { .. } => node_for_task.send_heartbeat(),
                    // do nothing
                    State::Learning { .. } => {}
                }
            }),
        )
        .spawn();

        let _ = self.timer.set(timer);
    }
}
