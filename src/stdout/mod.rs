use std::sync::Arc;

use anyhow::Result;
use linemux::MuxedLines;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

// Todo: A lot of code is duplicated between this file and syslog. Maybe we could extract the file reading code into a separate module?
pub struct StdoutWatcher {
    pub last_lines: Vec<String>,
}

pub async fn run_stdout_lines_read_thread(
    file_path: &str,
    pending_lines: Arc<RwLock<Vec<String>>>,
) {
    let line_reader = MuxedLines::new();

    if line_reader.is_err() {
        return;
    }

    let mut line_reader = line_reader.unwrap();

    let result = line_reader.add_file(file_path).await;

    if result.is_err() {
        return;
    }

    while let Ok(Some(line)) = line_reader.try_next().await {
        let mut vec = pending_lines.write().await;
        let line = line.line();
        vec.push(line.to_string());
    }
}

impl StdoutWatcher {
    pub fn new() -> StdoutWatcher {
        StdoutWatcher {
            last_lines: Vec::new(),
        }
    }

    pub async fn poll_stdout(&mut self, pending_lines: Arc<RwLock<Vec<String>>>) -> Result<()> {
        let mut lines = pending_lines.write().await;

        for line in self.last_lines.iter() {
            lines.push(line.clone());
        }
        // Todo: Stream lines to the webapp

        lines.clear();

        Ok(())
    }
}
