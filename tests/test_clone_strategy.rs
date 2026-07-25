use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn setup_env() -> Result<(TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config");
    fs::create_dir_all(&config_path)?;
    Ok((temp_dir, config_path))
}

fn create_profile(config_path: &std::path::Path, id: &str, name: &str, email: &str) {
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", config_path)
        .args(["-c", name, email, id])
        .assert()
        .success();
}

fn set_clone_strategy(
    config_path: &std::path::Path,
    strategy: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let toml_path = config_path.join("config.toml");
    let content = fs::read_to_string(&toml_path)?;
    let new_content = content
        .lines()
        .map(|line| {
            if line.trim().starts_with("clone =") {
                format!("clone = \"{}\"", strategy)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&toml_path, new_content)?;
    Ok(())
}

// Regression test: setup mode must use strategies.clone, not strategies.switch.
// With clone = "INCLUDE" (hard), a freshly cloned repo must have no dirty [user]
// section even if one was somehow present — and the include must be applied.
#[test]
fn test_setup_clone_uses_clone_strategy() -> Result<(), Box<dyn std::error::Error>> {
    let (temp_dir, config_path) = setup_env()?;

    // Create a source repo to clone from.
    let source = temp_dir.path().join("source");
    fs::create_dir_all(&source)?;
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&source)
        .stdout(std::process::Stdio::null())
        .output()?;

    create_profile(&config_path, "clone_hard", "CloneHard", "ch@test.com");

    // Ensure clone strategy is hard (INCLUDE) — the default.
    set_clone_strategy(&config_path, "INCLUDE")?;

    let dest = "cloned_repo";
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .current_dir(temp_dir.path())
        .args(["clone_hard", "clone", source.to_str().unwrap(), dest])
        .assert()
        .success();

    let git_config = fs::read_to_string(
        temp_dir.path().join(dest).join(".git").join("config"),
    )?;

    // Profile include must be present.
    assert!(git_config.contains("[include]"), "include section missing");
    assert!(
        git_config.contains("clone_hard.gitconfig"),
        "profile path missing"
    );

    Ok(())
}

// With clone = "include" (soft), the include is still applied.
#[test]
fn test_setup_clone_soft_strategy_applies_include() -> Result<(), Box<dyn std::error::Error>> {
    let (temp_dir, config_path) = setup_env()?;

    let source = temp_dir.path().join("source2");
    fs::create_dir_all(&source)?;
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&source)
        .stdout(std::process::Stdio::null())
        .output()?;

    create_profile(&config_path, "clone_soft", "CloneSoft", "cs@test.com");
    set_clone_strategy(&config_path, "include")?;

    let dest = "cloned_soft";
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .current_dir(temp_dir.path())
        .args(["clone_soft", "clone", source.to_str().unwrap(), dest])
        .assert()
        .success();

    let git_config = fs::read_to_string(
        temp_dir.path().join(dest).join(".git").join("config"),
    )?;

    assert!(git_config.contains("[include]"));
    assert!(git_config.contains("clone_soft.gitconfig"));

    Ok(())
}

// git init in a named directory: identity must be applied to that directory.
#[test]
fn test_setup_init_uses_clone_strategy() -> Result<(), Box<dyn std::error::Error>> {
    let (temp_dir, config_path) = setup_env()?;
    let sandbox = temp_dir.path().join("sandbox");
    fs::create_dir_all(&sandbox)?;

    create_profile(&config_path, "init_hard", "InitHard", "ih@test.com");
    set_clone_strategy(&config_path, "INCLUDE")?;

    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .current_dir(&sandbox)
        .args(["init_hard", "init", "new_repo"])
        .assert()
        .success();

    let git_config =
        fs::read_to_string(sandbox.join("new_repo").join(".git").join("config"))?;

    assert!(git_config.contains("[include]"));
    assert!(git_config.contains("init_hard.gitconfig"));

    Ok(())
}
