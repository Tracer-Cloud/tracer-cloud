use lazy_static::lazy_static;

use serde::{Deserialize, Serialize};
use std::borrow::Cow;

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Target {
    pub match_type: TargetMatch,
    pub display_name: Option<String>,
    pub merge_with_parents: bool,
    pub force_ancestor_to_match: bool,
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

lazy_static! {
    pub static ref TARGETS: Vec<Target> = [
        Target::new(TargetMatch::ProcessName("python".to_string())),
        Target::new(TargetMatch::CommandContains(CommandContainsStruct {
            process_name: Some("java".to_string()),
            command_content: "uk.ac.babraham.FastQC.FastQCApplication".to_string()
        }))
        .set_display_name(Some("fastqc".to_string()))
        .set_merge_with_parents(true)
        .set_force_ancestor_to_match(false),
        Target::new(TargetMatch::CommandContains(CommandContainsStruct {
            process_name: Some("bowtie2-build-s".to_string()),
            command_content: "/opt/conda/bin/bowtie2-build-s".to_string()
        }))
        .set_display_name(Some("bowtie2-build-s (Conda)".to_string())),
        Target::new(TargetMatch::ProcessName("STAR".to_string())),
        Target::new(TargetMatch::ProcessName("bowtie2".to_string())),
        Target::new(TargetMatch::ProcessName("bowtie2-build-s".to_string())),
        Target::new(TargetMatch::ProcessName("bowtie2-align-s".to_string())),
        Target::new(TargetMatch::ProcessName("bwa".to_string())),
        Target::new(TargetMatch::ProcessName("salmon".to_string())),
        Target::new(TargetMatch::ProcessName("hisat2".to_string())),
        Target::new(TargetMatch::ProcessName("hisat2-build".to_string())),
        Target::new(TargetMatch::ProcessName("stringtie".to_string())),
        Target::new(TargetMatch::ProcessName("HOMER".to_string())),
        Target::new(TargetMatch::ProcessName("samtools".to_string())),
        Target::new(TargetMatch::ProcessName("bedtools".to_string())),
        Target::new(TargetMatch::ProcessName("deeptools".to_string())),
        Target::new(TargetMatch::ProcessName("macs3".to_string())),
        Target::new(TargetMatch::ProcessName("plotCoverage".to_string())),
        Target::new(TargetMatch::ProcessName("plotFingerprint".to_string())),
        Target::new(TargetMatch::ProcessName("MACS33".to_string())),
        Target::new(TargetMatch::ProcessName("Genrich".to_string())),
        Target::new(TargetMatch::ProcessName("TopHat".to_string())),
        Target::new(TargetMatch::ProcessName("JAMM".to_string())),
        Target::new(TargetMatch::ProcessName("fastqc".to_string())),
        Target::new(TargetMatch::ShortLivedProcessExecutable(
            "fastqc".to_string()
        )),
        Target::new(TargetMatch::ProcessName("multiqc".to_string())),
        Target::new(TargetMatch::ProcessName("fastp".to_string())),
        Target::new(TargetMatch::ProcessName("PEAR".to_string())),
        Target::new(TargetMatch::ProcessName("Trimmomatic".to_string())),
        Target::new(TargetMatch::ProcessName("sra-toolkit".to_string())),
        Target::new(TargetMatch::ProcessName("Picard".to_string())),
        Target::new(TargetMatch::ProcessName("cutadapt".to_string())),
        Target::new(TargetMatch::ProcessName("cellranger".to_string())),
        Target::new(TargetMatch::ProcessName("STATsolo".to_string())),
        Target::new(TargetMatch::ProcessName("scTE".to_string())),
        Target::new(TargetMatch::ProcessName("scanpy".to_string())),
        Target::new(TargetMatch::ProcessName("Seurat".to_string())),
        Target::new(TargetMatch::ProcessName("LIGER".to_string())),
        Target::new(TargetMatch::ProcessName("SC3".to_string())),
        Target::new(TargetMatch::ProcessName("Louvain".to_string())),
        Target::new(TargetMatch::ProcessName("Leiden".to_string())),
        Target::new(TargetMatch::ProcessName("Garnett".to_string())),
        Target::new(TargetMatch::ProcessName("Monocle".to_string())),
        Target::new(TargetMatch::ProcessName("Harmony".to_string())),
        Target::new(TargetMatch::ProcessName("PAGA".to_string())),
        Target::new(TargetMatch::ProcessName("Palantir".to_string())),
        Target::new(TargetMatch::ProcessName("velocity".to_string())),
        Target::new(TargetMatch::ProcessName("CellPhoneDB".to_string())),
        Target::new(TargetMatch::ProcessName("CellChat".to_string())),
        Target::new(TargetMatch::ProcessName("NicheNet".to_string())),
        Target::new(TargetMatch::ProcessName("FIt-SNE".to_string())),
        Target::new(TargetMatch::ProcessName("umap".to_string())),
        Target::new(TargetMatch::ProcessName("bbmap".to_string())),
        Target::new(TargetMatch::ProcessName("cuffdiff".to_string())),
        Target::new(TargetMatch::ProcessName("RNA-SeQC".to_string())),
        Target::new(TargetMatch::ProcessName("RSeQC".to_string())),
        Target::new(TargetMatch::ProcessName("Trimgalore".to_string())),
        Target::new(TargetMatch::ProcessName("UCHIME".to_string())),
        Target::new(TargetMatch::ProcessName("Erange".to_string())),
        Target::new(TargetMatch::ProcessName("X-Mate".to_string())),
        Target::new(TargetMatch::ProcessName("SpliceSeq".to_string())),
        Target::new(TargetMatch::ProcessName("casper".to_string())),
        Target::new(TargetMatch::ProcessName("DESeq".to_string())),
        Target::new(TargetMatch::ProcessName("EdgeR".to_string())),
        Target::new(TargetMatch::ProcessName("kallisto".to_string())),
        Target::new(TargetMatch::ProcessName("pairtools".to_string())),
        Target::new(TargetMatch::ProcessName("HiCExplorer".to_string())),
        Target::new(TargetMatch::ProcessName("GITAR".to_string())),
        Target::new(TargetMatch::ProcessName("TADbit".to_string())),
        Target::new(TargetMatch::ProcessName("Juicer".to_string())),
        Target::new(TargetMatch::ProcessName("HiC-Pro".to_string())),
        Target::new(TargetMatch::ProcessName("cooler".to_string())),
        Target::new(TargetMatch::ProcessName("cooltools".to_string())),
        Target::new(TargetMatch::ProcessName("runHiC".to_string())),
        Target::new(TargetMatch::ProcessName("HTSlib".to_string())),
        Target::new(TargetMatch::ProcessName("htslib".to_string())),
        Target::new(TargetMatch::ProcessName("zlib".to_string())),
        Target::new(TargetMatch::ProcessName("libbz2".to_string())),
        Target::new(TargetMatch::ProcessName("liblzma".to_string())),
        Target::new(TargetMatch::ProcessName("libcurl".to_string())),
        Target::new(TargetMatch::ProcessName("libdeflate".to_string())),
        Target::new(TargetMatch::ProcessName("ncurses".to_string())),
        Target::new(TargetMatch::ProcessName("pthread".to_string())),
    ]
    .to_vec();
}
