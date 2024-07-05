use clap::{Parser, Subcommand};

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
