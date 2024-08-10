use serde::{Deserialize, Serialize};

use std::borrow::Cow;

use super::Target;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommandContainsStruct {
    pub process_name: Option<String>,
    pub command_content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TargetMatch {
    ProcessName(String),
    ShortLivedProcessExecutable(String),
    CommandContains(CommandContainsStruct),
}

pub fn to_lowercase(s: &str) -> Cow<str> {
    if s.chars().any(|c| c.is_uppercase()) {
        Cow::Owned(s.to_lowercase())
    } else {
        Cow::Borrowed(s)
    }
}

pub fn process_name_matches(expected_name: &str, process_name: &str) -> bool {
    let process_name_lower = to_lowercase(process_name);
    let expected_name_lower = to_lowercase(expected_name);
    process_name_lower == expected_name_lower
}

pub fn command_contains(command: &str, content: &str) -> bool {
    let command_lower = to_lowercase(command);
    let content_lower = to_lowercase(content);
    command_lower.contains(content_lower.as_ref())
}

pub fn matches_target(target: &Target, process_name: &str, command: &str) -> bool {
    match &target.match_type {
        TargetMatch::ProcessName(name) => {
            if name == "any" && command_contains(command, "/opt/conda/bin") {
                true
            } else {
                process_name_matches(name, process_name)
            }
        }
        TargetMatch::ShortLivedProcessExecutable(name) => command_contains(command, name),
        TargetMatch::CommandContains(inner) => {
            let process_name_matches = inner.process_name.as_ref().map_or(true, |expected_name| {
                process_name_matches(expected_name, process_name)
            });

            process_name_matches && command_contains(command, &inner.command_content)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_manager::target_process::Target;

    #[test]
    fn test_plotpca_command() {
        let target = Target::new(TargetMatch::CommandContains(CommandContainsStruct {
            process_name: Some("python3.12".to_string()),
            command_content: "plotPCA".to_string(),
        }));
        let process_name = "python3.12";
        let command =
            "/opt/conda/bin/python3.12 /opt/conda/bin/plotPCA -in rnaseq.npz -o PCA_rnaseq_2.png";

        assert!(matches_target(&target, process_name, command));

        // Test with incorrect process name
        assert!(!matches_target(&target, "python3", command));

        // Test with incorrect command content
        assert!(!matches_target(
            &target,
            process_name,
            "/opt/conda/bin/python3.12 /opt/conda/bin/differentCommand"
        ));

        // Test conda bin with any process name
        let conda_target = Target::new(TargetMatch::ProcessName("any".to_string()));
        assert!(matches_target(
            &conda_target,
            "any_process",
            "/opt/conda/bin/somecommand"
        ));
        assert!(!matches_target(
            &conda_target,
            "any_process",
            "/usr/bin/somecommand"
        ));
    }

    #[test]
    fn test_plotfingerprint_command() {
        let target = Target::new(TargetMatch::CommandContains(CommandContainsStruct {
            process_name: Some("python3.12".to_string()),
            command_content: "plotFingerprint".to_string(),
        }));
        let process_name = "python3.12";
        let command = "/opt/conda/bin/python3.12 /opt/conda/bin/plotFingerprint -b control.sorted.bam test.sorted.bam --labels Control Test";

        assert!(matches_target(&target, process_name, command));

        // Test with incorrect process name
        assert!(!matches_target(&target, "python3", command));

        // Test with incorrect command content
        assert!(!matches_target(
            &target,
            process_name,
            "/opt/conda/bin/python3.12 /opt/conda/bin/differentCommand"
        ));
    }

    #[test]
    fn test_kallisto_command() {
        let target = Target::new(TargetMatch::CommandContains(CommandContainsStruct {
            process_name: Some("kallisto".to_string()),
            command_content: "quant".to_string(),
        }));
        let process_name = "kallisto";
        let command =
            "kallisto quant -t 4 -i control_index -o ./control_quant_9 control1_1.fq control1_2.fq";

        assert!(matches_target(&target, process_name, command));

        // Test with incorrect process name
        assert!(!matches_target(&target, "different_process", command));

        // Test with incorrect command content
        assert!(!matches_target(
            &target,
            process_name,
            "kallisto index -i transcripts.idx transcripts.fa.gz"
        ));
    }

    #[test]
    fn test_conda_bin_always_matches() {
        let target = Target::new(TargetMatch::ProcessName("any".to_string()));
        let process_name = "any_process";
        let command = "/opt/conda/bin/somecommand";

        assert!(matches_target(&target, process_name, command));

        // Even with a non-matching process name and different command, it should match due to "/opt/conda/bin"
        assert!(matches_target(
            &target,
            "different_process",
            "/opt/conda/bin/different_command"
        ));
    }

    #[test]
    fn test_conda_bin_with_specific_process() {
        let target = Target::new(TargetMatch::ProcessName("specific_process".to_string()));
        let command = "/opt/conda/bin/somecommand";

        assert!(matches_target(&target, "specific_process", command));
        assert!(!matches_target(&target, "different_process", command));
    }
}
