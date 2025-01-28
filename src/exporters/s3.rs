use aws_config::SdkConfig;
use aws_sdk_s3::types::{BucketLocationConstraint, CreateBucketConfiguration};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    cloud_providers::aws::S3Client,
    types::{config::AwsConfig, parquet::FlattenedTracerEvent},
};

use super::{FsExportHandler, ParquetExport};

/// An extension of the File system handler. The underlying requirement is a parquet file has to be saved
/// first before it is exported to s3, after that, cleanup can take place
pub struct S3ExportHandler {
    fs_handler: FsExportHandler,
    s3_client: S3Client,
    export_bucket_name: String,
}

impl S3ExportHandler {
    pub async fn new(
        fs_handler: FsExportHandler,
        initializer: AwsConfig,
        region: &'static str,
    ) -> Self {
        let s3_client = S3Client::new(initializer, region).await;
        let export_bucket_name = String::from("tracer-client-events");

        let bucket_config = CreateBucketConfiguration::builder()
            .location_constraint(
                BucketLocationConstraint::from_str(region)
                    .expect("Failed to create BucketLocationConstraint"),
            )
            .build();

        // Add a small delay and retry for LocalStack's eventual consistency
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        Self::initialize_bucket(&s3_client, &export_bucket_name, bucket_config).await;

        Self {
            fs_handler,
            s3_client,
            export_bucket_name,
        }
    }

    pub async fn new_with_config(fs_handler: FsExportHandler, config: SdkConfig) -> Self {
        let region = config.region().expect("Failed to get region").to_string();
        let s3_client = S3Client::new_with_config(config, &region).await;
        let export_bucket_name = String::from("tracer-client-events");

        let bucket_config = CreateBucketConfiguration::builder()
            .location_constraint(BucketLocationConstraint::from(region.as_str()))
            .build();

        Self::initialize_bucket(&s3_client, &export_bucket_name, bucket_config).await;

        Self {
            fs_handler,
            s3_client,
            export_bucket_name,
        }
    }

    async fn initialize_bucket(
        client: &S3Client,
        bucket_name: &str,
        bucket_config: CreateBucketConfiguration,
    ) {
        // First try to create the bucket
        match client.create_bucket(bucket_name, Some(bucket_config)).await {
            Ok(_) => log::info!("Successfully created bucket {}", bucket_name),
            Err(e) => {
                if e.to_string().contains("BucketAlreadyOwnedByYou") {
                    log::info!("Bucket {} already exists", bucket_name);
                } else {
                    log::error!("Error creating bucket: {}", e);
                }
            }
        }

        // Wait for bucket to be available (LocalStack sometimes needs this)
        for _ in 0..3 {
            match client.client.head_bucket().bucket(bucket_name).send().await {
                Ok(_) => {
                    log::info!("Bucket {} is ready", bucket_name);
                    return;
                }
                Err(err) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    eprintln!("error creating bucket {err}");
                    continue;
                }
            }
        }

        panic!(
            "Failed to verify bucket {} exists after multiple attempts",
            bucket_name
        );
    }

    fn extract_key(&self, file_path: &Path) -> Option<String> {
        if let Some(path) = file_path.to_str() {
            if let Some(start_pos) = path.find("exports") {
                return Some(path[start_pos..].to_string());
            }
        }
        None
    }
}

#[async_trait::async_trait]
impl ParquetExport for S3ExportHandler {
    type ExportableType = FlattenedTracerEvent;

    async fn output(
        &self,
        data: &[crate::types::event::Event],
        run_name: &str,
    ) -> Result<PathBuf, String> {
        match self.fs_handler.output(data, run_name).await {
            Ok(file_path) => {
                let key = self
                    .extract_key(&file_path)
                    .unwrap_or("annonymous".to_string());

                let str_path = file_path
                    .to_str()
                    .expect("Failed to convert file path to str");

                if let Err(err) = self
                    .s3_client
                    .put_object(&self.export_bucket_name, str_path, &key)
                    .await
                {
                    Err(err)
                } else {
                    Ok(file_path)
                }
            }
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use crate::cloud_providers::aws::setup_env_vars;
    use serial_test::serial;

    use crate::event_recorder::{EventRecorder, EventType};
    use crate::metrics::SystemMetricsCollector;
    use aws_config::BehaviorVersion;
    use sysinfo::System;
    use tempdir::TempDir;

    #[tokio::test]
    #[serial]
    async fn test_s3_exporter_initializes_export_bucket_on_start() {
        let region = "us-east-2";
        setup_env_vars(region);

        let tracer_bucket_name = "tracer-client-events".to_string();
        let temp_dir = TempDir::new(&tracer_bucket_name).expect("failed to create tempdir");
        let base_dir = temp_dir.path().join("./exports");

        let endpoint_url = std::env::var("S3_ENDPOINT_URL").unwrap();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .endpoint_url(endpoint_url.clone())
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(true)
            .build();
        let s3_client = S3Client::new_with_s3_config(s3_config.clone(), region).await;

        let fs_handler = FsExportHandler::new(base_dir, None);
        let _exporter = S3ExportHandler::new_with_config(fs_handler, config).await;
        let buckets = s3_client
            .list_buckets()
            .await
            .expect("Failed to list buckets");

        assert_eq!(buckets.len(), 1);
        assert!(buckets.contains(&tracer_bucket_name));
    }

    #[tokio::test]
    #[serial]
    async fn test_s3_exporter_output_to_parquet_succeeds() -> Result<(), String> {
        let region = "us-east-2";
        setup_env_vars(region);
        let mut system = System::new();
        let mut logs = EventRecorder::default();
        let metrics_collector = SystemMetricsCollector::new();
        let temp_dir = TempDir::new("tracer-client-events").expect("failed to create tempdir");

        let base_dir = temp_dir.path().join("./exports");

        let endpoint_url = std::env::var("S3_ENDPOINT_URL").unwrap();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .endpoint_url(endpoint_url.clone())
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(true)
            .build();
        let s3_client = S3Client::new_with_s3_config(s3_config.clone(), region).await;
        let tracer_bucket_name = "tracer-client-events".to_string();

        let fs_handler = FsExportHandler::new(base_dir, None);
        let exporter = S3ExportHandler::new_with_config(fs_handler, config).await;

        metrics_collector
            .collect_metrics(&mut system, &mut logs)
            .expect("Failed to collect metrics");

        // Record a test event
        logs.record_event(
            EventType::TestEvent,
            "[submit_batched_data.rs] Test event".to_string(),
            None,
            None,
        );
        let data = logs.get_events();
        let file_path = exporter
            .output(data, "annoymous")
            .await
            .expect("failed to export output");
        logs.clear();

        let key = exporter.extract_key(file_path.as_path()).unwrap();

        println!("Key: {key}");

        // list objects
        let objects = s3_client
            .client
            .list_objects()
            .bucket(&tracer_bucket_name)
            .max_keys(1)
            .send()
            .await
            .expect("Failed to list objects")
            .contents
            .unwrap();

        let object = objects.first();

        assert_eq!(object.unwrap().key, Some(key));

        Ok(())
    }
}
