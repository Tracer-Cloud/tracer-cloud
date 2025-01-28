use super::event::attributes::system_metrics::SystemProperties;
use super::event::OtelJsonEvent;
use super::ParquetSchema;
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};

use super::event::{
    attributes::{
        process::{CompletedProcess, ProcessProperties},
        syslog::SyslogProperties,
        system_metrics::SystemMetric,
        EventAttributes,
    },
    Event,
};

///
/// This struct would serve as an intermediary between the events types tracer exports
///
/// The Reason for this is because by default, arrow doesn't support enumerated types
/// A straight forward approach would be to have a somewhat flat schema instead of using union
/// types which introduces complexity when it comes to querying or support as only dedicated
/// parquet engines fully support union types

#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct FlattenedTracerEvent {
    #[serde(with = "ts_seconds")]
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub event_type: String,
    pub process_type: String,
    pub process_status: String,

    pub run_name: Option<String>,
    pub run_id: Option<String>,

    pub system_properties: Option<SystemProperties>,

    pub process_attributes: Option<ProcessProperties>,
    pub system_metric_attributes: Option<SystemMetric>,
    pub completed_process_attributes: Option<CompletedProcess>,
    pub syslog_attributes: Option<SyslogProperties>,

    pub json_event: String,
}

impl From<Event> for FlattenedTracerEvent {
    fn from(value: Event) -> Self {
        let otel_event: OtelJsonEvent = value.clone().into();
        let json_event =
            serde_json::to_string_pretty(&otel_event).expect("Failed to create event str");
        let mut tracer_event = Self {
            timestamp: value.timestamp,
            message: value.message,
            event_type: value.event_type,
            process_type: value.process_type,
            process_status: value.process_status,
            run_name: value.run_name,
            run_id: value.run_id,
            json_event,
            ..Default::default()
        };

        if let Some(attributes) = value.attributes {
            match attributes {
                EventAttributes::Process(inner) => tracer_event.process_attributes = Some(inner),
                EventAttributes::CompletedProcess(inner) => {
                    tracer_event.completed_process_attributes = Some(inner)
                }
                EventAttributes::SystemMetric(inner) => {
                    tracer_event.system_metric_attributes = Some(inner)
                }
                EventAttributes::Syslog(inner) => tracer_event.syslog_attributes = Some(inner),
                EventAttributes::SystemProperties(inner) => {
                    tracer_event.system_properties = Some(inner)
                }
                // would take out the other or handle it by converting it into a str
                EventAttributes::Other(_inner) => (),
            }
        }
        tracer_event
    }
}

impl ParquetSchema for FlattenedTracerEvent {
    fn schema() -> arrow::datatypes::Schema {
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
            Field::new("run_name", DataType::Utf8, true),
            Field::new("run_id", DataType::Utf8, true),
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
            Field::new("json_event", DataType::Utf8, false),
        ];
        Schema::new(fields)
    }
}
