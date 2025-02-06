mod goal_based_tests;
mod tracer_tests;

use arrow::array::RecordBatch;
use arrow::json::ArrayWriter;
use duckdb::Connection;

pub(crate) async fn query<T: for<'de> serde::Deserialize<'de>>(query: &str) -> Vec<T> {
    let conn = Connection::open_in_memory().expect("Failed to create duckdb connection");
    let mut stmt = conn.prepare(query).expect("Query failed");

    let records: Vec<RecordBatch> = stmt
        .query_arrow([])
        .expect("failed to convert to record batch")
        .collect();

    records
        .iter()
        .map(record_batch_to_structs)
        .flatten()
        .collect()
}

fn record_batch_to_structs<T: for<'de> serde::Deserialize<'de>>(batch: &RecordBatch) -> Vec<T> {
    // Convert RecordBatch to JSON
    let buf = Vec::new();
    let mut writer = ArrayWriter::new(buf);
    writer.write_batches(&[batch]).unwrap();
    writer.finish().unwrap();

    let buf = writer.into_inner();
    let json_str = String::from_utf8(buf).unwrap();

    // Deserialize JSON into structs
    serde_json::from_str::<Vec<T>>(&json_str).unwrap()
}
