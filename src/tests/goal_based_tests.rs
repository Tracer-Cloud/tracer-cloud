use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use super::query;
use crate::config_manager::target_process::target_matching::TargetMatch;
use crate::config_manager::target_process::DisplayName;
use crate::config_manager::target_process::Target;
use crate::events::recorder::EventRecorder;
use crate::exporters::FsExportHandler;
use crate::exporters::ParquetExport;
use crate::extracts::file_watcher::FileWatcher;
use crate::extracts::process_watcher::ProcessWatcher;
use crate::types::parquet::FlattenedTracerEvent;
use itertools::Itertools;
use sysinfo::System;
use tempdir::TempDir;

fn setup_fs_exporter_and_path() -> (FsExportHandler, PathBuf) {
    let temp_dir = TempDir::new("exports").expect("failed to create tempdir");

    let export_dir = temp_dir.path().to_path_buf();

    (FsExportHandler::new(export_dir.clone(), None), export_dir)
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

#[tokio::test]
async fn test_query_via_duckdb_works() {
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

    let query_res: Vec<FlattenedTracerEvent> = query(&query_str).await;

    assert!(!query_res.is_empty());

    // cleanup
    let _ = std::fs::remove_dir_all(export_dir);
    let _ = output.kill();
}

#[tokio::test]
async fn test_tools_tracked_based_on_targets() {
    let total_duration = 10; // Total monitoring duration in seconds
    let python_ration = 0.6; // 60% of the time for python and 40% for top
    let file_path = "test-files/scripts/monitor.sh";

    let targets = vec![
        Target::new(TargetMatch::ProcessName("python".to_string()))
            .set_display_name(DisplayName::Default()),
        Target::new(TargetMatch::ProcessName("python2".to_string()))
            .set_display_name(DisplayName::Default()),
        Target::new(TargetMatch::ProcessName("python3".to_string()))
            .set_display_name(DisplayName::Default()),
        Target::new(TargetMatch::ProcessName("top".to_string()))
            .set_display_name(DisplayName::Default()),
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

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    struct ProcessSubSet {
        tool_pid: Option<String>,
        tool_name: String,
    }

    let mut expected_tool_names = vec!["python3".to_string(), "top".to_string()];
    expected_tool_names.sort();

    let query_processes_for_a_run_name = format!(
        r#"select process_attributes.tool_pid, process_attributes.tool_name
        from "{}/**/*.parquet"
        where process_attributes.tool_pid is not Null
        group by 
        process_attributes.tool_name, process_attributes.tool_pid;"#,
        export_dir.as_path().to_str().unwrap()
    );

    let query_res: Vec<ProcessSubSet> = query(&query_processes_for_a_run_name).await;

    assert_eq!(query_res.len(), 2);

    let queried_process_names: Vec<String> = query_res
        .into_iter()
        .map(|p| p.tool_name)
        .sorted()
        .collect();

    assert_eq!(queried_process_names, expected_tool_names);

    // cleanup
    let _ = output.kill();
    let _ = std::fs::remove_dir_all(export_dir);
}
