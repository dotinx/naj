use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub fn expand_path(path_str: &str) -> Result<PathBuf> {
    if path_str.starts_with('~') {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
        
        // Handle "~" exactly
        if path_str == "~" {
            return Ok(home);
        }

        // Handle "~/" or "~\" (windows)
        // We check for the separator to avoid matching "~user" which we don't support simple replacement for
        if path_str.starts_with("~/") || path_str.starts_with("~\\") {
            let remainder = &path_str[2..];
            return Ok(home.join(remainder));
        }
        
        // If it starts with ~ but isn't ~/ or ~\, we might want to support it or error? 
        // For simplicity and matching requirements, we treat it as literal if it doesn't match our expansion pattern
        // Or we could error. Given the requirement is mostly for config paths like "~/.config/...", we assume standard expansion.
    }

    Ok(PathBuf::from(path_str))
}
