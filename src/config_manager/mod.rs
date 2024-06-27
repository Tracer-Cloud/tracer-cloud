use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

pub const DEFAULT_CONFIG_PATH: &str = ".config/tracer/tracer.toml";

#[derive(Deserialize, Clone, Debug)]
pub struct ConfigFile {
    pub api_key: String,
    pub process_polling_interval_ms: u64,
    pub batch_submission_interval_ms: u64,
    pub service_url: String,
    pub targets: Vec<String>,
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn load_config() -> Result<ConfigFile> {
        Self::load_config_with_home(None)
    }

    pub fn load_config_with_home(home_dir: Option<PathBuf>) -> Result<ConfigFile> {
        let config_path = std::env::var("TRACER_CONFIG").unwrap_or_else(|_| {
            let home = home_dir.unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap()));
            home.join(DEFAULT_CONFIG_PATH).to_string_lossy().to_string()
        });

        println!("Loading config from: {}", config_path);
        let config_content = std::fs::read_to_string(&config_path)?;
        println!("Config content: {}", config_content);

        let config: ConfigFile = toml::from_str(&config_content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    const CONFIG_CONTENT: &str = r#"
        api_key = "bwoaLKcVG-k8obNcgt-a9"
        process_polling_interval_ms = 200
        batch_submission_interval_ms = 5000
        service_url = "https://app.tracer.bio/api/data-collector-api"
        targets = ["target1", "target2"]
    "#;

    fn create_test_config(content: &str, path: &str) {
        let mut file = File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_load_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let test_config_path = temp_dir.path().join("test_tracer.toml");
        create_test_config(CONFIG_CONTENT, test_config_path.to_str().unwrap());

        env::set_var("TRACER_CONFIG", test_config_path.to_str().unwrap());
        let config = ConfigManager::load_config().unwrap();
        env::remove_var("TRACER_CONFIG");

        assert_eq!(config.process_polling_interval_ms, 200);
        assert_eq!(config.batch_submission_interval_ms, 5000);
        assert_eq!(
            config.service_url.trim(),
            "https://app.tracer.bio/api/data-collector-api"
        );
        assert_eq!(
            config.targets,
            vec!["target1".to_string(), "target2".to_string()]
        );
    }

    #[test]
    fn test_load_default_config_path() {
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().to_path_buf();
        let config_dir = home_dir.join(".config").join("tracer");
        fs::create_dir_all(&config_dir).unwrap();

        let config_path = config_dir.join("tracer.toml");
        create_test_config(CONFIG_CONTENT, config_path.to_str().unwrap());

        let config = ConfigManager::load_config_with_home(Some(home_dir)).unwrap();

        assert_eq!(config.process_polling_interval_ms, 200);
        assert_eq!(config.batch_submission_interval_ms, 5000);
        assert_eq!(
            config.service_url.trim(),
            "https://app.tracer.bio/api/data-collector-api"
        );
        assert_eq!(
            config.targets,
            vec!["target1".to_string(), "target2".to_string()]
        );
    }
}
