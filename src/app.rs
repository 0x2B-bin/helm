use crate::ContainerData;

pub struct App {
    pub containers: Vec<ContainerData>,
    pub container_idx: usize,
    pub current_logs: Vec<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            containers: Vec::new(),
            container_idx: 0,
            current_logs: Vec::new(),
        }
    }
}
