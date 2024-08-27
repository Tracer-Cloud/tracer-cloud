use serde::Deserialize;

#[derive(Deserialize)]
pub struct InfoResponse {
    pub run_name: String,
    pub run_id: String,
    pub service_name: String,
}
