use std::time::Duration;
use futures::StreamExt;
use tokio::sync::mpsc;
use bollard::{
    Docker,
    config::{ContainerSummary, ContainerStatsResponse},
    query_parameters::{ListContainersOptionsBuilder, StatsOptionsBuilder}
};
use ratatui::crossterm::event;
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::TableState;
use app::App;

mod ui;
mod app;

pub enum ContainerState {
    Running,
    Paused,
    Exited
}

pub struct ContainerData {
    pub name: String,
    pub id: String,
    pub state: ContainerState,
    pub status: String,
    pub image: String
}

enum AppEvent {
    Tick,
    Key(event::KeyEvent),
    ContainerLoad(Vec<ContainerData>),
    DockerError(String),
}

fn transform_to_container_data(container: ContainerSummary, _stats: Option<ContainerStatsResponse>) -> ContainerData {
    let name = container.names.unwrap_or_default()[0].clone();
    let id = container.id.unwrap_or_default();
    let status = container.status.unwrap_or_default();
    let image = container.image.unwrap_or_default();

    let state = {
        if status.contains("Up") {
            if status.contains("Paused") {
                ContainerState::Paused
            } else {
                ContainerState::Running
            }
        } else {
            ContainerState::Exited
        }
    };

    ContainerData {
        name,
        id,
        state,
        status,
        image
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
                    if key.kind == event::KeyEventKind::Release {
                        continue;
                    }
                    let _ = tx_key.send(AppEvent::Key(key)).await;
                }
            }

            let _ = tx_key.send(AppEvent::Tick).await;
            let _ =  tokio::time::sleep(Duration::from_millis(250));
        }
    });

    let tx_docker = tx.clone();
    tokio::spawn(async move {
        let docker = Docker::connect_with_local_defaults().unwrap();

        loop {
            let list_config = ListContainersOptionsBuilder::new()
                .all(true)
                .build();

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

                            let mut stats_stream = docker_clone.stats(&container_id, Some(stats_options));
                            let stats = stats_stream.next().await.and_then(|res| res.ok());

                            transform_to_container_data(container, stats)
                        }
                    });

                    let payload = futures::future::join_all(stat_futures).await;
                    let _ = tx_docker.send(AppEvent::ContainerLoad(payload)).await;
                },
                Err(err) => {
                    let _ = tx_docker.send(AppEvent::DockerError(err.to_string())).await;
                }
            }
            let _ = tokio::time::sleep(Duration::from_millis(250)).await;
        }
    });

    let mut terminal = ratatui::init();
    let mut app = App::new();
    let mut table_state = TableState::default();
    table_state.select_first();
    table_state.select_first_column();

    loop {
        if let Some(event) = rx.recv().await {
            match event {
                AppEvent::ContainerLoad(containers) => {
                    app.containers = containers;
                },
                AppEvent::Key(key) => {
                    match key.code {
                        KeyCode::Char('q') => {
                            ratatui::restore();
                            std::process::exit(0);
                        },
                        KeyCode::Char('j') => {
                            table_state.select_next();
                        },
                        KeyCode::Char('k') => {
                            table_state.select_previous();
                        },
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        let _ = terminal.draw(|frame| ui::render(frame, &app, &mut table_state));
    }
}
