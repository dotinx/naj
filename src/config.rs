use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Context, Result};
use crate::utils::expand_path;

#[derive(Serialize, Deserialize, Debug)]
pub struct Strategies {
    pub clone: String,
    pub switch: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NajConfig {
    pub strategies: Strategies,
    pub profile_dir: String,
}

impl Default for NajConfig {
    fn default() -> Self {
        NajConfig {
            strategies: Strategies {
                clone: "INCLUDE".to_string(), 
                switch: "include".to_string(), 
            },
            profile_dir: "~/.config/naj/profiles".to_string(),
        }
    }
}

pub fn get_config_root() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("NAJ_CONFIG_PATH") {
        return Ok(PathBuf::from(path));
    }
    let config_dir = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    Ok(config_dir.join("naj"))
}

pub fn load_config() -> Result<NajConfig> {
    let root = get_config_root()?;
    let config_path = root.join("naj.toml");

    if !config_path.exists() {
        return initialize_config(&root, &config_path);
    }

    let content = fs::read_to_string(&config_path).context("Failed to read config file")?;
    let config: NajConfig = toml::from_str(&content).context("Failed to parse config file")?;
    Ok(config)
}

fn initialize_config(root: &Path, config_path: &Path) -> Result<NajConfig> {
    // Ensure root exists
    fs::create_dir_all(root).context("Failed to create config root")?;

    // Determine default profile_dir based on environment to support testing isolation
    let profile_dir_str = if let Ok(env_path) = std::env::var("NAJ_CONFIG_PATH") {
         // If NAJ_CONFIG_PATH is set, default profile dir should be inside it for isolation
         let p = PathBuf::from(env_path).join("profiles");
         // Use forward slashes for TOML consistency if possible, though PathBuf handles it.
         // On Windows, replace backslashes to avoid escape issues in TOML string if not raw
         // We'll just rely on to_string_lossy but be careful with escaping in the format macro
         p.to_string_lossy().to_string()
    } else {
         "~/.config/naj/profiles".to_string()
    };

    // Use toml serialization to ensure string is escaped properly? 
    // Manual format is safer for comments.
    // If path contains backslashes (Windows), we need to escape them for the TOML string literal: "C:\\Foo"
    let escaped_profile_dir = profile_dir_str.replace("\\", "\\\\");

    let generated_toml = format!(r#"# Naj Configuration

profile_dir = "{}"

[strategies]
clone = "INCLUDE" # Hard strategy
switch = "include" # Soft strategy
"#, escaped_profile_dir);

    fs::write(config_path, &generated_toml).context("Failed to write default config")?;
    
    // Create the profiles directory recursively
    let expanded_profile_dir = expand_path(&profile_dir_str)?;
    fs::create_dir_all(&expanded_profile_dir).context("Failed to create profiles directory")?;

    let config: NajConfig = toml::from_str(&generated_toml)?;
    Ok(config)
}
