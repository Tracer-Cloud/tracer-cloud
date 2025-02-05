use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};
use serde::{Deserialize, Serialize};

use crate::types::ParquetSchema;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InputFile {
    pub file_name: String,
    pub file_size: u64,
    pub file_path: String,
    pub file_directory: String,
    pub file_updated_at_timestamp: String,
}
impl ParquetSchema for InputFile {
    fn schema() -> Schema {
        let fields = vec![
            Field::new("file_name", DataType::Utf8, false),
            Field::new("file_size", DataType::UInt64, false),
            Field::new("file_path", DataType::Utf8, false),
            Field::new("file_directory", DataType::Utf8, false),
            Field::new("file_updated_at_timestamp", DataType::Utf8, false),
        ];
        Schema::new(fields)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessProperties {
    pub tool_name: String,
    pub tool_pid: String,
    pub tool_parent_pid: String,
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
    pub process_status: String,
    pub input_files: Option<Vec<InputFile>>,
}

impl ParquetSchema for ProcessProperties {
    fn schema() -> Schema {
        let input_file_dt = DataType::Struct(InputFile::schema().fields);
        let fields = vec![
            Field::new("tool_name", DataType::Utf8, false),
            Field::new("tool_pid", DataType::Utf8, false),
            Field::new("tool_parent_pid", DataType::Utf8, false),
            Field::new("tool_binary_path", DataType::Utf8, false),
            Field::new("tool_cmd", DataType::Utf8, false),
            Field::new("start_timestamp", DataType::Utf8, false),
            Field::new("process_cpu_utilization", DataType::Float32, false),
            Field::new("process_memory_usage", DataType::UInt64, false),
            Field::new("process_memory_virtual", DataType::UInt64, false),
            Field::new("process_run_time", DataType::UInt64, false),
            Field::new(
                "process_disk_usage_read_last_interval",
                DataType::UInt64,
                false,
            ),
            Field::new(
                "process_disk_usage_write_last_interval",
                DataType::UInt64,
                false,
            ),
            Field::new("process_disk_usage_read_total", DataType::UInt64, false),
            Field::new("process_disk_usage_write_total", DataType::UInt64, false),
            Field::new("process_status", DataType::Utf8, false),
            Field::new(
                "input_files",
                DataType::List(Arc::new(Field::new("file", input_file_dt, false))),
                true,
            ),
        ];
        Schema::new(fields)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedProcess {
    pub tool_name: String,
    pub tool_pid: String,
    pub duration_sec: u64,
}

impl ParquetSchema for CompletedProcess {
    fn schema() -> Schema {
        let fields = vec![
            Field::new("file_name", DataType::Utf8, false),
            Field::new("file_path", DataType::Utf8, false),
            Field::new("duration_sec", DataType::UInt64, false),
        ];
        Schema::new(fields)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSetsProcessed {
    pub datasets: String,
    pub count: u64,
}

impl ParquetSchema for DataSetsProcessed {
    fn schema() -> Schema {
        let fields = vec![
            Field::new("datasets", DataType::Utf8, false),
            Field::new("count", DataType::UInt64, false),
        ];
        Schema::new(fields)
    }
}
