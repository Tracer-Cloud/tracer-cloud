use std::collections::HashSet;
use std::fs;
use std::{collections::HashMap, path::Path};

use anyhow::Result;
use async_recursion::async_recursion;
use chrono::{DateTime, TimeDelta, Utc};
use lazy_static::lazy_static;
use predicates::prelude::predicate;
use predicates::str::RegexPredicate;
use predicates::Predicate;

use crate::debug_log::Logger;
use crate::upload::upload_from_file_path;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub last_update: DateTime<Utc>,
    pub last_upload: Option<DateTime<Utc>>,
    pub cached_path: Option<String>,
    pub action: FileAction,
}

pub struct FileWatcher {
    watched_files: HashMap<String, FileInfo>,
}

pub enum FilePattern {
    DirectoryPath(String),
    FilenameMatch(RegexPredicate),
    PathMatch(RegexPredicate),
}

#[derive(Clone, Debug)]
pub enum FileAction {
    None,
    Upload,
}

#[derive(Debug)]
enum FileUploadType {
    None,
    Old,
    New,
}

const CACHED_FILE_NAME_CHARSET: &str = "abcdefghijklmnoprstuwxyz0123456789";
const CACHED_FILE_NAME_LENGTH: usize = 16;

lazy_static! {
    static ref FILE_WATCHER_PATTERNS: Vec<(FilePattern, FileAction)> = vec![
        (
            FilePattern::FilenameMatch(predicate::str::is_match("Log.final.out").unwrap()),
            FileAction::Upload
        ),
        (
            FilePattern::FilenameMatch(predicate::str::is_match(".narrowPeak").unwrap()),
            FileAction::Upload
        ),
        (
            FilePattern::FilenameMatch(predicate::str::is_match("_counts.summary").unwrap()),
            FileAction::Upload
        ),
        (
            FilePattern::DirectoryPath("example-directory-path/".to_string()),
            FileAction::Upload
        ),
        (
            FilePattern::PathMatch(predicate::str::is_match("example-path[a-zA-Z]*").unwrap()),
            FileAction::Upload
        ),
        (
            FilePattern::FilenameMatch(predicate::str::is_match("example-filename").unwrap()),
            FileAction::Upload,
        ),
        (
            FilePattern::PathMatch(predicate::str::is_match("example-path-nonaction").unwrap()),
            FileAction::None
        ),
    ];
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            watched_files: HashMap::new(),
        }
    }

    #[async_recursion]
    pub async fn gather_pattern_from_directory(
        current_watched_files: &mut HashMap<String, FileInfo>,
        directory: &Path,
        pattern: &FilePattern,
        action: &FileAction,
    ) -> Result<()> {
        let logger = Logger::new();

        if !directory.exists() {
            return Ok(());
        }

        let files = directory.read_dir().unwrap();

        for file in files {
            if let Err(err) = file {
                logger
                    .log(&format!("Error reading file: {}", err), None)
                    .await;
                continue;
            }

            let file = file.unwrap();
            if file.path().is_dir() {
                Self::gather_pattern_from_directory(
                    current_watched_files,
                    &file.path(),
                    pattern,
                    action,
                )
                .await?;
                continue;
            }

            let mut matched = false;
            let file_path = file.path();
            let file_path = file_path.to_str().unwrap();
            let file_name = file.file_name().into_string().unwrap();

            match pattern {
                FilePattern::DirectoryPath(path) => {
                    if path == directory.to_str().unwrap() {
                        matched = true;
                    }
                }
                FilePattern::FilenameMatch(regex) => {
                    if regex.eval(&file_name) {
                        matched = true;
                    }
                }
                FilePattern::PathMatch(regex) => {
                    if regex.eval(file_path) {
                        matched = true;
                    }
                }
            }

            if matched {
                let metadata = file.metadata().unwrap();
                let last_update = metadata.modified().unwrap();
                let size = metadata.len();

                current_watched_files.insert(
                    file_path.to_string(),
                    FileInfo {
                        path: file_path.to_string(),
                        size,
                        last_update: last_update.into(),
                        cached_path: None,
                        action: action.clone(),
                        last_upload: None,
                    },
                );
            }
        }

        Ok(())
    }

    fn check_if_file_to_update<'a>(
        &self,
        old_file_info: Option<&'a FileInfo>,
        new_file_info: Option<&'a FileInfo>,
    ) -> bool {
        match (old_file_info, new_file_info) {
            (Some(old), Some(new)) => new.last_update > old.last_update,
            (None, Some(_)) => true,
            _ => false,
        }
    }

    fn check_if_file_to_upload<'a>(
        &self,
        new_size_duration: TimeDelta,
        old_file_info: Option<&'a FileInfo>,
        new_file_info: Option<&'a FileInfo>,
    ) -> FileUploadType {
        match (old_file_info, new_file_info) {
            (Some(old), Some(new)) => match (&old.action, &new.action) {
                (FileAction::Upload, FileAction::None) => {
                    if new.size < old.size {
                        FileUploadType::Old
                    } else {
                        FileUploadType::None
                    }
                }
                (FileAction::Upload, FileAction::Upload) => {
                    if new.last_update == old.last_update
                        && chrono::Utc::now() - new.last_update > new_size_duration
                        && (old.last_upload.is_none() || old.last_upload.unwrap() < new.last_update)
                    {
                        FileUploadType::New
                    } else if new.size < old.size {
                        FileUploadType::Old
                    } else {
                        FileUploadType::None
                    }
                }
                _ => FileUploadType::None,
            },
            (Some(old), None) => match &old.action {
                FileAction::Upload => FileUploadType::Old,
                _ => FileUploadType::None,
            },
            _ => FileUploadType::None,
        }
    }

    pub fn cache_file(&self, file_cache_dir: &str, file_info: &mut FileInfo) -> Result<()> {
        if file_info.cached_path.is_none() {
            let file_name =
                random_string::generate(CACHED_FILE_NAME_LENGTH, CACHED_FILE_NAME_CHARSET);
            file_info.cached_path = Some(
                Path::new(file_cache_dir)
                    .join(file_name)
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
        }
        fs::copy(&file_info.path, file_info.cached_path.as_ref().unwrap())?;
        Ok(())
    }

    pub fn prepare_cache_directory(&self, file_cache_dir: &str) -> Result<()> {
        let path = Path::new(file_cache_dir);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
        fs::create_dir_all(path)?;
        Ok(())
    }

    pub async fn upload_file(
        &self,
        service_url: &str,
        api_key: &str,
        file_info: &FileInfo,
    ) -> Result<()> {
        let logger = Logger::new();
        logger
            .log(&format!("Uploading file: {}", file_info.path), None)
            .await;

        let file_path = file_info.cached_path.as_ref().unwrap_or(&file_info.path);

        upload_from_file_path(
            service_url,
            api_key,
            file_path,
            Path::new(&file_info.path).file_name().unwrap().to_str(),
        )
        .await?;

        Ok(())
    }

    pub async fn poll_files(
        &mut self,
        service_url: &str,
        api_key: &str,
        workflow_directory: &str,
        file_cache_dir: &str,
        new_size_duration: TimeDelta,
    ) -> Result<()> {
        let logger = Logger::new();
        let mut to_upload: Vec<FileInfo> = Vec::new();
        let workflow_path = Path::new(workflow_directory);
        if !workflow_path.exists() {
            logger
                .log(
                    &format!("Workflow directory does not exist - {:?}", workflow_path),
                    None,
                )
                .await;
            return Ok(());
        }

        let mut found_files = HashMap::new();

        for (pattern, action) in FILE_WATCHER_PATTERNS.iter() {
            Self::gather_pattern_from_directory(&mut found_files, workflow_path, pattern, action)
                .await?;
        }

        let paths = found_files.keys().cloned().collect::<Vec<String>>();

        logger.log(&format!("Found files: {:?}", paths), None).await;

        let paths: HashSet<String> = HashSet::from_iter(
            [
                paths,
                self.watched_files.keys().cloned().collect::<Vec<String>>(),
            ]
            .concat(),
        );

        // Upload action processing
        for path in paths {
            let old_file_info = self.watched_files.get(&path);
            let new_file_info = found_files.get_mut(&path);

            let upload_type = self.check_if_file_to_upload(
                new_size_duration,
                old_file_info,
                new_file_info.as_deref(),
            );

            match upload_type {
                FileUploadType::Old => {
                    to_upload.push(old_file_info.unwrap().clone());
                }
                FileUploadType::New => {
                    let new_file_info = new_file_info.unwrap();
                    new_file_info.last_upload = Some(Utc::now());
                    to_upload.push(new_file_info.clone());
                }
                _ => {}
            }
        }

        for file_info in to_upload {
            self.upload_file(service_url, api_key, &file_info).await?;
        }

        for file_info in found_files.values_mut() {
            let old_file_info = self.watched_files.get(&file_info.path);
            let update = self.check_if_file_to_update(old_file_info, Some(file_info));
            if update {
                self.cache_file(file_cache_dir, file_info)?;
            } else if let Some(old_file_info) = old_file_info {
                file_info.cached_path = old_file_info.cached_path.clone();
                file_info.last_upload = if let Some(last_upload) = old_file_info.last_upload {
                    Some(last_upload)
                } else {
                    file_info.last_upload
                };
            }
        }

        self.watched_files = found_files;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Days;

    use super::*;

    #[test]
    fn test_check_if_file_to_update_no_changes() {
        let now: DateTime<Utc> = Utc::now();
        let file_watcher = FileWatcher::new();
        let old_file_info = FileInfo {
            path: "/tmp/test.txt".to_string(),
            size: 50,
            last_update: now.clone(),
            last_upload: Some(now.clone()),
            cached_path: None,
            action: FileAction::None,
        };

        let new_file_info = FileInfo {
            path: "/tmp/test.txt".to_string(),
            size: 50,
            last_update: now.clone(),
            last_upload: Some(now.clone()),
            cached_path: None,
            action: FileAction::None,
        };

        assert!(!file_watcher.check_if_file_to_update(Some(&old_file_info), Some(&new_file_info)));
    }

    #[test]
    fn test_check_if_file_to_update_new_file() {
        let now: DateTime<Utc> = Utc::now();
        let file_watcher = FileWatcher::new();
        let old_file_info = FileInfo {
            path: "/tmp/test.txt".to_string(),
            size: 50,
            last_update: now.clone(),
            last_upload: Some(now.clone()),
            cached_path: None,
            action: FileAction::None,
        };

        let newer = now.checked_add_days(Days::new(1)).unwrap();
        let new_file_info = FileInfo {
            path: "/tmp/test.txt".to_string(),
            size: 50,
            last_update: newer.clone(),
            last_upload: Some(now.clone()),
            cached_path: None,
            action: FileAction::None,
        };

        assert!(file_watcher.check_if_file_to_update(Some(&old_file_info), Some(&new_file_info)));
    }
}
