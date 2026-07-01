use crate::{
    AppEvent, DockerCMD,
    app::{App, TransitioningState, UiState, View},
    docker_worker::DockerHandle,
};
use ratatui::crossterm::event::KeyCode;
use tokio::sync::mpsc::Receiver;

pub struct EventHandler {
    receiver: Receiver<AppEvent>,
    init: bool,
}

impl EventHandler {
    pub fn new(receiver: Receiver<AppEvent>) -> Self {
        Self {
            receiver,
            init: true,
        }
    }

    pub async fn next(
        &mut self,
        app: &mut App,
        docker_handle: &DockerHandle,
        ui_state: &mut UiState,
    ) {
        if let Some(event) = self.receiver.recv().await {
            self.handle_event(app, docker_handle, event, ui_state);
        }
    }

    pub fn drain(
        &mut self,
        app: &mut App,
        docker_handle: &DockerHandle,
        ui_state: &mut UiState,
        amount: u8,
    ) {
        let mut drained = 0;

        while let Ok(event) = self.receiver.try_recv() {
            self.handle_event(app, docker_handle, event, ui_state);
            drained += 1;

            if drained >= amount {
                break;
            }
        }
    }

    fn handle_event(
        &mut self,
        app: &mut App,
        docker_handle: &DockerHandle,
        event: AppEvent,
        ui_state: &mut UiState,
    ) {
        match event {
            AppEvent::ContainerLoad(containers) => {
                app.containers = containers;
                if app.container_idx > app.containers.len() && !app.containers.is_empty() {
                    app.container_idx = app.containers.len() - 1;
                }

                if self.init {
                    docker_handle.send_cmd(DockerCMD::SwitchLogTarget(
                        app.containers[app.container_idx].id.clone(),
                    ));

                    if !app.containers.is_empty() {
                        ui_state.container_table.select(Some(app.container_idx));
                    }
                    self.init = false;
                }
            }
            AppEvent::NewLogLine(line) => {
                app.current_logs.push(line);

                if app.log_autoscroll && !app.current_logs.is_empty() {
                    app.log_idx = app.current_logs.len() - 1;
                    ui_state.log_list.select(Some(app.log_idx));
                }
            }
            AppEvent::TransitionComplete(container_id) => {
                let _ = app.transitioning_containers.remove(&container_id);
            }
            AppEvent::Key(key) => match app.active_view {
                View::Containers => match key.code {
                    KeyCode::Char('q') => {
                        ratatui::restore();
                        std::process::exit(0);
                    }
                    KeyCode::Char('j') => {
                        if app.container_idx < app.containers.len() - 1 {
                            app.container_idx += 1;
                            app.current_logs.clear();
                            docker_handle.send_cmd(DockerCMD::SwitchLogTarget(
                                app.containers[app.container_idx].id.clone(),
                            ));
                        }
                        ui_state.container_table.select(Some(app.container_idx));
                    }
                    KeyCode::Char('k') => {
                        if app.container_idx > 0 {
                            app.container_idx -= 1;
                            app.current_logs.clear();
                            docker_handle.send_cmd(DockerCMD::SwitchLogTarget(
                                app.containers[app.container_idx].id.clone(),
                            ));
                        }
                        ui_state.container_table.select(Some(app.container_idx));
                    }
                    KeyCode::Char('l') => {
                        if !app.current_logs.is_empty() {
                            app.log_idx = app.current_logs.len() - 1;
                        }
                        ui_state.log_list.select_last();
                        app.active_view = View::Log;
                    }
                    KeyCode::Char('s')
                        if !app.containers.is_empty()
                            && !app
                                .transitioning_containers
                                .contains_key(&app.containers[app.container_idx].id) =>
                    {
                        let container_id = app.containers[app.container_idx].id.clone();
                        docker_handle.send_cmd(DockerCMD::StopContainer(container_id.clone()));
                        app.transitioning_containers
                            .insert(container_id, TransitioningState::Stopping);
                    }
                    KeyCode::Char('y')
                        if !app.containers.is_empty()
                            && !app
                                .transitioning_containers
                                .contains_key(&app.containers[app.container_idx].id) =>
                    {
                        let container_id = app.containers[app.container_idx].id.clone();
                        docker_handle.send_cmd(DockerCMD::StartContainer(container_id.clone()));
                        app.transitioning_containers
                            .insert(container_id, TransitioningState::Starting);
                    }
                    KeyCode::Char('r')
                        if !app.containers.is_empty()
                            && !app
                                .transitioning_containers
                                .contains_key(&app.containers[app.container_idx].id) =>
                    {
                        let container_id = app.containers[app.container_idx].id.clone();
                        docker_handle.send_cmd(DockerCMD::RestartContainer(container_id.clone()));
                        app.transitioning_containers
                            .insert(container_id, TransitioningState::Restarting);
                    }
                    _ => {}
                },
                View::Log => match key.code {
                    KeyCode::Char('q') => {
                        ratatui::restore();
                        std::process::exit(0);
                    }
                    KeyCode::Char('j') => {
                        if app.log_idx + 1 < app.current_logs.len() {
                            app.log_idx += 1;
                            ui_state.log_list.select_next();
                        }
                        if app.log_idx + 1 == app.current_logs.len() {
                            app.log_autoscroll = true;
                        }
                    }
                    KeyCode::Char('k') if app.log_idx > 0 => {
                        app.log_autoscroll = false;
                        app.log_idx -= 1;
                        ui_state.log_list.select_previous();
                    }
                    KeyCode::Char('c') => {
                        app.active_view = View::Containers;
                    }
                    _ => {}
                },
            },
            _ => {}
        }
    }
}
