use std::sync::Arc;

use super::system_metrics::SystemMetric;
use arrow::datatypes::{DataType, Field, Schema};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SyslogProperties {
    pub system_metrics: SystemMetric,
    pub error_display_name: String,
    pub error_id: String,
    pub error_line: String,
    pub file_line_number: usize,
    pub file_previous_logs: Vec<String>,
}

impl SyslogProperties {
    pub fn schema() -> Schema {
        let system_metrics_dt = DataType::Struct(SystemMetric::schema().fields);
        let fields = vec![
            Field::new("system_metrics", system_metrics_dt, false),
            Field::new("error_display_name", DataType::Utf8, false),
            Field::new("error_id", DataType::Utf8, false),
            Field::new("error_line", DataType::Utf8, false),
            // TODO: find usize repr
            Field::new("file_line_number", DataType::UInt64, false),
            Field::new(
                "file_previous_logs",
                DataType::List(Arc::new(Field::new("log", DataType::Utf8, false))),
                false,
            ),
        ];
        Schema::new(fields)
    }
}
