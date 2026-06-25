use crate::ContainerData;

pub enum View {
    Containers,
    Log
}

pub struct App {
    pub containers: Vec<ContainerData>,
    pub container_idx: usize,
    pub current_logs: Vec<String>,
    pub log_autoscroll: bool,
    pub log_idx: usize,
    pub active_view: View
}

impl App {
    pub fn new() -> Self {
        Self {
            containers: Vec::new(),
            container_idx: 0,
            current_logs: Vec::new(),
            log_autoscroll: true,
            log_idx: 0,
            active_view: View::Containers
        }
    }
}
