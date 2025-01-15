use std::{collections::HashMap, sync::Arc};

use arrow::datatypes::{DataType, Field, Fields, Schema};

use crate::types::{event::aws_metadata::AwsInstanceMetaData, ParquetSchema};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DiskStatistic {
    pub disk_total_space: u64,
    pub disk_used_space: u64,
    pub disk_available_space: u64,
    pub disk_utilization: f64,
}

impl ParquetSchema for DiskStatistic {
    fn schema() -> Schema {
        let fields = vec![
            Field::new("disk_total_space", DataType::UInt64, false),
            Field::new("disk_used_space", DataType::UInt64, false),
            Field::new("disk_available_space", DataType::UInt64, false),
            Field::new("disk_utilization", DataType::Float64, false),
        ];
        Schema::new(fields)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SystemMetric {
    pub events_name: String,
    pub system_memory_total: u64,
    pub system_memory_used: u64,
    pub system_memory_available: u64,
    pub system_memory_utilization: f64,
    pub system_memory_swap_total: u64,
    pub system_memory_swap_used: u64,
    pub system_cpu_utilization: f32,
    pub system_disk_io: HashMap<String, DiskStatistic>,
}

impl ParquetSchema for SystemMetric {
    fn schema() -> Schema {
        let disk_stat_data_type = DataType::Struct(DiskStatistic::schema().fields);

        let mapped = DataType::Struct(Fields::from(vec![
            Field::new("key", DataType::Utf8, false),
            Field::new("value", disk_stat_data_type, false),
        ]));

        let fields = vec![
            Field::new("events_name", DataType::Utf8, false),
            Field::new("system_memory_total", DataType::UInt64, false),
            Field::new("system_memory_used", DataType::UInt64, false),
            Field::new("system_memory_available", DataType::UInt64, false),
            Field::new("system_memory_utilization", DataType::Float64, false),
            Field::new("system_memory_swap_total", DataType::UInt64, false),
            Field::new("system_memory_swap_used", DataType::UInt64, false),
            Field::new("system_cpu_utilization", DataType::Float32, false),
            Field::new(
                "system_disk_io",
                DataType::Map(Arc::new(Field::new("entries", mapped, false)), false),
                false,
            ),
        ];
        Schema::new(fields)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemProperties {
    pub os: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub arch: Option<String>,
    pub num_cpus: usize,
    pub hostname: Option<String>,
    pub total_memory: u64,
    pub total_swap: u64,
    pub uptime: u64,
    pub aws_metadata: Option<AwsInstanceMetaData>,
    pub is_aws_instance: bool,
    pub system_disk_io: HashMap<String, DiskStatistic>,
}

impl ParquetSchema for SystemProperties {
    fn schema() -> Schema {
        let disk_stat_data_type = DataType::Struct(DiskStatistic::schema().fields);
        let aws_metadata = DataType::Struct(AwsInstanceMetaData::schema().fields);

        let mapped = DataType::Struct(Fields::from(vec![
            Field::new("key", DataType::Utf8, false),
            Field::new("value", disk_stat_data_type, false),
        ]));

        let fields = vec![
            Field::new("os", DataType::Utf8, true),
            Field::new("os_version", DataType::Utf8, true),
            Field::new("kernel_version", DataType::Utf8, true),
            Field::new("arch", DataType::Utf8, true),
            Field::new("num_cpus", DataType::UInt64, false),
            Field::new("hostname", DataType::Utf8, true),
            Field::new("total_memory", DataType::UInt64, false),
            Field::new("total_swap", DataType::UInt64, false),
            Field::new("uptime", DataType::UInt64, false),
            Field::new("aws_metadata", aws_metadata, true),
            Field::new("is_aws_instance", DataType::Boolean, false),
            Field::new(
                "system_disk_io",
                DataType::Map(Arc::new(Field::new("entries", mapped, false)), false),
                false,
            ),
        ];
        Schema::new(fields)
    }
}
