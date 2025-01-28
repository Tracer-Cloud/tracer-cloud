use std::{fmt::Debug, path::PathBuf};

use crate::types::{event::Event, ParquetSchema};

pub mod fs;
pub mod s3;

pub use fs::FsExportHandler;
pub use s3::S3ExportHandler;

/// Exports the ``tracer events (FlattenedTracerEvent)`` to location based on the run_id
/// Returns path to parquet file.
pub trait ParquetExport
where
    Self::ExportableType: ParquetSchema + Debug,
{
    type ExportableType;
    #[allow(async_fn_in_trait)]
    async fn output(&self, data: &[Event], run_name: &str) -> Result<PathBuf, String>;
}
