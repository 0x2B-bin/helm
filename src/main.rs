use app::{App, UiState};
use bollard::{
    Docker,
    config::{
        ContainerCpuStats, ContainerStatsResponse, ContainerSummary, ContainerSummaryStateEnum,
    },
    query_parameters::{ListContainersOptionsBuilder, StatsOptionsBuilder},
};
use docker_worker::{DockerCMD, DockerWorker};
use futures::StreamExt;
use ratatui::crossterm::event;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::docker_worker::DockerHandle;

mod app;
mod docker_worker;
mod event_handler;
mod ui;

pub struct ContainerData {
    pub name: String,
    pub id: String,
    pub state: ContainerSummaryStateEnum,
    pub state_string: String,
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
    TransitionComplete(String),
    //#[allow(dead_code)]
    DockerError(String),
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
        .unwrap_or(ContainerSummaryStateEnum::EMPTY);
    let state_string = format!("{}", state).to_lowercase();
    let mut cpu_percentage = "0.00%".to_string();
    let mut memory_usage = "0".to_string();
    let mut memory_limit = "0".to_string();

    let get_cpu_total_usage =
        |s: &ContainerCpuStats| -> Option<u64> { s.cpu_usage.as_ref()?.total_usage };

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

    if let Some(s) = stats
        && let (Some(curr_cpu_stats), Some(prev_cpu_stats)) = (s.cpu_stats, s.precpu_stats)
    {
        if let (Some(curr_usage), Some(prev_usage)) = (
            get_cpu_total_usage(&curr_cpu_stats),
            get_cpu_total_usage(&prev_cpu_stats),
        ) {
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

        if let Some(memory_stats) = s.memory_stats {
            let usage = memory_stats.usage.unwrap_or(0);
            let limit = memory_stats.limit.unwrap_or(0);

            memory_usage = format_bytes(usage);
            memory_limit = format_bytes(limit);
        }
    }

    ContainerData {
        name,
        id,
        state,
        state_string,
        status,
        image,
        cpu_percentage,
        memory_usage,
        memory_limit,
    }
}

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_local_defaults().unwrap();

    let (tx, rx) = mpsc::channel::<AppEvent>(100);

    let tx_key = tx.clone();
    tokio::spawn(async move {
        loop {
            if event::poll(Duration::from_millis(50)).unwrap()
                && let event::Event::Key(key) = event::read().unwrap()
                && key.kind != event::KeyEventKind::Release
            {
                let _ = tx_key.send(AppEvent::Key(key)).await;
            }
            let _ = tx_key.send(AppEvent::Tick).await;
            let _ = tokio::time::sleep(Duration::from_millis(33)).await;
        }
    });

    let tx_docker = tx.clone();
    let docker_1 = docker.clone();
    tokio::spawn(async move {
        loop {
            let list_config = ListContainersOptionsBuilder::new().all(true).build();

            match docker_1.list_containers(Some(list_config)).await {
                Ok(container_summery) => {
                    let stat_futures = container_summery.into_iter().map(|container| {
                        let docker_clone = docker_1.clone();
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

    let (tx_docker_cmd, rx_docker_cmd) = mpsc::channel::<DockerCMD>(50);
    let tx_docker_app_event = tx.clone();
    let docker_2 = docker.clone();
    DockerWorker::spawn(rx_docker_cmd, tx_docker_app_event, docker_2);
    let docker_handle = DockerHandle::new(tx_docker_cmd);

    let mut terminal = ratatui::init();
    let mut app = App::new();
    let mut ui_state = UiState::default();
    let mut event_handler = event_handler::EventHandler::new(rx);

    loop {
        event_handler
            .next(&mut app, &docker_handle, &mut ui_state)
            .await;
        event_handler.drain(&mut app, &docker_handle, &mut ui_state, 10);
        let _ = terminal.draw(|frame| ui::render(frame, &app, &mut ui_state));
    }
}
