/// src/events/mod.rs
use anyhow::Result;
use serde_json::json;

use crate::TracerAppConfig;

use anyhow::Result;
use serde_json::{json, Value};

use crate::TracerAppConfig;

use super::utils::handle_response;

#[derive(Debug)]
pub enum EventStatus {
    NewRun,
    FinishedRun,
    RunStatusMessage,
    ToolExecution,
    InstallationFinished,
    MetricEvent,
}

impl EventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::NewRun => "new_run",
            EventStatus::FinishedRun => "finished_run",
            EventStatus::RunStatusMessage => "run_status_message",
            EventStatus::ToolExecution => "tool_execution",
            EventStatus::InstallationFinished => "installation_finished",
            EventStatus::MetricEvent => "metric_event",
        }
    }
}

pub struct Tool {
    pub name: String,
    pub version: String,
}

pub async fn event_pipeline_run_start_new() -> Result<()> {
    println!("Starting new pipeline...");
    let config = TracerAppConfig::load_config()?;

    event_metrics().await?;
    event_pipeline_new_run(&config, "[CLI] Starting pipeline run").await?;
    println!("Started pipeline run successfully...");

    Ok(())
}

pub async fn event_pipeline_run_end() -> Result<()> {
    println!("Ending tracer session...");
    let config = TracerAppConfig::load_config()?;

    event_metrics().await?;
    event_pipeline_finish_run(&config).await?;
    Ok(())
}

async fn event_pipeline_new_run(config: &TracerAppConfig, msg: &str) -> Result<()> {
    send_event(
        config,
        EventStatus::NewRun.as_str(),
        &format!("Initialized pipeline run with name: {}", msg),
        None,
        false,
    )
    .await
}

async fn event_tool_process(config: &TracerAppConfig, tool: &Tool) -> Result<()> {
    let properties = json!({
        "tool_version": &tool.version,
        "tool_name": &tool.name,
    });

    send_event(
        config,
        EventStatus::ToolExecution.as_str(),
        &format!("Tool process: {}", &tool.name),
        Some(properties),
        false,
    )
    .await
}

async fn event_log_message(config: &TracerAppConfig, message: &str) -> Result<()> {
    send_event(
        config,
        EventStatus::RunStatusMessage.as_str(),
        message,
        None,
        false,
    )
    .await
}

async fn event_pipeline_finish_run(config: &TracerAppConfig) -> Result<()> {
    send_event(
        config,
        EventStatus::FinishedRun.as_str(),
        "Pipeline run concluded successfully",
        None,
        false,
    )
    .await
}

async fn event_metrics() -> Result<()> {
    // Assuming there is some implementation for metrics collection
    Ok(())
}
