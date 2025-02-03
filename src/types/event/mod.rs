pub mod attributes;
pub mod aws_metadata;

use attributes::EventAttributes;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Event {
    #[serde(with = "ts_seconds")]
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub event_type: String,
    pub process_type: String,
    pub process_status: String,
    pub pipeline_name: Option<String>,
    pub run_name: Option<String>,
    pub run_id: Option<String>,
    pub attributes: Option<EventAttributes>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct OtelJsonEvent {
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub event_type: String,
    pub process_type: String,
    pub process_status: String,
    pub run_name: Option<String>,
    pub run_id: Option<String>,
    pub attributes: Option<EventAttributes>,
}

impl From<Event> for OtelJsonEvent {
    fn from(value: Event) -> Self {
        OtelJsonEvent {
            timestamp: value.timestamp,
            message: value.message,
            event_type: value.event_type,
            process_type: value.process_type,
            process_status: value.process_status,
            run_name: value.run_name,
            run_id: value.run_id,
            attributes: value.attributes,
        }
    }
}
