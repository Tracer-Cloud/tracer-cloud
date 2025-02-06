use crate::config_manager::{Config, ConfigManager};
use crate::{monitor_processes_with_tracer_client, FsExportHandler, S3ExportHandler, TracerClient};
use aws_config::BehaviorVersion;
use dotenv::dotenv;
use tempdir::TempDir;

fn load_test_config() -> Config {
    ConfigManager::load_default_config()
}

pub fn setup_env_vars(region: &str) {
    dotenv().ok(); // Load from .env file in development
    std::env::set_var("AWS_REGION", region);
}

#[tokio::test]
async fn test_monitor_processes_with_tracer_client() {
    let config = load_test_config();
    let pwd = std::env::current_dir().unwrap();
    let region = "us-east-2";

    setup_env_vars(region);

    let temp_dir = TempDir::new("export").expect("failed to create tempdir");
    let base_dir = temp_dir.path().join("./exports");
    let fs_handler = FsExportHandler::new(base_dir, None);

    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .load()
        .await;

    let s3_handler = S3ExportHandler::new_with_config(fs_handler, aws_config).await;

    let mut tracer_client = TracerClient::new(
        config,
        pwd.to_str().unwrap().to_string(),
        s3_handler,
        "testing".to_string(),
    )
    .await
    .unwrap();
    let result = monitor_processes_with_tracer_client(&mut tracer_client).await;
    assert!(result.is_ok());
}
