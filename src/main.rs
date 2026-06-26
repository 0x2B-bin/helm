use app::{App, View};
use bollard::{
    Docker,
    config::{
        ContainerCpuStats, ContainerStatsResponse, ContainerSummary, ContainerSummaryStateEnum,
    },
    container::LogOutput,
    query_parameters::{ListContainersOptionsBuilder, LogsOptionsBuilder, StatsOptionsBuilder},
};
use futures::{StreamExt, stream::BoxStream};
use ratatui::crossterm::event;
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::{TableState, ListState};
use std::time::Duration;
use tokio::sync::mpsc;

mod app;
mod ui;

pub enum ContainerState {
    Running,
    Paused,
    Exited,
}

pub struct ContainerData {
    pub name: String,
    pub id: String,
    pub state: ContainerSummaryStateEnum,
    pub status: String,
    pub image: String,
    pub cpu_percentage: String,
    pub memory_usage: String,
    pub memory_limit: String,
}

enum AppEvent {
    Tick,
    Key(event::KeyEvent),
    ContainerLoad(Vec<ContainerData>),
    NewLogLine(String),
    #[allow(dead_code)]
    DockerError(String),
}

enum UiCommand {
    SwitchLogTarget(String),
}

fn transform_to_container_data(
    container: ContainerSummary,
    stats: Option<ContainerStatsResponse>,
) -> ContainerData {
    let name = container
        .names
        .as_ref()
        .and_then(|names| names.first())
        .cloned()
        .unwrap_or_else(|| "UNKOWN".to_string());

    let id = container.id.unwrap_or_default();
    let status = container.status.unwrap_or_default();
    let image = container.image.unwrap_or_default();
    let state = container
        .state
        .unwrap_or_else(|| ContainerSummaryStateEnum::EMPTY);

    let mut cpu_percentage = "0.00%".to_string();
    let mut memory_usage = "0".to_string();
    let mut memory_limit = "0".to_string();

    let get_cpu_total_usage =
        |s: &ContainerCpuStats| -> Option<u64> { Some(s.cpu_usage.as_ref()?.total_usage?) };

    let format_bytes = |bytes: u64| -> String {
        let mut size = bytes as f64;
        let units = ["B", "KiB", "MiB", "GiB", "TiB"];
        let mut unit_idx = 0;

        while size > 1024.0 && unit_idx < units.len() {
            size /= 1024.0;
            unit_idx += 1;
        }

        format!("{:.2} {}", size, units[unit_idx])
    };

    if let Some(s) = stats {
        match (s.cpu_stats, s.precpu_stats) {
            (Some(curr_cpu_stats), Some(prev_cpu_stats)) => {
                match (
                    get_cpu_total_usage(&curr_cpu_stats),
                    get_cpu_total_usage(&prev_cpu_stats),
                ) {
                    (Some(curr_usage), Some(prev_usage)) => {
                        let cpu_delta = curr_usage.saturating_sub(prev_usage);
                        let system_delta = curr_cpu_stats
                            .system_cpu_usage
                            .unwrap_or(0)
                            .saturating_sub(prev_cpu_stats.system_cpu_usage.unwrap_or(0));

                        if system_delta > 0 && cpu_delta > 0 {
                            let percent = (cpu_delta as f64 / system_delta as f64) * 100.0;
                            cpu_percentage = format!("{:.2}%", percent);
                        }
                    }
                    _ => {}
                }

                match s.memory_stats {
                    Some(memory_stats) => {
                        let usage = memory_stats.usage.unwrap_or(0);
                        let limit = memory_stats.limit.unwrap_or(0);

                        memory_usage = format_bytes(usage);
                        memory_limit = format_bytes(limit);
                    }
                    None => {}
                }
            }
            _ => {}
        }
    }

    ContainerData {
        name,
        id,
        state,
        status,
        image,
        cpu_percentage,
        memory_usage,
        memory_limit,
    }
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<AppEvent>(100);

    let tx_key = tx.clone();
    tokio::spawn(async move {
        loop {
            if event::poll(Duration::from_millis(50)).unwrap() {
                if let event::Event::Key(key) = event::read().unwrap() {
                    if key.kind != event::KeyEventKind::Release {
                        let _ = tx_key.send(AppEvent::Key(key)).await;
                    }
                }
            }
            let _ = tx_key.send(AppEvent::Tick).await;
            let _ = tokio::time::sleep(Duration::from_millis(33)).await;
        }
    });

    let tx_docker = tx.clone();
    tokio::spawn(async move {
        let docker = Docker::connect_with_local_defaults().unwrap();

        loop {
            let list_config = ListContainersOptionsBuilder::new().all(true).build();

            match docker.list_containers(Some(list_config)).await {
                Ok(container_summery) => {
                    let stat_futures = container_summery.into_iter().map(|container| {
                        let docker_clone = docker.clone();
                        async move {
                            let container_id = container.id.clone().unwrap_or_default();
                            let stats_options = StatsOptionsBuilder::default()
                                .stream(false)
                                .one_shot(true)
                                .build();

                            let mut stats_stream =
                                docker_clone.stats(&container_id, Some(stats_options));
                            let stats = stats_stream.next().await.and_then(|res| res.ok());

                            transform_to_container_data(container, stats)
                        }
                    });

                    let payload = futures::future::join_all(stat_futures).await;
                    let _ = tx_docker.send(AppEvent::ContainerLoad(payload)).await;
                }
                Err(err) => {
                    let _ = tx_docker.send(AppEvent::DockerError(err.to_string())).await;
                }
            }
            let _ = tokio::time::sleep(Duration::from_millis(750)).await;
        }
    });

    let (tx_ui, mut rx_ui) = mpsc::channel::<UiCommand>(100);
    let tx_log = tx.clone();
    tokio::spawn(async move {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let mut log_handle: Option<tokio::task::JoinHandle<()>> = None;

        loop {
            if let Some(ui_command) = rx_ui.recv().await {
                match ui_command {
                    UiCommand::SwitchLogTarget(container_id) => {
                        if let Some(handle) = log_handle {
                            handle.abort();
                        }

                        let log_options = LogsOptionsBuilder::new()
                            .follow(true)
                            .tail("50")
                            .stdout(true)
                            .build();

                        let mut log_stream = docker.logs(&container_id, Some(log_options));
                        let tx_log_clone = tx_log.clone();
                        let handle = tokio::spawn(async move {
                            while let Some(Ok(log_output)) = log_stream.next().await {
                                if let LogOutput::StdOut { message: bytes } = log_output {
                                    let line =
                                        String::from_utf8_lossy(&Vec::from(bytes)).to_string();
                                    let _ = tx_log_clone.send(AppEvent::NewLogLine(line)).await;
                                }
                            }
                        });

                        log_handle = Some(handle)
                    }
                }
            }
        }
    });

    let mut terminal = ratatui::init();
    let mut app = App::new();
    let mut table_state = TableState::default();
    let mut list_state = ListState::default();
    table_state.select_next();

    loop {
        if let Some(event) = rx.recv().await {
            handle_event(event, &mut app, &mut table_state, &mut list_state, &tx_ui);

            while let Ok(event) = rx.try_recv() {
                handle_event(event, &mut app, &mut table_state, &mut list_state, &tx_ui);
            }
        }

        let _ = terminal.draw(|frame| ui::render(frame, &app, &mut table_state, &mut list_state));
    }
}

