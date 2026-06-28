use crate::ContainerData;
use ratatui::widgets::{ListState, TableState};

pub enum View {
    Containers,
    Log,
}

pub struct App {
    pub containers: Vec<ContainerData>,
    pub container_idx: usize,
    pub current_logs: Vec<String>,
    pub log_autoscroll: bool,
    pub log_idx: usize,
    pub active_view: View,
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
        }
    }
}
