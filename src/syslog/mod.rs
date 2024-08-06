use regex::Regex;
use std::error::Error;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::upload::upload_from_file_path;

pub async fn grep_out_of_memory_errors(file_path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let file = File::open(file_path).await?;
    let reader = BufReader::new(file);
    let re = Regex::new(r"(?i)Out of memory")?;

    let mut errors = Vec::new();
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if re.is_match(&line) {
            errors.push(line);
        }
    }

    upload_from_file_path(file_path).await?;

    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_log::Logger;

    #[tokio::test]
    async fn test_grep_out_of_memory_errors() {
        let path = "test-files/var/log/syslog";

        match grep_out_of_memory_errors(path).await {
            Ok(errors) => {
                let logger = Logger::new();

                let _ = logger
                    .log(
                        "grep_out_of_memory_errors",
                        Some(&serde_json::json!({
                            "errors": errors,
                        })),
                    )
                    .await;
            }
            Err(e) => eprintln!("Error occurred: {}", e),
        }
    }
}
