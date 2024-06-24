use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration, time::Instant};
use sysinfo::{Disks, Pid, System};

pub const DEFAULT_CONFIG_PATH: &str = ".config/tracer/tracer.toml";

#[derive(Deserialize)]
pub struct ConfigFile {
    pub api_key: String,
    pub polling_interval_ms: u64,
    pub targets: Vec<String>,
}

struct Proc {
    name: String,
    start_time: DateTime<Utc>,
}

pub struct TracerClient {
    api_key: String,
    targets: Vec<String>,
    seen: HashMap<Pid, Proc>,
    system: System,
    service_url: String,
    last_sent: Instant,
    interval: Duration,
}

#[derive(Debug)]
pub enum EventStatus {
    FinishedRun,
    ToolExecution,
    MetricEvent,
    // NewRun,
    // RunStatusMessage,
}

impl EventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::FinishedRun => "finished_run",
            EventStatus::ToolExecution => "tool_execution",
            EventStatus::MetricEvent => "metric_event",
            // EventStatus::NewRun => "new_run",
            // EventStatus::RunStatusMessage => "run_status_message",
            // EventStatus::InstallationFinished => "installation_finished",
        }
    }
}

impl TracerClient {
    pub fn from_config(config: ConfigFile) -> Result<Self> {
        let service_url = std::env::var("TRACER_SERVICE_URL")
            .unwrap_or_else(|_| "https://app.tracer.bio/api/data-collector-api".to_string());

        Ok(Self {
            api_key: config.api_key,
            targets: config.targets,
            seen: HashMap::new(),
            system: System::new_all(),
            last_sent: Instant::now(),
            interval: Duration::from_millis(config.polling_interval_ms),
            service_url,
        })
    }

