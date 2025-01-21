use std::path::{Path, PathBuf};

use crate::{cloud_providers::aws::S3Client, types::parquet::FlattenedTracerEvent};

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
        profile: Option<&str>,
        role_arn: Option<&str>,
        region: &'static str,
    ) -> Self {
        let s3_client = S3Client::new(profile, role_arn, region).await;
        let export_bucket_name = String::from("tracer-client-events");

        Self {
            fs_handler,
            s3_client,
            export_bucket_name,
        }
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
