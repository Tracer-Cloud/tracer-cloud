// src/process_watcher.rs
use crate::event_recorder::EventRecorder;
use crate::event_recorder::EventType;
use anyhow::Result;
use chrono::{DateTime, Utc};
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

        let properties = json!({
            "tool_name": proc.name(),
            "tool_pid": pid.to_string(),
            "tool_binary_path": p.exe(),
            "tool_cmd": p.cmd().join(" "),
            "start_timestamp": start_time.to_string(),
            "tool_cpu_usage": proc.cpu_usage(),
            "tool_memory_usage": proc.memory()
        });

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
}
