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
