mod config_manager;
mod http_client;
mod process;

extern crate daemonize;

use crate::config_manager::ConfigManager;
use anyhow::Result;
use daemonize::Daemonize;
use process::TracerClient;
use std::fs::File;

fn main() -> Result<()> {
    start_daemon()?;

    async_main()
}

fn start_daemon() -> Result<()> {
    let daemonize = Daemonize::new()
        .pid_file("/tmp/tracerd.pid")
        .working_directory("/tmp")
        .stdout(File::create("/tmp/tracerd.out")?)
        .stderr(File::create("/tmp/tracerd.err")?);

    match daemonize.start() {
        Ok(_) => println!("tracer-daemon started"),
        Err(e) => eprintln!("Error, {}", e),
    }
    Ok(())
}

#[tokio::main]
async fn async_main() -> Result<()> {
    let config = ConfigManager::load_config()?;

    let mut tracer_client = TracerClient::from_config(config)?;

    loop {
        TracerClient::remove_completed_processes(&mut tracer_client).await?;
        TracerClient::poll_processes(&mut tracer_client).await?;
        TracerClient::send_event(&mut tracer_client).await?;
        TracerClient::refresh(&mut tracer_client);
    }
}
