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
    if command_contains(command, "/opt/conda/bin") {
        return true;
    }
    match &target.match_type {
        TargetMatch::ProcessName(name) => process_name_matches(name, process_name),
        TargetMatch::ShortLivedProcessExecutable(_) => false,
        TargetMatch::CommandContains(inner) => {
            let process_name_matches = inner.process_name.as_ref().map_or(true, |expected_name| {
                process_name_matches(expected_name, process_name)
            });

            process_name_matches && command_contains(command, &inner.command_content)
        }
    }
}
