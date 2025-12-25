pub mod ruft;

use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;
use crate::meta::Meta;
use crate::repeat_timer::{RepeatTimer, RepeatTimerHandle};
use crate::role::state::State;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

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
    id: u64,
    meta: Mutex<Meta>,
    pub state: Mutex<State>,
    timer: OnceLock<Timer>,
}

impl Node {
    pub fn new(_config: Config) -> Self {
        Node {
            id: 0,
            meta: Mutex::new(Meta::new(PathBuf::new())),
            state: Mutex::new(State::Electing),
            timer: OnceLock::new(),
        }
    }

    pub fn start(self: &Arc<Self>) {
        self.init_timer();
    }

    fn init_timer(self: &Arc<Self>) {
        let node_for_send_hb = self.clone();
        let send_heartbeat_timer = RepeatTimer::new(
            "send_heartbeat".to_string(),
            Box::new(|| Duration::from_millis(100)),
            Box::new(move || {
                node_for_send_hb.send_heartbeat();
            }),
        )
        .spawn();

        let node_for_wait_hb = self.clone();
        let wait_heartbeat_timer = RepeatTimer::new(
            "wait_heartbeat".to_string(),
            Box::new(|| Duration::from_millis(100)),
            Box::new(move || {
                if let Some(timer) = node_for_wait_hb.timer.get() {
                    timer.restart_elect();
                }
            }),
        )
        .spawn();

        let node_for_elect = self.clone();
        let elect_timer = RepeatTimer::new(
            "elect".to_string(),
            Box::new(|| Duration::from_millis(100)),
            Box::new(move || {
                node_for_elect.elect();
            }),
        )
        .spawn();

        let _ = self.timer.set(Timer {
            send_heartbeat_timer,
            wait_heartbeat_timer,
            elect_timer,
        });
    }

    fn elect(&self) {}

    fn send_heartbeat(&self) {}

    pub fn update_member(&self, _endpoints: Vec<Endpoint>) {
        // TODO: 实现更新成员逻辑
    }

    pub fn emit(&self, cmd: CmdReq) -> CmdResp {
        let state = self.state.lock().unwrap();
        match &*state {
            State::Electing => CmdResp::Failure {
                message: String::from("Electing"),
            },
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

pub struct Config {
    origin_endpoint: Vec<Endpoint>,
}

impl Config {
    pub fn new(endpoints: Vec<Endpoint>) -> Self {
        Config {
            origin_endpoint: endpoints,
        }
    }
}
