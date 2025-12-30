use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;
use crate::node::meta::MetaHolder;
use crate::repeat_timer::{RepeatTask, RepeatTimer, RepeatTimerHandle};
use crate::role::candidate::Candidate;
use crate::role::leader::leader::append_entry;
use crate::role::state::State;
use crate::rpc::{init_remote_client, run_server, RemoteClient};
use crate::Config;
use dashmap::DashMap;
use rand::Rng;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};
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
        self.remote_clients.clear();

        let members = {
            let guard = self.meta.lock().await;
            guard.members()
        };

        for endpoint in members {
            if endpoint == self.me {
                continue;
            }

            match init_remote_client(&endpoint).await {
                Ok(client) => {
                    self.remote_clients.insert(endpoint, client);
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

        let timer = RepeatTimer::from_fns(
            "raft_timer".to_string(),
            move || {
                let node = node_for_delay.clone();
                Box::pin(async move {
                    let guard = node.state.read().await;
                    match guard.deref() {
                        State::Electing { .. } => Duration::from_millis(rand::thread_rng().gen_range(100..300)),
                        State::Following { .. } => Duration::from_millis(node.config.heartbeat_interval_millis + 50),
                        State::Leading { .. } => Duration::from_millis(node.config.heartbeat_interval_millis),
                        State::Learning { .. } => Duration::from_millis(node.config.heartbeat_interval_millis + 50),
                    }
                })
            },
            move || {
                let node = node_for_task.clone();
                Box::pin(async move {
                    let state_guard = node.state.read().await;
                    match state_guard.deref() {
                        // elect leader
                        State::Electing { .. } => node.elect_leader().await,
                        // wait heartbeat timeout
                        State::Following { .. } => {
                            drop(state_guard);
                            let mut state_guard = node.state.write().await;
                            let mut meta_guard = node.meta.lock().await;
                            let next_term = meta_guard.next_term();
                            *state_guard = State::Electing { term: next_term, votes_received: 0 };
                        }
                        // send heartbeat interval ends
                        State::Leading { .. } => node.send_heartbeat().await,
                        // do nothing
                        State::Learning { .. } => {}
                    }
                })
            },
        )
        .spawn();

        let _ = self.timer.set(timer);
    }
}

impl Node {
    pub fn new(me: Endpoint, config: Config) -> Self {
        let mut meta = MetaHolder::new(&config);
        let next_term = meta.next_term();
        Node {
            state: RwLock::new(State::Electing { term: next_term, votes_received: 0 }),
            meta: Mutex::new(meta),
            me,
            config,
            timer: OnceLock::new(),
            remote_clients: DashMap::new(),
        }
    }

    pub async fn start(self: &Arc<Self>) {
        self.start_timer().await;
        self.start_rpc().await;
    }

    async fn elect_leader(&self) {
        let guard = self.state.read().await;
        if let State::Electing { term, votes_received } = &*guard {
        } else {
            info!("elect_leader called but state is not Electing");
        };
    }

    async fn send_heartbeat(&self) {}

    pub async fn update_member(&self, _endpoints: Vec<Endpoint>) {}

    pub async fn emit(&mut self, cmd: CmdReq) -> CmdResp {
        append_entry(self, cmd).await
    }
}
