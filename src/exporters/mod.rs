use std::{fmt::Debug, path::PathBuf};

use crate::types::{event::Event, ParquetSchema};

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
