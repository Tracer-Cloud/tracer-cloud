pub mod aws;
pub mod config;
pub mod event;
pub mod parquet;

pub trait ParquetSchema {
    fn schema() -> arrow::datatypes::Schema;
}
