use crate::utils::expand_path;
use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct Strategies {
    pub clone: String,
    pub switch: SwitchStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchStrategy {
    IncludeSoft,
    IncludeHard,
    OverrideSoft,
    OverrideHard,
}

impl Default for SwitchStrategy {
    fn default() -> Self {
        SwitchStrategy::IncludeSoft
    }
}

impl Serialize for SwitchStrategy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            SwitchStrategy::IncludeSoft => "include",
            SwitchStrategy::IncludeHard => "INCLUDE",
            SwitchStrategy::OverrideSoft => "override",
            SwitchStrategy::OverrideHard => "OVERRIDE",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for SwitchStrategy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Sanitize input to handle potential trailing whitespace from manual edits
        Ok(match s.trim() {
            "INCLUDE" => SwitchStrategy::IncludeHard,
            "OVERRIDE" => SwitchStrategy::OverrideHard,
            "override" => SwitchStrategy::OverrideSoft,
            "include" => SwitchStrategy::IncludeSoft,
            _ => SwitchStrategy::IncludeSoft, // Default fallback
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NajConfig {
    #[serde(default)]
    pub strategies: Strategies,
    pub profile_dir: String,
}

impl Default for NajConfig {
    fn default() -> Self {
        NajConfig {
            strategies: Strategies::default(),
            profile_dir: "~/.config/naj/profiles".to_string(),
        }
    }
}

pub fn get_config_root() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("NAJ_CONFIG_PATH") {
        return Ok(PathBuf::from(path));
    }
    let config_dir =
        dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    Ok(config_dir.join("naj"))
}

pub fn load_config() -> Result<NajConfig> {
    let root = get_config_root()?;
    let config_path = root.join("config.toml");

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

    // Determine default profile_dir based on environment to support testing isolation.
    // When NAJ_CONFIG_PATH is set, we prefer keeping profiles relative to it.
    let profile_dir_str = if let Ok(env_path) = std::env::var("NAJ_CONFIG_PATH") {
        let p = PathBuf::from(env_path).join("profiles");
        p.to_string_lossy().to_string()
    } else {
        "~/.config/naj/profiles".to_string()
    };

    // Manual formatting allows us to include helpful comments in the generated file.
    // On Windows, we must escape backslashes to ensure the TOML string literal is valid.
    let escaped_profile_dir = profile_dir_str.replace("\\", "\\\\");

    let generated_toml = format!(
        r#"# Naj Configuration

profile_dir = "{}"

[strategies]
# include: Include the profile file in the git config
# override: Override the git config with the profile file
# INCLUDE, OVERRIDE: clear the value in the git config and apply config
clone = "INCLUDE" # Hard strategy
switch = "include" # Soft strategy
"#,
        escaped_profile_dir
    );

    fs::write(config_path, &generated_toml).context("Failed to write default config")?;

    // Ensure the profiles directory exists so the user can immediately start adding files
    let expanded_profile_dir = expand_path(&profile_dir_str)?;
    fs::create_dir_all(&expanded_profile_dir).context("Failed to create profiles directory")?;

    let config: NajConfig = toml::from_str(&generated_toml)?;
    Ok(config)
}
