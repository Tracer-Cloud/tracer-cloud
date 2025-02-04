use sysinfo::System;
use tracer::config_manager::ConfigManager;
use tracer::events::recorder::EventRecorder;
use tracer::exporters::ParquetExport;
use tracer::exporters::{FsExportHandler, S3ExportHandler};
use tracer::extracts::metrics::SystemMetricsCollector;

/// This file goes to S3 but needs tweaking

#[tokio::main]
async fn main() {
    let collector = SystemMetricsCollector::new();
    let pipeline_name = "s3_test_pipeline".to_string();
    let run_name = "test_run_two_22".to_string();
    let mut recorder = EventRecorder::new(
        Some(pipeline_name),
        Some(run_name.clone()),
        Some("test_id".to_string()),
    );
    let mut system = System::new();

    // loads default config with profile as initialization
    let raw_config = ConfigManager::load_default_config();

    let export_dir =
        ConfigManager::get_tracer_parquet_export_dir().expect("Failed to get export dir");

    let fs_handler = FsExportHandler::new(export_dir, None);

    // default config loads Profile either [default] or [me] from aws credentials
    let s3_handler = S3ExportHandler::new(
        fs_handler,
        raw_config.aws_init_type.clone(),
        raw_config.aws_region.as_str(),
    )
    .await;

    let mut count = 5;

    while count > 0 {
        let _ = collector.collect_metrics(&mut system, &mut recorder);
        count -= 1;
        std::thread::sleep(std::time::Duration::from_millis(100));
        system.refresh_all();
    }

    let data = recorder.get_events();

    if let Err(err) = s3_handler.output(data, &run_name).await {
        println!("error from creating parquet file {}", err)
    }
}
