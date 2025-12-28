use crate::Config;
use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;
use crate::node::meta::MetaHolder;
use crate::repeat_timer::{RepeatTimer, RepeatTimerHandle};
use crate::role::candidate::Candidate;
use crate::role::state::State;
use crate::rpc::{RemoteClient, init_remote_client, run_server};
use dashmap::DashMap;
use rand::Rng;
use std::ops::Deref;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};

pub(crate) struct Node {
    meta: Mutex<MetaHolder>,
    me: Endpoint,
    remote_clients: DashMap<Endpoint, RemoteClient>,

    config: Config,
    pub state: RwLock<State>,
    timer: OnceLock<RepeatTimerHandle>,
}

impl Node {
    async fn start_rpc(self: &Arc<Self>) {
        self.init_server().await;
        self.init_client().await;
    }

    async fn init_server(self: &Arc<Self>) {
        let node = self.clone();
        let _rpc_server_handle = run_server(node).await;
    }

    async fn init_client(self: &Arc<Self>) {
        let node = self.clone();
        node.remote_clients.clear();

        let members = {
            let guard = node.meta.lock().await;
            guard.members()
        };

        for endpoint in members {
            if endpoint == node.me {
                continue;
            }

            match init_remote_client(&endpoint).await {
                Ok(client) => {
                    node.remote_clients.insert(endpoint.clone(), client);
                }
                Err(e) => {
                    error!("Failed to init remote client for {}: {}", endpoint, e);
                }
            }
        }
    }
}

impl Node {
    async fn start_timer(self: &Arc<Self>) {
        self.init_timer().await;
        if let Some(timer) = self.timer.get() {
            timer.restart();
        }
    }
    async fn init_timer(self: &Arc<Self>) {
        let node_for_delay = self.clone();
        let node_for_task = self.clone();

        let timer = RepeatTimer::new(
            "raft_timer".to_string(),
            Box::new(async move || {
                if let Ok(guard) = node_for_delay.state.read().await {
                    match guard.deref() {
                        State::Electing => Duration::from_millis(rand::thread_rng().gen_range(100..300)),
                        State::Following { .. } => Duration::from_millis(node_for_delay.config.heartbeat_interval_millis + 50),
                        State::Leading { .. } => Duration::from_millis(node_for_delay.config.heartbeat_interval_millis),
                        State::Learning { .. } => Duration::from_millis(node_for_delay.config.heartbeat_interval_millis + 50),
                    }
                } else {
                    //
                    Duration::from_millis(node_for_delay.config.heartbeat_interval_millis + 50)
                }
            }),
            Box::new(move || {
                if let Ok(guard) = node_for_task.state.read() {
                    match guard.deref() {
                        // elect leader
                        State::Electing => node_for_task.elect_leader(),
                        // wait heartbeat timeout
                        State::Following { .. } => {
                            drop(guard);
                            if let Ok(mut guard) = node_for_task.state.write() {
                                *guard = State::Electing;
                            }
                        }
                        // send heartbeat interval ends
                        State::Leading { .. } => node_for_task.send_heartbeat(),
                        // do nothing
                        State::Learning { .. } => {}
                    }
                }
            }),
        )
        .spawn();

        let _ = self.timer.set(timer);
    }
}

impl Node {
    pub fn new(me: Endpoint, config: Config) -> Self {
        let meta = MetaHolder::new(&config);
        Node {
            state: RwLock::new(State::Electing {
                candidate: Candidate::new(me.clone(), meta.members()),
            }),
            meta: Mutex::new(meta),
            me,
            config,
            timer: OnceLock::new(),
            remote_clients: DashMap::new(),
        }
    }

    pub fn start(self: &Arc<Self>) {
        let _ = self.start_timer();
        let _ = self.start_rpc();
    }

    fn elect_leader(&self) {
        let Ok(state) = self.state.read() else {
            warn!("elect_leader: failed to acquire state lock");
            return;
        };

        if !matches!(*state, State::Electing) {
            info!("elect_leader called but state is not Electing, {}", state);
            return;
        }

        // TODO: 实现选举逻辑
    }

    fn send_heartbeat(&self) {}

    pub fn update_member(&self, _endpoints: Vec<Endpoint>) {
        // TODO: 实现更新成员逻辑
    }

    pub fn emit(&self, cmd: CmdReq) -> CmdResp {
        let Ok(state) = self.state.read() else {
            return CmdResp::Failure {
                message: String::from("maybe electing?"),
            };
        };

        match state.deref() {
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
