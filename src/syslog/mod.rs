use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

pub fn grep_out_of_memory_errors(file_path: &str) -> io::Result<Vec<String>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let re = Regex::new(r"(?i)Out of memory").unwrap();

    let mut errors = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if re.is_match(&line) {
            errors.push(line);
        }
    }

    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grep_out_of_memory_errors() {
        // Create a temporary log file for testing
        let test_log = "var/log/syslog";
        std::fs::write(
            test_log,
            "\
            This is a test log\n\
            Out of memory error occurred\n\
            Another line\n\
            Yet another Out of memory issue\n",
        )
        .unwrap();

        let errors = grep_out_of_memory_errors(test_log).unwrap();
        std::fs::remove_file(test_log).unwrap();

        assert_eq!(errors.len(), 2);
        assert!(errors.contains(&"Out of memory error occurred".to_string()));
        assert!(errors.contains(&"Yet another Out of memory issue".to_string()));
    }
}
