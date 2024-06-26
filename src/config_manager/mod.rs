use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

pub const DEFAULT_CONFIG_PATH: &str = ".config/tracer/tracer.toml";

#[derive(Deserialize)]
pub struct ConfigFile {
    pub api_key: String,
    pub polling_interval_ms: u64,
    pub targets: Vec<String>,
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn load_config() -> Result<ConfigFile> {
        let config_path = std::env::var("TRACER_CONFIG").unwrap_or_else(|_| {
            let home_dir = std::env::var("HOME").unwrap();
            PathBuf::from(home_dir)
                .join(DEFAULT_CONFIG_PATH)
                .to_string_lossy()
                .to_string()
        });

        println!("Loading config from: {}", config_path); // Debug statement
        let config_content = std::fs::read_to_string(&config_path)?;
        println!("Config content: {}", config_content); // Debug statement

        let config: ConfigFile = toml::from_str(&config_content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    fn create_test_config(content: &str, path: &str) {
        let mut file = File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_load_valid_config() {
        let test_config_path = "/tmp/test_tracer.toml";
        let config_content = r#"
            api_key = "test_api_key"
            polling_interval_ms = 1000
            targets = ["target1", "target2"]
        "#;
        create_test_config(config_content, test_config_path);

        env::set_var("TRACER_CONFIG", test_config_path);
        let config = ConfigManager::load_config().unwrap();
        env::remove_var("TRACER_CONFIG");

        assert_eq!(config.api_key.trim(), "test_api_key");
        assert_eq!(config.polling_interval_ms, 1000);
        assert_eq!(
            config.targets,
            vec!["target1".to_string(), "target2".to_string()]
        );
    }

    #[test]
    fn test_load_default_config_path() {
        let home_dir = env::var("HOME").unwrap();
        let default_config_path = PathBuf::from(home_dir).join(DEFAULT_CONFIG_PATH);
        let config_content = r#"
            api_key = "test_api_key"
            polling_interval_ms = 1000
            targets = ["target1", "target2"]
        "#;
        create_test_config(config_content, default_config_path.to_str().unwrap());

        let config = ConfigManager::load_config().unwrap();

        assert_eq!(config.api_key.trim(), "test_api_key");
        assert_eq!(config.polling_interval_ms, 1000);
        assert_eq!(
            config.targets,
            vec!["target1".to_string(), "target2".to_string()]
        );
    }
}