fn handle_event(
    event: AppEvent,
    app: &mut App,
    table_state: &mut TableState,
    log_list_state: &mut ListState,
    tx_ui: &mpsc::Sender<UiCommand>,
) {
    match event {
        AppEvent::ContainerLoad(containers) => {
            app.containers = containers;

        }
        AppEvent::NewLogLine(line) => {
            app.current_logs.push(line);

            if app.log_autoscroll && app.current_logs.len() > 0 {
                app.log_idx = app.current_logs.len() - 1;
            }
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
                        let _ = tx_ui.try_send(UiCommand::SwitchLogTarget(
                            app.containers[app.container_idx].id.clone(),
                        ));
                    }
                    table_state.select_next();
                }
                KeyCode::Char('k') => {
                    if app.container_idx > 0 {
                        app.container_idx -= 1;
                        app.current_logs.clear();
                        let _ = tx_ui.try_send(UiCommand::SwitchLogTarget(
                            app.containers[app.container_idx].id.clone(),
                        ));
                    }
                    table_state.select_previous();
                }
                KeyCode::Char('l') => {
                    if app.current_logs.len() > 0 {
                        app.log_idx = app.current_logs.len() - 1;
                    }
                    log_list_state.select_last();
                    app.active_view = View::Log;
                }
                _ => {}
            }
            View::Log => match key.code {
                KeyCode::Char('q') => {
                    ratatui::restore();
                    std::process::exit(0);
                }
                KeyCode::Char('j') => {
                    if app.log_idx + 1 < app.current_logs.len() {
                        app.log_idx += 1;
                        log_list_state.select_next();
                    }
                    if app.log_idx + 1 == app.current_logs.len() {
                        app.log_autoscroll = true;
                    }
                }
                KeyCode::Char('k') => {
                    if app.log_idx > 0 {
                        app.log_autoscroll = false;
                        app.log_idx -= 1;
                        log_list_state.select_previous();
                    }
                }
                KeyCode::Char('c') => {
                    app.active_view = View::Containers;
                }
                _ => {}
            }
        },
        _ => {}
    }
}
