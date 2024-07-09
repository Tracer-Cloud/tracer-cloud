use std::collections::HashMap;

/// src/system_metrics.rs
use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use sysinfo::{Disks, System};

use crate::event_recorder::{EventRecorder, EventType};

pub struct SystemMetricsCollector;

impl SystemMetricsCollector {
    pub fn new() -> Self {
        SystemMetricsCollector
    }

    pub fn collect_metrics(&self, system: &mut System, logs: &mut EventRecorder) -> Result<()> {
        let used_memory = system.used_memory();
        let total_memory = system.total_memory();
        let memory_utilization = (used_memory as f64 / total_memory as f64) * 100.0;

        let cpu_usage = system.global_cpu_info().cpu_usage();

        let disks: Disks = Disks::new_with_refreshed_list();

        let mut d_stats: HashMap<String, serde_json::Value> = HashMap::new();

        for d in disks.iter() {
            let Some(d_name) = d.name().to_str() else {
                continue;
            };

            let total_space = d.total_space();
            let available_space = d.available_space();
            let used_space = total_space - available_space;
            let disk_utilization = (used_space as f64 / total_space as f64) * 100.0;

            let disk_data = json!({
                  "disk_total_space": total_space,
                  "disk_used_space": used_space,
                  "disk_available_space": available_space,
                  "disk_utilization": disk_utilization,
            });

            d_stats.insert(d_name.to_string(), disk_data);
        }

        let attributes = json!({
            "events_name": "global_system_metrics",
            "system_memory_total": total_memory,
            "system_memory_used": used_memory,
            "system_memory_available": system.available_memory(),
            "system_memory_utilization": memory_utilization,
            "system_cpu_utilization": cpu_usage,
            "system_disk_io": d_stats,
        });

        logs.record_event(
            EventType::MetricEvent,
            format!("[{}] System's resources metric", Utc::now()),
            Some(attributes),
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_recorder::EventRecorder;

    #[test]
    fn test_collect_metrics() {
        let mut system = System::new_all();
        let mut logs = EventRecorder::new();
        let collector = SystemMetricsCollector::new();

        collector.collect_metrics(&mut system, &mut logs).unwrap();

        let events = logs.get_events();
        assert_eq!(events.len(), 1);

        let event = &events[0];

        assert!(event.attributes.is_some());

        let attributes = event.attributes.as_ref().unwrap();
        assert_eq!(attributes["events_name"], "global_system_metrics");
        assert!(attributes["system_memory_total"].is_number());
        assert!(attributes["system_memory_used"].is_number());
        assert!(attributes["system_memory_available"].is_number());
        assert!(attributes["system_memory_utilization"].is_number());
        assert!(attributes["system_cpu_utilization"].is_number());
        assert!(attributes["system_disk_io"].is_object());
    }
}
