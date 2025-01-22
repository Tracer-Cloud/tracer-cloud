use sysinfo::System;
use tracer::config_manager::ConfigManager;
use tracer::event_recorder::EventRecorder;
use tracer::exporters::ParquetExport;
use tracer::exporters::{FsExportHandler, S3ExportHandler};
use tracer::metrics::SystemMetricsCollector;

#[tokio::main]
async fn main() {
    let collector = SystemMetricsCollector::new();
    let run_name = format!("test_run_two");
    let mut recorder = EventRecorder::new(Some(run_name.clone()), Some(format!("test_id")));
    let mut system = System::new();

    let raw_config = ConfigManager::load_config();

    let export_dir =
        ConfigManager::get_tracer_parquet_export_dir().expect("Failed to get export dir");

    let fs_handler = FsExportHandler::new(export_dir, None);
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
