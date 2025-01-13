pub mod event;

pub trait ParquetSchema {
    fn schema() -> arrow::datatypes::Schema;
}
