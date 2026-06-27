use crate::AppEvent;
use bollard::Docker;
use bollard::{container::LogOutput, query_parameters::LogsOptionsBuilder};
use futures::StreamExt;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

pub enum DockerCMD {
    SwitchLogTarget(String),
}

pub struct DockerHandle {
    sender: Sender<DockerCMD>,
}

pub struct DockerWorker {
    receiver: Receiver<DockerCMD>,
    sender: Sender<AppEvent>,
    docker: Docker,
}

impl DockerHandle {
    pub fn send_cmd(&self, cmd: DockerCMD) {
        let _ = self.sender.try_send(cmd);
    }
}

impl DockerHandle {
    pub fn new(sender: Sender<DockerCMD>) -> Self {
        Self { sender }
    }
}

impl DockerWorker {
    pub fn spawn(receiver: Receiver<DockerCMD>, sender: Sender<AppEvent>, docker: Docker) {
        tokio::spawn(async move {
            let mut worker = Self {
                receiver,
                sender,
                docker,
            };
            worker.run().await;
        });
    }

    async fn run(&mut self) {
        let mut log_handle: Option<JoinHandle<()>> = None;
        while let Some(cmd) = self.receiver.recv().await {
            match cmd {
                DockerCMD::SwitchLogTarget(container_id) => {
                    if let Some(handle) = log_handle.take() {
                        handle.abort();
                    }

                    let log_options = LogsOptionsBuilder::new()
                        .follow(true)
                        .tail("50")
                        .stdout(true)
                        .build();

                    let mut log_stream = self.docker.logs(&container_id, Some(log_options));
                    let tx_log_clone = self.sender.clone();
                    let handle = tokio::spawn(async move {
                        while let Some(Ok(log_output)) = log_stream.next().await {
                            match log_output {
                                LogOutput::StdOut { message: bytes } => {
                                    let line = String::from_utf8_lossy(&bytes).to_string();
                                    let _ = tx_log_clone.send(AppEvent::NewLogLine(line)).await;
                                }
                                LogOutput::StdErr { message: bytes } => {
                                    let line = String::from_utf8_lossy(&bytes).to_string();
                                    let _ = tx_log_clone.send(AppEvent::NewLogLine(line)).await;
                                }
                                _ => {}
                            }
                        }
                    });
                    log_handle = Some(handle)
                }
            }
        }
    }
}
