// File: src/target/mod.rs
pub mod target_matching;
pub mod targets_list;
use serde::{Deserialize, Serialize};
use target_matching::{matches_target, TargetMatch};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommandContainsStruct {
    pub process_name: Option<String>,
    pub command_content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Target {
    pub match_type: TargetMatch,
    pub display_name: Option<String>,
    pub merge_with_parents: bool,
    pub force_ancestor_to_match: bool,
}

impl Target {
    pub fn new(match_type: TargetMatch) -> Target {
        Target {
            match_type,
            display_name: None,
            merge_with_parents: true,
            force_ancestor_to_match: true,
        }
    }

    pub fn set_display_name(self, display_name: Option<String>) -> Target {
        Target {
            display_name,
            ..self
        }
    }

    pub fn set_merge_with_parents(self, merge_with_parents: bool) -> Target {
        Target {
            merge_with_parents,
            ..self
        }
    }

    pub fn set_force_ancestor_to_match(self, force_ancestor_to_match: bool) -> Target {
        Target {
            force_ancestor_to_match,
            ..self
        }
    }

    pub fn matches(&self, process_name: &str, command: &str) -> bool {
        matches_target(self, process_name, command)
    }

    pub fn should_be_merged_with_parents(&self) -> bool {
        self.merge_with_parents
    }

    pub fn should_force_ancestor_to_match(&self) -> bool {
        self.force_ancestor_to_match
    }

    pub fn get_display_name(&self) -> Option<String> {
        self.display_name.clone()
    }
}
