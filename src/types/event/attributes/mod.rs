use process::{CompletedProcess, ProcessProperties, ProcessedDataSampleStats};
use syslog::SyslogProperties;
use system_metrics::{SystemMetric, SystemProperties};

pub mod process;
pub mod syslog;
pub mod system_metrics;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum EventAttributes {
    Process(ProcessProperties),
    CompletedProcess(CompletedProcess),
    SystemMetric(SystemMetric),
    Syslog(SyslogProperties),
    SystemProperties(SystemProperties),
    // TODO: take out when done with demo
    ProcessStatistics(ProcessedDataSampleStats),
    Other(serde_json::Value),
}
