use std::collections::HashMap;
use std::fmt;
use crate::ContainerData;
use ratatui::widgets::{ListState, TableState};

pub enum View {
    Containers,
    Log,
}

pub enum TransitioningState {
    Stopping,
    Restarting,
    Starting,
}


pub struct App {
    pub containers: Vec<ContainerData>,
    pub container_idx: usize,
    pub current_logs: Vec<String>,
    pub log_autoscroll: bool,
    pub log_idx: usize,
    pub active_view: View,
    pub transitioning_containers: HashMap<String, TransitioningState>,
}

#[derive(Default)]
pub struct UiState {
    pub container_table: TableState,
    pub log_list: ListState,
}

impl App {
    pub fn new() -> Self {
        Self {
            containers: Vec::new(),
            container_idx: 0,
            current_logs: Vec::new(),
            log_autoscroll: true,
            log_idx: 0,
            active_view: View::Containers,
            transitioning_containers: HashMap::new(),
        }
    }
}

impl fmt::Display for TransitioningState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            TransitioningState::Stopping => write!(f, "stoppping"),
            TransitioningState::Restarting => write!(f, "restarting"),
            TransitioningState::Starting => write!(f, "starting"),
        }
    }
}
