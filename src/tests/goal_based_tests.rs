use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use crate::config_manager::target_process::target_matching::TargetMatch;
use crate::config_manager::target_process::DisplayName;
use crate::config_manager::target_process::Target;
use crate::events::recorder::EventRecorder;
use crate::exporters::FsExportHandler;
use crate::exporters::ParquetExport;
use crate::extracts::file_watcher::FileWatcher;
use crate::extracts::process_watcher::ProcessWatcher;
use crate::types::parquet::FlattenedTracerEvent;
use arrow::array::RecordBatch;
use arrow::json::ArrayWriter;
use duckdb::Connection;
use sysinfo::System;
use tempdir::TempDir;

fn setup_fs_exporter_and_path() -> (FsExportHandler, PathBuf) {
    let temp_dir = TempDir::new("exports").expect("failed to create tempdir");

    let export_dir = temp_dir.path().to_path_buf();

    (FsExportHandler::new(export_dir.clone(), None), export_dir)
}

async fn query(query: &str) -> Vec<FlattenedTracerEvent> {
    let conn = Connection::open_in_memory().expect("Failed to create duckdb connection");
    let mut stmt = conn.prepare(query).expect("Query failed");

    let records: Vec<RecordBatch> = stmt
        .query_arrow([])
        .expect("failed to convert to record batch")
        .collect();

    records
        .iter()
        .map(record_batch_to_structs)
        .flatten()
        .collect()
}

fn record_batch_to_structs(batch: &RecordBatch) -> Vec<FlattenedTracerEvent> {
    // Convert RecordBatch to JSON
    let buf = Vec::new();
    let mut writer = ArrayWriter::new(buf);
    writer.write_batches(&[batch]).unwrap();
    writer.finish().unwrap();

    let buf = writer.into_inner();
    let json_str = String::from_utf8(buf).unwrap();

    // Deserialize JSON into structs
    serde_json::from_str::<Vec<FlattenedTracerEvent>>(&json_str).unwrap()
}

async fn run_process_watcher(
    events_recorder: &mut EventRecorder,
    duration: Duration,
    targets: Vec<Target>,
) {
    let mut process_watcher = ProcessWatcher::new(targets);

    let mut system = System::new();

    let file_watcher = FileWatcher::new();

    let start_time = Instant::now();

    while start_time.elapsed() < duration {
        process_watcher
            .poll_processes(&mut system, events_recorder, &file_watcher)
            .expect("Failed to poll processes");
        process_watcher
            .poll_process_metrics(
                &system,
                events_recorder,
                std::time::Duration::from_millis(3),
            )
            .expect("failed to poll process metrices");
        tokio::time::sleep(Duration::from_secs(2)).await;
        system.refresh_all();
    }
}

#[ignore = "Not complete"]
#[tokio::test]
async fn test_tools_tracked_based_on_targets() {
    let total_duration = 10; // Total monitoring duration in seconds
    let python_ration = 0.6; // 60% of the time for python and 40% for top
    let file_path = "test-files/scripts/monitor.sh";

    let targets = vec![
        Target::new(TargetMatch::ProcessName("python".to_string()))
            .set_display_name(DisplayName::UseFirstArgumentBaseName()),
        Target::new(TargetMatch::ProcessName("python2".to_string()))
            .set_display_name(DisplayName::UseFirstArgumentBaseName()),
        Target::new(TargetMatch::ProcessName("python3".to_string()))
            .set_display_name(DisplayName::UseFirstArgumentBaseName()),
        Target::new(TargetMatch::ProcessName("top".to_string()))
            .set_display_name(DisplayName::UseFirstArgumentBaseName()),
    ];

    // execute scripts
    let mut output = std::process::Command::new(file_path)
        .arg(total_duration.to_string())
        .arg(python_ration.to_string())
        .spawn()
        .expect("failed to run script");

    let mut events_recorder = EventRecorder::default();
    run_process_watcher(
        &mut events_recorder,
        Duration::from_secs(total_duration),
        targets,
    )
    .await;

    let data = events_recorder.get_events();

    let (handler, export_dir) = setup_fs_exporter_and_path();
    handler
        .output(data, "testing")
        .await
        .expect("failed to output");

    let query_str = format!(
        r#" select * from "{}/**/*.parquet" limit 10;"#,
        export_dir.as_path().to_str().unwrap()
    );

    let _ = query(&query_str).await;

    // cleanup
    let _ = output.kill();
}
