use std::collections::HashMap;
use std::path::PathBuf;
use std::str;
use async_nats::{Client, ConnectOptions};
use futures::stream::StreamExt;
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use pueue_lib::network::message::{AddMessage, Message};
use pueue_lib::settings::Settings;
use pueue_lib::state::SharedState;
use crate::daemon::network::message_handler::handle_message;


pub async fn receive_messages(state: SharedState,
                              settings: Settings) -> anyhow::Result<()> {
    let nats_host = if let Some(host) = &settings.daemon.nats_host {
        host.clone()
    } else {
        std::env::var("NATS_HOST").unwrap_or("localhost".to_owned())
    };
    let worker = PueuedWorker::new(&settings, "UP");
    let options = ConnectOptions::new().name("pueued-worker");
    let nc = async_nats::connect_with_options(&nats_host, options).await.unwrap();
    info!("Begin to receive messages from NATS: {}, inbox: {}", nats_host, worker.inbox());
    println!("Begin to receive messages from NATS: {}, inbox: {}", nats_host, worker.inbox());
    // subscribe demo subject
    let inbox_name = worker.inbox();
    let nc_demo_handler = nc.clone();
    let demo_handle = tokio::task::spawn({
        async move {
            let mut subscriber = nc_demo_handler.subscribe(inbox_name).await.unwrap();
            while let Some(msg) = subscriber.next().await {
                let payload = String::from_utf8(msg.payload.to_vec()).unwrap();
                let message = payload.trim();
                if message == "pong" {
                    info!("pueue-001200: Pueue worker registered successfully!");
                    println!("pueue-001200: Pueue worker registered successfully!");
                } else if message.starts_with("{") { // json message
                    if let Ok(origin_msg) = serde_json::from_str::<AddMessage>(&message) {
                        let group = if origin_msg.group.is_empty() {
                            "default".to_owned()
                        } else {
                            origin_msg.group.clone()
                        };
                        let add_msg = AddMessage {
                            command: adjust_command_path(&origin_msg.command),
                            path: PathBuf::from("/tmp"),
                            envs: origin_msg.envs,
                            start_immediately: origin_msg.start_immediately,
                            stashed: origin_msg.stashed,
                            group,
                            enqueue_at: origin_msg.enqueue_at,
                            dependencies: origin_msg.dependencies,
                            priority: origin_msg.priority,
                            label: origin_msg.label,
                            print_task_id: false,
                        };
                        let _ = handle_message(Message::Add(add_msg), &state, &settings);
                    } else {
                        error!("pueue-001201: Invalid message format: {}", message);
                    }
                } else { // command line only
                    let add_msg = AddMessage {
                        command: adjust_command_path(&message),
                        path: PathBuf::from("/tmp"),
                        envs: Default::default(),
                        start_immediately: false,
                        stashed: false,
                        group: "default".to_owned(),
                        enqueue_at: None,
                        dependencies: vec![],
                        priority: None,
                        label: None,
                        print_task_id: false,
                    };
                    let _ = handle_message(Message::Add(add_msg), &state, &settings);
                }
            }
            Ok::<(), async_nats::Error>(())
        }
    });
    // register pueued work
    register_worker(&nc, &worker).await;
    // run both publishing tasks in parallel and gather the results.
    match futures::try_join!(demo_handle) {
        Ok((_demo_duration, )) => info!("Finished to subscribe subjects: demo"),
        Err(err) => error!("Error to talk with NATS: {:?} ", err.to_string()),
    }
    Ok(())
}

#[derive(PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct PueuedWorker {
    pub id: String,
    pub host_name: String,
    pub app: String,
    pub ip_addr: String,
    pub vip_address: Option<String>,
    pub secure_vip_address: Option<String>,
    pub status: String,
    pub port: Option<u32>,
    pub secure_port: Option<u32>,
    pub home_page_url: Option<String>,
    pub status_page_url: Option<String>,
    pub health_check_url: Option<String>,
    pub data_center_info: Option<DataCenterInfo>,
    pub metadata: HashMap<String, String>,
}

#[derive(PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct DataCenterInfo {
    pub name: String,
    pub metadata: HashMap<String, String>,
}

impl PueuedWorker {
    pub fn new(settings: &Settings, status: &str) -> Self {
        let worker_id = if let Some(id) = &settings.daemon.worker_id {
            id.to_string()
        } else {
            std::env::var("WORKER_ID").unwrap_or(uuid::Uuid::new_v4().to_string())
        };
        let worker_ip_addr = if let Ok(ip) = local_ip_address::local_ip() {
            ip.to_string()
        } else {
            "127.0.0.1".to_owned()
        };
        let mut metadata: HashMap<String, String> = HashMap::new();
        metadata.insert("uuid".to_owned(), worker_id.to_string());
        metadata.insert("inbox".to_owned(), format!("pueued.worker.{}", worker_id));
        PueuedWorker {
            id: worker_id.to_string(),
            host_name: worker_ip_addr.to_string(),
            app: "pueued-worker".to_string(),
            ip_addr: worker_ip_addr.to_string(),
            vip_address: None,
            secure_vip_address: None,
            status: status.to_string(),
            port: None,
            secure_port: None,
            home_page_url: None,
            status_page_url: None,
            health_check_url: None,
            data_center_info: None,
            metadata,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn inbox(&self) -> String {
        format!("pueued.worker.{}", self.id)
    }
}

fn adjust_command_path(command_line: &str) -> String {
    return if command_line.contains("/") {
        command_line.to_owned()
    } else {
        let offset = command_line.find(" ").unwrap_or(0);
        let command = if offset > 0 {
            &command_line[..offset]
        } else {
            command_line
        };
        if let Ok(path) = which::which(command) {
            format!("{}{}", path.to_string_lossy(), &command_line[offset..])
        } else {
            command_line.to_owned()
        }
    };
}

async fn register_worker(nc: &Client, worker: &PueuedWorker) {
    nc.publish_with_reply("pueued.registry", worker.inbox(), worker.to_json().into()).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjust_command_path() {
        let command_line = "java --version";
        println!("command_line: {}", adjust_command_path(command_line));
    }

    #[test]
    fn parse_add_msg() {
        let json_text = r#"
{
  "command": "java --version",
  "path": "/tmp",
  "envs": {},
  "start_immediately": false,
  "stashed": false,
  "group": "default",
  "enqueue_at": null,
  "dependencies": [],
  "label": "task-xxxxx",
  "print_task_id": false
}
        "#;
        let add_msg = serde_json::from_str::<AddMessage>(json_text).unwrap();
        println!("{:?}", add_msg);
    }
}
