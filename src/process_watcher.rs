// src/process_watcher.rs
use crate::event_recorder::EventRecorder;
use crate::event_recorder::EventType;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use sysinfo::{Pid, Process, System};

pub struct ProcessWatcher {
    targets: Vec<String>,
    seen: HashMap<Pid, Proc>,
}

pub struct Proc {
    name: String,
    start_time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct ProcessProperties {
    pub tool_name: String,
    pub tool_pid: String,
    pub tool_binary_path: String,
    pub tool_cmd: String,
    pub start_timestamp: String,
    pub process_cpu_utilization: f32,
    pub process_memory_usage: u64,
    pub process_memory_virtual: u64,
}

#[derive(Serialize, Deserialize)]
pub struct QuickCommandLog {
    pub command: String,
    pub timestamp: String,
    pub properties: ProcessProperties,
}

impl ProcessWatcher {
    pub fn new(targets: Vec<String>) -> Self {
        ProcessWatcher {
            targets,
            seen: HashMap::new(),
        }
    }

    pub fn poll_processes(
        &mut self,
        system: &mut System,
        event_logger: &mut EventRecorder,
    ) -> Result<()> {
        for (pid, proc) in system.processes().iter() {
            if !self.seen.contains_key(pid) && self.targets.contains(&proc.name().to_string()) {
                self.add_new_process(*pid, proc, system, event_logger)?;
            }
        }
        Ok(())
    }

    pub fn remove_completed_processes(
        &mut self,
        system: &mut System,
        event_logger: &mut EventRecorder,
    ) -> Result<()> {
        let mut to_remove = vec![];
        for (pid, proc) in self.seen.iter() {
            if !system.processes().contains_key(pid) {
                self.log_completed_process(proc, event_logger)?;
                to_remove.push(*pid);
            }
        }

        for pid in to_remove {
            self.seen.remove(&pid);
        }

        Ok(())
    }

    pub fn gather_process_data(pid: &Pid, proc: &Process) -> ProcessProperties {
        let start_time = Utc::now();

        ProcessProperties {
            tool_name: proc.name().to_owned(),
            tool_pid: pid.to_string(),
            tool_binary_path: proc
                .exe()
                .unwrap()
                .as_os_str()
                .to_str()
                .unwrap()
                .to_string(),
            tool_cmd: proc.cmd().join(" "),
            start_timestamp: start_time.to_string(),
            process_cpu_utilization: proc.cpu_usage(),
            process_memory_usage: proc.memory(),
            process_memory_virtual: proc.virtual_memory(),
        }
    }

    pub fn fill_logs_with_quick_commands(
        &mut self,
        quick_commands: Vec<QuickCommandLog>,
        event_logger: &mut EventRecorder,
    ) -> Result<()> {
        for quick_command in quick_commands {
            let properties = json!(quick_command.properties);
            event_logger.record_event(
                EventType::ToolExecution,
                format!(
                    "[{}] Quick command: {}",
                    quick_command.timestamp, quick_command.command
                ),
                Some(properties),
            );

            if !self.seen.contains_key(&quick_command.properties.tool_pid.parse().unwrap()) {
                self.seen.insert(
                    quick_command.properties.tool_pid.parse().unwrap(),
                    Proc {
                        name: quick_command.command,
                        start_time: Utc::now(),
                    },
                );
            }
        }

        Ok(())
    }

    fn add_new_process(
        &mut self,
        pid: Pid,
        proc: &Process,
        system: &System,
        event_logger: &mut EventRecorder,
    ) -> Result<()> {
        self.seen.insert(
            pid,
            Proc {
                name: proc.name().to_string(),
                start_time: Utc::now(),
            },
        );

        let Some(p) = system.process(pid) else {
            eprintln!("[{}] Process({}) wasn't found", Utc::now(), proc.name());
            return Ok(());
        };

        let start_time = Utc::now();

        let properties = json!(Self::gather_process_data(&pid, p));

        event_logger.record_event(
            EventType::ToolExecution,
            format!("[{}] Tool process: {}", start_time, proc.name()),
            Some(properties),
        );

        Ok(())
    }

    fn log_completed_process(&self, proc: &Proc, event_logger: &mut EventRecorder) -> Result<()> {
        let duration = (Utc::now() - proc.start_time).to_std()?.as_millis();
        let properties = json!({
            "execution_duration": duration,
        });

        event_logger.record_event(
            EventType::FinishedRun,
            format!("[{}] {} exited", Utc::now(), &proc.name),
            Some(properties),
        );

        Ok(())
    }

    pub fn reload_targets(&mut self, targets: Vec<String>) {
        if targets == self.targets {
            return;
        }

        self.targets = targets;
        self.seen.clear();
    }
}
