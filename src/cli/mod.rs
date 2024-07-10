use crate::{
    config_manager::{ConfigManager, TargetMatch},
    daemon_communication::client::{
        send_alert_request, send_end_run_request, send_log_request, send_start_run_request,
        send_stop_request, send_update_tags_request,
    },
    nondaemon_commands::{
        clean_up_after_daemon, print_config_info_sync, setup_config, test_service_config_sync,
        update_tracer,
    },
    run, start_daemon,
    task_wrapper::{log_short_lived_process, setup_aliases},
    SOCKET_PATH,
};
use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use std::env;

#[derive(Parser)]
#[clap(
    name = "tracer",
    about = "A tool for monitoring bioinformatics applications",
    version = env!("CARGO_PKG_VERSION")
)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Setup {
        api_key: Option<String>,
        service_url: Option<String>,
        process_polling_interval_ms: Option<u64>,
        batch_submission_interval_ms: Option<u64>,
    },
    Log {
        message: String,
    },
    Alert {
        message: String,
    },
    Init,
    Cleanup,
    Info,
    Stop,
    Update,
    Start,
    End,
    Test,
    Tag {
        tags: Vec<String>,
    },
    ApplyBashrc,
    LogShortLivedProcess {
        command: String,
    },
    Version,
}

pub fn process_cli() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            let test_result = test_service_config_sync();
            if test_result.is_err() {
                return Ok(());
            }
            println!("Starting daemon...");
            let result = start_daemon();
            if result.is_err() {
                println!("Failed to start daemon. Maybe the daemon is already running? If it's not, run `tracer cleanup` to clean up the previous daemon files.");
                return Ok(());
            }
            run()?;
            clean_up_after_daemon()
        }
        Commands::Test => {
            let result = test_service_config_sync();
            if result.is_ok() {
                println!("Tracer was able to successfully communicate with the API service.");
            }
            Ok(())
        }
        Commands::Cleanup => {
            let result = clean_up_after_daemon();
            if result.is_ok() {
                println!("Daemon files cleaned up successfully.");
            }
            result
        }
        Commands::ApplyBashrc => {
            let config = ConfigManager::load_config();
            setup_aliases(
                env::current_exe()?,
                config
                    .targets
                    .iter()
                    .filter_map(|target| {
                        if let TargetMatch::ShortLivedProcessExecutable(_) = &target.match_type {
                            Some(target)
                        } else {
                            None
                        }
                    })
                    .collect(),
            )
        }
        Commands::Info => print_config_info_sync(),
        _ => run_async_command(cli.command),
    }
}

#[tokio::main]
pub async fn run_async_command(commands: Commands) -> Result<()> {
    let value = match commands {
        Commands::Log { message } => send_log_request(SOCKET_PATH, message).await,
        Commands::Alert { message } => send_alert_request(SOCKET_PATH, message).await,
        Commands::Stop => send_stop_request(SOCKET_PATH).await,
        Commands::Start => send_start_run_request(SOCKET_PATH).await,
        Commands::End => send_end_run_request(SOCKET_PATH).await,
        Commands::Update => update_tracer().await,
        Commands::Tag { tags } => send_update_tags_request(SOCKET_PATH, &tags).await,
        Commands::Setup {
            api_key,
            service_url,
            process_polling_interval_ms,
            batch_submission_interval_ms,
        } => {
            setup_config(
                &api_key,
                &service_url,
                &process_polling_interval_ms,
                &batch_submission_interval_ms,
            )
            .await
        }
        Commands::LogShortLivedProcess { command } => {
            log_short_lived_process(SOCKET_PATH, &command).await
        }
        _ => {
            println!("Command not implemented yet");
            Ok(())
        }
    };

    if value.is_err() {
        println!("Failed to send command to the daemon. Maybe the daemon is not running? If it's not, run `tracer init` to start the daemon.");
    } else {
        println!("Command sent successfully.")
    }

    Ok(())
}
