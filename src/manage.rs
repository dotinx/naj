use crate::config::NajConfig;
use crate::utils::expand_path;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_profile_path(config: &NajConfig, id: &str) -> Result<PathBuf> {
    let profile_dir = expand_path(&config.profile_dir)?;
    Ok(profile_dir.join(format!("{}.gitconfig", id)))
}

pub fn create_profile(config: &NajConfig, name: &str, email: &str, id: &str) -> Result<()> {
    let file_path = get_profile_path(config, id)?;

    if file_path.exists() {
        bail!("Profile '{}' already exists", id);
    }

    // Ensure dir exists (it typically should from init, but good to be safe)
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = format!(
        "[user]\n    name = {}\n    email = {}\n    # signingkey = \n",
        name, email
    );
    fs::write(&file_path, content).with_context(|| format!("Failed to create profile {}", id))?;
    println!("Created profile '{}'", id);
    Ok(())
}

pub fn remove_profile(config: &NajConfig, id: &str) -> Result<()> {
    let file_path = get_profile_path(config, id)?;

    if !file_path.exists() {
        bail!("Profile '{}' does not exist", id);
    }

    fs::remove_file(&file_path).with_context(|| format!("Failed to remove profile {}", id))?;
    println!("Removed profile '{}'", id);
    Ok(())
}

#[allow(dead_code)]
pub fn edit_profile(config: &NajConfig, id: &str) -> Result<()> {
    let file_path = get_profile_path(config, id)?;

    if !file_path.exists() {
        bail!("Profile '{}' does not exist", id);
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    let status = Command::new(&editor)
        .arg(&file_path)
        .status()
        .with_context(|| format!("Failed to launch editor '{}'", editor))?;

    if !status.success() {
        bail!("Editor exited with non-zero status");
    }
    Ok(())
}

pub fn list_profiles(config: &NajConfig) -> Result<()> {
    let profile_dir = expand_path(&config.profile_dir)?;

    if !profile_dir.exists() {
        println!(
            "No profiles found (directory {:?} does not exist)",
            profile_dir
        );
        return Ok(());
    }

    for entry in fs::read_dir(profile_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "gitconfig") {
            if let Some(stem) = path.file_stem() {
                println!("{}", stem.to_string_lossy());
            }
        }
    }
    Ok(())
}
