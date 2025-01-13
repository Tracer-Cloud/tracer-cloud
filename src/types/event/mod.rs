pub mod attributes;

use attributes::process::{CompletedProcess, ProcessProperties};
use attributes::syslog::SyslogProperties;
use attributes::system_metrics::SystemMetric;
use attributes::EventAttributes;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};

use arrow::datatypes::{DataType, Field, Schema, TimeUnit};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Event {
    #[serde(with = "ts_seconds")]
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub event_type: String,
    pub process_type: String,
    pub process_status: String,
    pub attributes: Option<EventAttributes>,
}

impl Event {
    // two approaches for is the following:
    // Use a union field to represent all possible variants, but this inturn increases the query
    // complexity
    // Second approach is to have nullable struct fields for all variants, while this maintains a
    // somewhat flat structure, we'd have nullable entries on each columns
    pub fn schema() -> Schema {
        // get all variant structs
        let process_dt = ProcessProperties::schema().fields;
        let completed_process_dt = CompletedProcess::schema().fields;
        let system_metrics_dt = SystemMetric::schema().fields;
        let syslog_dt = SyslogProperties::schema().fields;
        let fields = vec![
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Second, None),
                false,
            ),
            Field::new("message", DataType::Utf8, false),
            Field::new("event_type", DataType::Utf8, false),
            Field::new("process_type", DataType::Utf8, false),
            Field::new("process_status", DataType::Utf8, false),
            Field::new("process_status", DataType::Utf8, false),
            Field::new("process_attributes", DataType::Struct(process_dt), true),
            Field::new(
                "completed_process_attributes",
                DataType::Struct(completed_process_dt),
                true,
            ),
            Field::new(
                "system_metric_attributes",
                DataType::Struct(system_metrics_dt),
                true,
            ),
            Field::new("syslog_attributes", DataType::Struct(syslog_dt), true),
        ];
        Schema::new(fields)
    }
}
