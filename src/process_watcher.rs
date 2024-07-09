use crate::config_manager::Target;
// src/process_watcher.rs
use crate::event_recorder::EventRecorder;
use crate::event_recorder::EventType;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;
use std::path::Path;
use sysinfo::{Pid, Process, System};

pub struct ProcessWatcher {
    targets: Vec<Target>,
    seen: HashMap<Pid, Proc>,
}

pub struct Proc {
    name: String,
    start_time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
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
pub struct ShortLivedProcessLog {
    pub command: String,
    pub timestamp: String,
    pub properties: ProcessProperties,
}

#[derive(Clone)]
pub struct ProcessTreeNode {
    pub properties: ProcessProperties,
    pub children: Vec<ProcessTreeNode>,
    pub parent_id: Option<Pid>,
}

impl ProcessWatcher {
    pub fn new(targets: Vec<Target>) -> Self {
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
            if !self.seen.contains_key(pid)
                && self.targets.iter().any(|target| {
                    !target.should_be_merged_with_parents()
                        && target.matches(proc.name(), &proc.cmd().join(" "))
                })
            {
                self.add_new_process(*pid, proc, system, event_logger)?;
            }
        }

        self.parse_process_tree(
            system,
            self.targets
                .iter()
                .filter(|target| target.should_be_merged_with_parents())
                .cloned()
                .collect(),
        )?;
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

    pub fn build_process_trees(&self, system: &System) -> HashMap<Pid, ProcessTreeNode> {
        let mut nodes: HashMap<Pid, ProcessTreeNode> = HashMap::new();

        for (pid, proc) in system.processes() {
            let properties = Self::gather_process_data(pid, proc);
            let node = ProcessTreeNode {
                properties,
                children: vec![],
                parent_id: proc.parent(),
            };

            nodes.insert(*pid, node);
        }

        for (pid, proc) in system.processes() {
            let parent = proc.parent();
            if let Some(parent) = parent {
                let node = nodes.get(pid).unwrap().clone();
                if let Some(parent_node) = nodes.get_mut(&parent) {
                    parent_node.children.push(node.clone());
                }
            }
        }

        nodes
    }

    pub fn get_parent_processes(
        &self,
        map: &HashMap<Pid, ProcessTreeNode>,
        valid_processes: &Vec<Pid>,
    ) -> Vec<Pid> {
        let mut result = vec![];

        for process in valid_processes {
            let mut parent = *process;
            let mut last_valid_parent = *process;

            while let Some(parent_node) = map.get(&parent) {
                parent = parent_node.parent_id.unwrap();
                if !valid_processes.contains(&parent) {
                    break;
                }
                last_valid_parent = parent;
            }

            if !result.contains(&last_valid_parent) {
                result.push(last_valid_parent);
            }
        }

        result
    }

    pub fn parse_process_tree(&mut self, system: &System, targets: Vec<Target>) -> Result<()> {
        let nodes: HashMap<Pid, ProcessTreeNode> = self.build_process_trees(system);

        let mut processes_to_gather = vec![];

        for target in targets {
            let mut valid_processes = vec![];

            for (pid, node) in &nodes {
                if target.matches(&node.properties.tool_name, &node.properties.tool_cmd) {
                    valid_processes.push(*pid);
                }
            }

            let parents = self.get_parent_processes(&nodes, &valid_processes);

            for parent in parents {
                if !processes_to_gather.contains(&parent) {
                    processes_to_gather.push(parent);
                }
            }
        }

        for pid in processes_to_gather {
            if !self.seen.contains_key(&pid) {
                let process = system.process(pid);
                if process.is_none() {
                    eprintln!("[{}] Process({}) wasn't found", Utc::now(), pid);
                    continue;
                }
                let proc = process.unwrap();
                self.add_new_process(pid, proc, system, &mut EventRecorder::new())?;
            }
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
                .unwrap_or_else(|| Path::new(""))
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

    pub fn fill_logs_with_short_lived_process(
        &mut self,
        short_lived_process: ShortLivedProcessLog,
        event_logger: &mut EventRecorder,
    ) -> Result<()> {
        let properties = json!(short_lived_process.properties);
        event_logger.record_event(
            EventType::ToolExecution,
            format!(
                "[{}] Short lived process: {}",
                short_lived_process.timestamp, short_lived_process.command
            ),
            Some(properties),
        );

        if let Vacant(v) = self
            .seen
            .entry(short_lived_process.properties.tool_pid.parse().unwrap())
        {
            v.insert(Proc {
                name: short_lived_process.command,
                start_time: Utc::now(),
            });
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

    pub fn reload_targets(&mut self, targets: Vec<Target>) {
        if targets == self.targets {
            return;
        }

        self.targets = targets;
        self.seen.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_parent_processes() {
        let dataset = vec![
            (1, 2),
            (2, 3),
            (2, 4),
            (1, 5),
            (4, 6),
            (4, 7),
            (5, 8),
            (1, 9),
            (1, 10),
        ];

        let mut nodes: HashMap<Pid, ProcessTreeNode> = HashMap::new();

        for (parent, child) in dataset {
            let properties = ProcessProperties {
                tool_name: "test".to_string(),
                tool_pid: child.to_string(),
                tool_binary_path: "test".to_string(),
                tool_cmd: "test".to_string(),
                start_timestamp: "test".to_string(),
                process_cpu_utilization: 0.0,
                process_memory_usage: 0,
                process_memory_virtual: 0,
            };

            let node = ProcessTreeNode {
                properties,
                children: vec![],
                parent_id: Some(parent.into()),
            };

            nodes.insert(child.into(), node);
        }

        let watcher = ProcessWatcher::new(vec![]);

        let result = watcher.get_parent_processes(
            &nodes,
            &vec![4.into(), 5.into(), 6.into(), 7.into(), 8.into()],
        );

        assert_eq!(result, vec![4.into(), 5.into()]);
    }
}
