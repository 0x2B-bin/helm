use crate::ContainerData;



pub struct App {
    pub containers: Vec<ContainerData>,
    selected_index: usize
}

impl App {
    pub fn new() -> Self {
        Self {
            containers: Vec::new(),
            selected_index: 0
        }
    }
}
