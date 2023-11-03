use std::fs::File;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    // run_on_startup: bool,
    pub log_to_file: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { log_to_file: false }
    }
}

pub fn read_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let config_path = "config.json";

    let mut file = match File::open(config_path) {
        Ok(file) => file,
        Err(_) => {
            let default_config = AppConfig::default();
            let config_json = serde_json::to_string_pretty(&default_config)?;
            let mut file = File::create(config_path)?;
            file.write_all(config_json.as_bytes())?;
            file.sync_all()?;
            File::open(config_path)?
        }
    };

    let mut config_content = String::new();
    file.read_to_string(&mut config_content)?;

    let config: AppConfig = serde_json::from_str(&config_content)?;

    Ok(config)
}
