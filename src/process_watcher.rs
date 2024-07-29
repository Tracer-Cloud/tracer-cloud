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
use std::time::Duration;
use sysinfo::{Pid, Process, System};

pub struct ProcessWatcher {
    targets: Vec<Target>,
    seen: HashMap<Pid, Proc>,
    process_tree: HashMap<Pid, ProcessTreeNode>,
}

pub struct Proc {
    name: String,
    start_time: DateTime<Utc>,
    last_update: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessProperties {
    pub tool_name: String,
    pub tool_pid: String,
    pub tool_binary_path: String,
    pub tool_cmd: String,
    pub start_timestamp: String,
    pub process_cpu_utilization: f32,
    pub process_memory_usage: u64,
    pub process_memory_virtual: u64,
    pub process_run_time: u64,
    pub process_disk_usage_read_last_interval: u64,
    pub process_disk_usage_write_last_interval: u64,
    pub process_disk_usage_read_total: u64,
    pub process_disk_usage_write_total: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ShortLivedProcessLog {
    pub command: String,
    pub timestamp: String,
    pub properties: ProcessProperties,
}

#[derive(Clone, Debug)]
pub struct ProcessTreeNode {
    pub properties: ProcessProperties,
    pub children: Vec<ProcessTreeNode>,
    pub parent_id: Option<Pid>,
    pub start_time: DateTime<Utc>,
}

impl ProcessWatcher {
    pub fn new(targets: Vec<Target>) -> Self {
        ProcessWatcher {
            targets,
            seen: HashMap::new(),
            process_tree: HashMap::new(),
        }
    }

    pub fn poll_processes(
        &mut self,
        system: &mut System,
        event_logger: &mut EventRecorder,
    ) -> Result<()> {
        for (pid, proc) in system.processes().iter() {
            if !self.seen.contains_key(pid) {
                let target = self
                    .targets
                    .iter()
                    .find(|target| target.matches(proc.name(), &proc.cmd().join(" ")));
                if let Some(target) = target {
                    self.add_new_process(*pid, proc, system, event_logger, Some(&target.clone()))?;
                }
            }
        }

        self.parse_process_tree(
            system,
            self.targets
                .iter()
                .filter(|target| target.should_be_merged_with_parents())
                .cloned()
                .collect(),
            event_logger,
        )?;

        Ok(())
    }

    pub fn poll_process_metrics(
        &mut self,
        system: &System,
        event_logger: &mut EventRecorder,
        process_metrics_send_interval: Duration,
    ) -> Result<()> {
        for (pid, proc) in system.processes().iter() {
            if let Some(p) = self.seen.get(pid) {
                if Utc::now() - process_metrics_send_interval > p.last_update {
                    self.add_process_metrics(proc, event_logger, None)?;
                    self.seen.get_mut(pid).unwrap().last_update = Utc::now();
                }
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
                self.log_completed_process(pid, proc, event_logger)?;
                to_remove.push(*pid);
            }
        }

        for pid in to_remove {
            self.seen.remove(&pid);
        }

        Ok(())
    }

    pub fn build_process_trees(&mut self, system_processes: &HashMap<Pid, Process>) {
        let mut nodes: HashMap<Pid, ProcessTreeNode> = HashMap::new();

        for (pid, proc) in system_processes {
            let properties = Self::gather_process_data(pid, proc, None);
            let node = ProcessTreeNode {
                properties,
                children: vec![],
                parent_id: proc.parent(),
                start_time: DateTime::from_timestamp(proc.start_time() as i64, 0).unwrap(),
            };

            nodes.insert(*pid, node);
        }

        for (pid, proc) in system_processes {
            let parent = proc.parent();
            if let Some(parent) = parent {
                let node = nodes.get(pid).unwrap().clone();
                if let Some(parent_node) = nodes.get_mut(&parent) {
                    parent_node.children.push(node.clone());
                }
            }
        }

        self.process_tree = nodes
    }

    pub fn get_parent_processes(
        &self,
        map: &HashMap<Pid, ProcessTreeNode>,
        valid_processes: &Vec<Pid>,
        force_ancestor_to_match: bool,
    ) -> Vec<Pid> {
        let mut result = vec![];

        for process in valid_processes {
            let mut parent = *process;
            let mut last_valid_parent = *process;

            while let Some(parent_node) = map.get(&parent) {
                parent = parent_node.parent_id.unwrap();
                if !valid_processes.contains(&parent) {
                    if !force_ancestor_to_match {
                        last_valid_parent = parent;
                    }
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

    pub fn parse_process_tree(
        &mut self,
        system: &System,
        targets: Vec<Target>,
        event_logger: &mut EventRecorder,
    ) -> Result<()> {
        self.build_process_trees(system.processes());
        let nodes: &HashMap<Pid, ProcessTreeNode> = &self.process_tree;

        let mut processes_to_gather = vec![];

        for target in &targets {
            let mut valid_processes = vec![];

            for (pid, node) in nodes {
                if target.matches(&node.properties.tool_name, &node.properties.tool_cmd) {
                    valid_processes.push(*pid);
                }
            }

            let parents = self.get_parent_processes(
                nodes,
                &valid_processes,
                target.should_force_ancestor_to_match(),
            );

            for parent in parents {
                if !processes_to_gather.contains(&(parent, target)) {
                    processes_to_gather.push((parent, target));
                }
            }
        }

        for (pid, target) in processes_to_gather {
            if !self.seen.contains_key(&pid) {
                let process = system.process(pid);
                if process.is_none() {
                    eprintln!("[{}] Process({}) wasn't found", Utc::now(), pid);
                    continue;
                }
                let proc = process.unwrap();
                self.add_new_process(pid, proc, system, event_logger, Some(target))?;
            }
        }
        Ok(())
    }

    pub fn gather_process_data(
        pid: &Pid,
        proc: &Process,
        display_name: Option<String>,
    ) -> ProcessProperties {
        let start_time = Utc::now();

        ProcessProperties {
            tool_name: display_name.unwrap_or(proc.name().to_owned()),
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
            process_run_time: proc.run_time(),
            process_disk_usage_read_total: proc.disk_usage().total_read_bytes,
            process_disk_usage_write_total: proc.disk_usage().total_written_bytes,
            process_disk_usage_read_last_interval: proc.disk_usage().read_bytes,
            process_disk_usage_write_last_interval: proc.disk_usage().written_bytes,
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
            None,
        );

        if let Vacant(v) = self
            .seen
            .entry(short_lived_process.properties.tool_pid.parse().unwrap())
        {
            v.insert(Proc {
                name: short_lived_process.command,
                start_time: Utc::now(),
                last_update: Utc::now(),
            });
        }

        Ok(())
    }

    pub fn gather_short_lived_process_data(system: &System, command: &str) -> ShortLivedProcessLog {
        let process = system.processes_by_name(command).last();
        if let Some(process) = process {
            ShortLivedProcessLog {
                command: command.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                properties: ProcessWatcher::gather_process_data(&process.pid(), process, None),
            }
        } else {
            ShortLivedProcessLog {
                command: command.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                properties: ProcessProperties {
                    tool_name: command.to_string(),
                    tool_pid: "".to_string(),
                    tool_binary_path: "".to_string(),
                    tool_cmd: command.to_string(),
                    start_timestamp: chrono::Utc::now().to_rfc3339(),
                    process_cpu_utilization: 0.0,
                    process_memory_usage: 0,
                    process_memory_virtual: 0,
                    process_run_time: 0,
                    process_disk_usage_read_last_interval: 0,
                    process_disk_usage_write_last_interval: 0,
                    process_disk_usage_read_total: 0,
                    process_disk_usage_write_total: 0,
                },
            }
        }
    }

    fn add_new_process(
        &mut self,
        pid: Pid,
        proc: &Process,
        system: &System,
        event_logger: &mut EventRecorder,
        target: Option<&Target>,
    ) -> Result<()> {
        self.seen.insert(
            pid,
            Proc {
                name: proc.name().to_string(),
                start_time: Utc::now(),
                last_update: Utc::now(),
            },
        );

        let Some(p) = system.process(pid) else {
            eprintln!("[{}] Process({}) wasn't found", Utc::now(), proc.name());
            return Ok(());
        };

        let start_time = Utc::now();

        let display_name = if let Some(target) = target {
            if let Some(display_name) = target.get_display_name() {
                display_name
            } else {
                proc.name().to_owned()
            }
        } else {
            proc.name().to_owned()
        };

        let properties = json!(Self::gather_process_data(
            &pid,
            p,
            Some(display_name.clone())
        ));

        event_logger.record_event(
            EventType::ToolExecution,
            format!("[{}] Tool process: {}", start_time, &display_name),
            Some(properties),
            None,
        );

        Ok(())
    }

    fn add_process_metrics(
        &mut self,
        proc: &Process,
        event_logger: &mut EventRecorder,
        target: Option<&Target>,
    ) -> Result<()> {
        let pid = proc.pid();
        let start_time = Utc::now();

        let display_name = if let Some(target) = target {
            if let Some(display_name) = target.get_display_name() {
                display_name
            } else {
                proc.name().to_owned()
            }
        } else {
            proc.name().to_owned()
        };

        let properties = json!(Self::gather_process_data(
            &pid,
            proc,
            Some(display_name.clone())
        ));

        event_logger.record_event(
            EventType::ToolMetricEvent,
            format!("[{}] Tool metric event: {}", start_time, &display_name),
            Some(properties),
            None,
        );

        Ok(())
    }

    pub fn get_earliest_process_time(&self) -> DateTime<Utc> {
        let mut earliest = Utc::now();

        for proc in self.seen.values() {
            if proc.start_time < earliest {
                earliest = proc.start_time;
            }
        }

        earliest
    }

    pub fn get_parent_pid(&self, run_start: Option<DateTime<Utc>>) -> Option<Pid> {
        let mut possible_parents = vec![];

        let parent = self
            .seen
            .iter()
            .find(|(_, proc)| run_start.is_none() || proc.start_time > run_start.unwrap())?;

        let mut pid = parent.0.to_owned();
        loop {
            let process = self.process_tree.get(&pid);

            if process.is_none() {
                break;
            }

            pid = process.unwrap().parent_id?;

            possible_parents.push(pid);
        }

        for process in self.seen.iter() {
            let mut pid = process.0.to_owned();
            loop {
                let process = self.process_tree.get(&pid);

                if process.is_none() {
                    break;
                }

                pid = process.unwrap().parent_id?;

                if possible_parents.contains(&pid) {
                    let index = possible_parents.iter().position(|&x| x == pid).unwrap();
                    if index > 0 {
                        possible_parents.drain(0..index - 1);
                    }
                    break;
                }
            }
        }

        possible_parents.retain(|x| {
            run_start.is_none() || self.process_tree[x].start_time > run_start.unwrap()
        });

        if possible_parents.is_empty() {
            None
        } else {
            Some(*possible_parents.last().unwrap())
        }
    }

    pub fn is_process_alive(&self, system: &System, pid: Pid) -> bool {
        system.process(pid).is_some()
    }

    fn log_completed_process(
        &self,
        pid: &Pid,
        proc: &Proc,
        event_logger: &mut EventRecorder,
    ) -> Result<()> {
        let duration = (Utc::now() - proc.start_time).to_std()?.as_millis();

        let properties = json!({
            "tool_name": proc.name.clone(),
            "tool_pid": pid.to_string(),
            "duration": duration
        });

        event_logger.record_event(
            EventType::FinishedToolExecution,
            format!("[{}] {} exited", Utc::now(), &proc.name),
            Some(properties),
            None,
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

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
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
                process_run_time: 0,
                process_disk_usage_read_last_interval: 0,
                process_disk_usage_write_last_interval: 0,
                process_disk_usage_read_total: 0,
                process_disk_usage_write_total: 0,
            };

            let node = ProcessTreeNode {
                properties,
                children: vec![],
                parent_id: Some(parent.into()),
                start_time: Utc::now(),
            };

            nodes.insert(child.into(), node);
        }

        let watcher = ProcessWatcher::new(vec![]);

        let result = watcher.get_parent_processes(
            &nodes,
            &vec![4.into(), 5.into(), 6.into(), 7.into(), 8.into()],
            true,
        );

        let result2 = watcher.get_parent_processes(
            &nodes,
            &vec![4.into(), 5.into(), 6.into(), 7.into(), 8.into()],
            false,
        );

        assert_eq!(result, vec![4.into(), 5.into()]);
        assert_eq!(result2, vec![2.into(), 1.into()]);
    }

    #[test]
    fn test_create_process_tree() -> Result<()> {
        let mut process_watcher = ProcessWatcher::new(vec![]);
        let system = System::new_all();

        process_watcher.build_process_trees(system.processes());

        Ok(())
    }
}
