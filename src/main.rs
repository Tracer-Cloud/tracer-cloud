use anyhow::{Context, Ok, Result};
use tracer::cli::process_cli;

pub fn main() -> Result<()> {
    process_cli()
}
