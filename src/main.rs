mod config_manager;
mod http_client;
mod process;

use anyhow::{Context, Result};
use daemonize::Daemonize;
use std::fs::File;
use tokio::time::{sleep, Duration};

use crate::config_manager::ConfigManager;
use crate::process::TracerClient;

const PID_FILE: &str = "/tmp/tracerd.pid";
const WORKING_DIR: &str = "/tmp";
const STDOUT_FILE: &str = "/tmp/tracerd.out";
const STDERR_FILE: &str = "/tmp/tracerd.err";
const DEFAULT_POLLING_INTERVAL: Duration = Duration::from_micros(100); // 0.1 ms in microseconds

#[tokio::main]
async fn main() -> Result<()> {
    start_daemon()?;
    run().await
}

fn start_daemon() -> Result<()> {
    Daemonize::new()
        .pid_file(PID_FILE)
        .working_directory(WORKING_DIR)
        .stdout(File::create(STDOUT_FILE).context("Failed to create stdout file")?)
        .stderr(File::create(STDERR_FILE).context("Failed to create stderr file")?)
        .start()
        .context("Failed to start daemon")?;
    println!("tracer-daemon started");
    Ok(())
}

async fn run() -> Result<()> {
    let config = ConfigManager::load_config().context("Failed to load config")?;
    let mut tracer_client =
        TracerClient::from_config(config).context("Failed to create TracerClient")?;

    loop {
        process_tracer_client(&mut tracer_client).await?;
        sleep(DEFAULT_POLLING_INTERVAL).await;
    }
}

async fn process_tracer_client(tracer_client: &mut TracerClient) -> Result<()> {
    TracerClient::remove_completed_processes(tracer_client).await?;
    TracerClient::poll_processes(tracer_client).await?;
    TracerClient::send_event(tracer_client).await?;
    TracerClient::refresh(tracer_client);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use tokio::time::timeout;

    const TEST_CONFIG_PATH: &str = "/tmp/test_tracer.toml";
    const TEST_CONFIG_CONTENT: &str = r#"
        api_key = "_Zx2h6toXUnD1i_QjuRvD"
        polling_interval_ms = 1000
        targets = ["target1", "target2"]
    "#;

    fn create_test_config() {
        let mut file = File::create(TEST_CONFIG_PATH).unwrap();
        file.write_all(TEST_CONFIG_CONTENT.as_bytes()).unwrap();
    }

    #[tokio::test]
    async fn test_run() {
        create_test_config();
        env::set_var("TRACER_CONFIG", TEST_CONFIG_PATH);

        let result = timeout(Duration::from_secs(5), run()).await;
        assert!(
            result.is_err(),
            "run() should not complete within 5 seconds"
        );

        env::remove_var("TRACER_CONFIG");
        std::fs::remove_file(TEST_CONFIG_PATH).unwrap();
    }
}
