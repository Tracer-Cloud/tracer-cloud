use anyhow::Result;
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};
use sysinfo::System;

use crate::{
    config_manager::{Target, TargetMatch},
    daemon_communication::client::send_log_short_lived_process_request,
    process_watcher::{ProcessProperties, ProcessWatcher, ShortLivedProcessLog},
};

const TRACER_BASH_RC_PATH: &str = ".config/tracer/.bashrc";
const WRAPPER_SOURCE_COMMAND: &str = "source ~/.config/tracer/.bashrc";

pub fn get_task_wrapper(
    current_tracer_exe_path: PathBuf,
    command_name: &str,
    display_name: &str,
) -> String {
    format!(
        "alias {}=\"{} & {} log-short-lived-process \\\"{}\\\"; wait\"\n",
        command_name,
        command_name,
        current_tracer_exe_path.as_os_str().to_str().unwrap(),
        display_name,
    )
}

pub fn rewrite_wrapper_bashrc_file(
    current_tracer_exe_path: PathBuf,
    targets: Vec<&Target>,
) -> Result<()> {
    let path = homedir::get_my_home()?.unwrap();

    let mut bashrc_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path.join(TRACER_BASH_RC_PATH))?;

    for command in targets.into_iter().map(|target| {
        let name = target.get_display_name();
        let command_to_alias = match &target.match_type {
            TargetMatch::ShortLivedProcessExecutable(alias) => alias.clone(),
            _ => "unknown_command".to_string(),
        };
        format!(
            "{}\n",
            get_task_wrapper(
                current_tracer_exe_path.clone(),
                &command_to_alias,
                &name.unwrap_or(command_to_alias.clone())
            )
        )
    }) {
        bashrc_file.write_all(command.as_bytes()).unwrap();
    }

    Ok(())
}

pub fn modify_bashrc_file(bashrc_file_path: &str) -> Result<()> {
    let path = homedir::get_my_home()?.unwrap();

    let mut bashrc_file = OpenOptions::new()
        .read(true)
        .append(true)
        .open(path.join(bashrc_file_path))?;

    let reader = BufReader::new(&bashrc_file);
    for line in reader.lines() {
        let line = line.unwrap();
        if line.contains(WRAPPER_SOURCE_COMMAND) {
            return Ok(());
        }
    }

    bashrc_file
        .write_all(WRAPPER_SOURCE_COMMAND.as_bytes())
        .unwrap();

    Ok(())
}

pub fn setup_aliases(current_tracer_exe_path: PathBuf, commands: Vec<&Target>) -> Result<()> {
    rewrite_wrapper_bashrc_file(current_tracer_exe_path, commands)?;
    modify_bashrc_file(".bashrc")?;

    println!("Aliases setup successfully.");
    Ok(())
}

pub async fn log_short_lived_process(socket_path: &str, command: &str) -> Result<()> {
    let system = System::new();

    // Doing logging here so we have a larger time window for the process to be alive
    let process = system.processes_by_name(command).last();
    let data: ShortLivedProcessLog = if let Some(process) = process {
        ShortLivedProcessLog {
            command: command.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            properties: ProcessWatcher::gather_process_data(&process.pid(), process, None),
        }
    } else {
        ShortLivedProcessLog {
            command: command.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            properties: ProcessProperties {
                tool_name: command.to_string(),
                tool_pid: "".to_string(),
                tool_binary_path: "".to_string(),
                tool_cmd: command.to_string(),
                start_timestamp: chrono::Utc::now().to_rfc3339(),
                process_cpu_utilization: 0.0,
                process_memory_usage: 0,
                process_memory_virtual: 0,
            },
        }
    };

    send_log_short_lived_process_request(socket_path, data).await?;

    println!("Logged short lived process: {}", command);
    Ok(())
}