    pub async fn send_event(
        &self,
        process_status: EventStatus,
        message: &str,
        attributes: Option<Value>,
    ) -> Result<()> {
        let mut data = json!({
            "logs": [{
                "message": message,
                "event_type": "process_status",
                "process_type": "pipeline",
                "process_status": process_status.as_str(),
                "api_key": self.api_key,
                "attributes": attributes // Add attributes if provided
            }]
        });

        // logs the message to a file located at `/tmp/tracerd.out`
        println!("msg: {}", data);

        if let Some(props) = attributes {
            data["logs"][0]["attributes"] = props;
        }

        let res = Client::new()
            .post(&self.service_url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await;

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Error while sending metrics: {}", e);
                Ok(())
            }
        }
    }

    pub async fn poll_processes(&mut self) -> Result<()> {
        for (pid, proc) in self.system.processes().iter() {
            if !self.seen.contains_key(pid) && self.targets.contains(&proc.name().to_string()) {
                self.seen.insert(
                    *pid,
                    Proc {
                        name: proc.name().to_string(),
                        start_time: Utc::now(),
                    },
                );

                let Some(p) = self.system.process(*pid) else {
                    eprintln!("[{}] Process({}) wasn't found", Utc::now(), proc.name());
                    continue;
                };

                let start_time = Utc::now();
                let properties = json!({
                    "tool_name": proc.name(),
                    "tool_pid": pid.to_string(),
                    "tool_binary_path": p.exe(),
                    "start_timestamp": start_time.to_string(),
                });

                self.send_event(
                    EventStatus::ToolExecution,
                    &format!("[{}] Tool process: {}", start_time, proc.name()),
                    Some(properties),
                )
                .await?;
            }
        }

        Ok(())
    }

    pub async fn remove_completed_processes(&mut self) -> Result<()> {
        let mut to_remove = vec![];
        for (pid, proc) in self.seen.iter() {
            if !self.system.processes().contains_key(pid) {
                let duration = (Utc::now() - proc.start_time).to_std()?.as_millis();
                let attributes = json!({
                    "execution_duration": duration,
                });

                self.send_event(
                    EventStatus::FinishedRun,
                    &format!("[{}] {} exited", Utc::now(), &proc.name),
                    Some(attributes),
                )
                .await?;

                to_remove.push(*pid);
            }
        }
        // cleanup exited processes
        for i in to_remove.iter() {
            self.seen.remove(i);
        }

        Ok(())
    }

    pub async fn send_metrics(&mut self) -> Result<()> {
        if Instant::now() - self.last_sent >= self.interval {
            self.send_global_stat().await?;

            // TODO: commented until backend would be able to handle it
            // self.send_proc_stat().await?;

            self.last_sent = Instant::now();
        }

        Ok(())
    }

    // Sends current load of a system to the server
    async fn send_global_stat(&self) -> Result<()> {
        let used_memory = self.system.used_memory();
        let total_memory = self.system.total_memory();
        let memory_utilization = (used_memory as f64 / total_memory as f64) * 100.0;

        let cpu_usage = self.system.global_cpu_info().cpu_usage();

        let disks = Disks::new_with_refreshed_list();

        let mut d_stats = vec![];

        for d in disks.iter() {
            let Some(d_name) = d.name().to_str() else {
                continue;
            };

            let total_space = d.total_space();
            let available_space = d.available_space();
            let used_space = total_space - available_space;
            let disk_utilization = (used_space as f64 / total_space as f64) * 100.0;

            let disk_data = json!({
                d_name: {
                  "disk_total_space": total_space,
                  "disk_used_space": used_space,
                  "disk_available_space": available_space,
                  "disk_utilization": disk_utilization,
                },
            });

            d_stats.push(disk_data);
        }

        let attributes = json!({
            "events_name": "global_system_metrics",
            "total_memory": total_memory,
            "used_memory": used_memory,
            "available_memory": self.system.available_memory(),
            "memory_utilization": memory_utilization,
            "cpu_usage_percentage": cpu_usage,
            "disk_data": d_stats,
        });

        self.send_event(
            EventStatus::MetricEvent,
            &format!("[{}] System's resources metric", Utc::now()),
            Some(attributes),
        )
        .await?;

        Ok(())
    }

    // Sends current resource consumption of target processes to the server
    // async fn send_proc_stat(&self) -> Result<()> {
    //     for (pid, proc) in self.seen.iter() {
    //         let Some(p) = self.system.process(*pid) else {
    //             eprintln!("[{}] Process({}) wasn't found", Utc::now(), proc);
    //             return Ok(());
    //         };

    //         let attributes = json!({
    //             "name": format!("{} metric", proc),
    //             "memory_usage": p.memory(),
    //             "cpu_usage": p.cpu_usage(),
    //         });
    //         self.send_event(
    //             EventStatus::MetricEvent,
    //             &format!("[{}] {}({}) resources metric", Utc::now(), proc, pid),
    //             Some(attributes),
    //         )
    //         .await?;
    //     }
    //     Ok(())
    // }

    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }
}

// TODO: uncomment
#[cfg(test)]
mod test {
    use super::*;

    fn create_conf() -> ConfigFile {
        toml::from_str(
            &std::fs::read_to_string(
                std::env::var("TRACER_CONFIG").unwrap_or("tracer.toml".to_string()),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn from_config() {
        let tr = TracerClient::from_config(create_conf());
        assert!(tr.is_ok())
    }

    #[tokio::test]
    async fn tool_exec() {
        let mut tr = TracerClient::from_config(create_conf()).unwrap();
        tr.targets = vec!["sleep".to_string()];

        let mut cmd = std::process::Command::new("sleep")
            .arg("1")
            .spawn()
            .unwrap();

        while tr.seen.len() <= 0 {
            tr.refresh();
            tr.poll_processes().await.unwrap();
        }

        cmd.wait().unwrap();

        assert!(tr.seen.len() > 0)
    }

    #[tokio::test]
    async fn tool_finish() {
        let mut tr = TracerClient::from_config(create_conf()).unwrap();
        tr.targets = vec!["sleep".to_string()];

        let mut cmd = std::process::Command::new("sleep")
            .arg("1")
            .spawn()
            .unwrap();

        while tr.seen.len() <= 0 {
            tr.refresh();
            tr.poll_processes().await.unwrap();
        }

        cmd.wait().unwrap();
        tr.refresh();

        tr.remove_completed_processes().await.unwrap();

        assert!(tr.seen.len() == 0)
    }
}
