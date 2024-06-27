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
            "available_memory": system.available_memory(),
            "memory_utilization": memory_utilization,
            "cpu_usage_percentage": cpu_usage,
            "disk_data": d_stats,
        });

        logs.record(
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
        assert!(attributes["total_memory"].is_number());
        assert!(attributes["used_memory"].is_number());
        assert!(attributes["available_memory"].is_number());
        assert!(attributes["memory_utilization"].is_number());
        assert!(attributes["cpu_usage_percentage"].is_number());
        assert!(attributes["disk_data"].is_array());
    }
}
