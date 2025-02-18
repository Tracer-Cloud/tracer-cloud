use std::{fmt::Debug, path::PathBuf};

use crate::types::{event::Event, parquet::FlattenedTracerEvent, ParquetSchema};

pub mod db;
pub mod fs;
pub mod s3;

pub use fs::FsExportHandler;
pub use s3::S3ExportHandler;

/// Exports the ``tracer events (FlattenedTracerEvent)`` to location based on the run_id
/// Returns path to parquet file.
#[async_trait::async_trait]
pub trait ParquetExport
where
    Self::ExportableType: ParquetSchema + Debug,
{
    type ExportableType;
    async fn output(&self, data: &[Event], run_name: &str) -> Result<PathBuf, String>;
}

pub enum Exporter {
    FS(FsExportHandler),
    S3(S3ExportHandler),
}

#[async_trait::async_trait]
impl ParquetExport for Exporter {
    type ExportableType = FlattenedTracerEvent;

    async fn output(&self, data: &[Event], run_name: &str) -> Result<PathBuf, String> {
        match self {
            Self::FS(exporter) => exporter.output(data, run_name).await,
            Self::S3(exporter) => exporter.output(data, run_name).await,
        }
    }
}
