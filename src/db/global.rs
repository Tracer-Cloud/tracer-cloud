use std::sync::Arc;
use tokio::sync::OnceCell;
use crate::db::aurora_client::AuroraClient;

pub static AURORA_CLIENT: OnceCell<Arc<AuroraClient>> = OnceCell::const_new();

/// Returns the global AuroraClient connection pool. Initializes it on first call.
pub async fn get_aurora_client() -> Arc<AuroraClient> {
    AURORA_CLIENT
        .get_or_init(|| {
            // Wrap the async block in a synchronous closure
            async {
                Arc::new(
                    AuroraClient::new()
                        .await
                        .expect("Failed to initialize AuroraClient"),
                )
            }
        })
        .await
        .clone()
}