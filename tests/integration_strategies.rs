use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn setup_env(
) -> Result<(TempDir, std::path::PathBuf, std::path::PathBuf), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config");
    let repo_dir = temp_dir.path().join("repo");

    // Create config dir
    fs::create_dir_all(&config_path)?;

    // Create repo
    fs::create_dir_all(&repo_dir)?;
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_dir)
        .stdout(std::process::Stdio::null())
        .output()?;

    Ok((temp_dir, config_path, repo_dir))
}

fn create_profile(config_path: &std::path::Path, id: &str, name: &str, email: &str) {
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", config_path)
        .args(&["-c", name, email, id])
        .assert()
        .success();
}

fn set_strategy(
    config_path: &std::path::Path,
    strategy: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let toml_path = config_path.join("config.toml");
    println!("DEBUG: [set_strategy] Target: {:?}", toml_path);

    if toml_path.exists() {
        let content = fs::read_to_string(&toml_path)?;
        let mut replaced = false;
        let new_content = content
            .lines()
            .map(|line| {
                if line.trim().starts_with("switch =") {
                    replaced = true;
                    format!("switch = \"{}\"", strategy)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        if !replaced {
            panic!(
                "DEBUG: Failed to find 'switch =' line in config:\n{}",
                content
            );
        }

        fs::write(&toml_path, &new_content)?;

        let verify = fs::read_to_string(&toml_path)?;
        if !verify.contains(&format!("switch = \"{}\"", strategy)) {
            panic!("DEBUG: Verification failed! Content:\n{}", verify);
        }
    } else {
        panic!("DEBUG: config.toml not found at {:?}", toml_path);
    }
    Ok(())
}

fn add_dirty_config(repo_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let git_dir = repo_dir.join(".git");
    let config_file = git_dir.join("config");
    let mut current_config = fs::read_to_string(&config_file)?;
    current_config.push_str(
        r#"
[user]
    name = DirtyUser
    email = dirty@example.com
    extra = StayHere
"#,
    );
    fs::write(&config_file, current_config)?;
    Ok(())
}

#[test]
fn test_strategy_include_lowercase_soft() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, config_path, repo_dir) = setup_env()?;
    create_profile(&config_path, "soft_user", "Soft", "soft@test.com");
    set_strategy(&config_path, "include")?;

    add_dirty_config(&repo_dir)?;

    // Run Switch
    // Run Switch
    let output = Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .current_dir(&repo_dir)
        .arg("soft_user")
        .output()
        .expect("Failed to run naj");

    println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success());

    let git_config = fs::read_to_string(repo_dir.join(".git/config"))?;

    // Check 1: Include is present
    assert!(git_config.contains("path ="));
    assert!(git_config.contains("soft_user.gitconfig"));

    // Check 2: Dirty config is preserved (Soft mode)
    assert!(git_config.contains("name = DirtyUser"));
    assert!(git_config.contains("extra = StayHere"));

    Ok(())
}

#[test]
fn test_strategy_include_uppercase_hard() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, config_path, repo_dir) = setup_env()?;
    create_profile(&config_path, "hard_user", "Hard", "hard@test.com");
    set_strategy(&config_path, "INCLUDE")?;

    add_dirty_config(&repo_dir)?;

    // Run Switch
    // Run Switch
    let output = Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .current_dir(&repo_dir)
        .arg("hard_user")
        .output()
        .expect("Failed to run naj");

    println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success());

    let git_config = fs::read_to_string(repo_dir.join(".git/config"))?;

    // Check 1: Include is present
    assert!(git_config.contains("path ="));
    assert!(git_config.contains("hard_user.gitconfig"));

    // Check 2: Dirty config is REMOVED (Hard mode)
    // The [user] section should be gone
    assert!(!git_config.contains("name = DirtyUser"));
    assert!(!git_config.contains("extra = StayHere"));

    Ok(())
}

#[test]
fn test_strategy_override_lowercase_soft() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, config_path, repo_dir) = setup_env()?;
    create_profile(&config_path, "soft_over", "SoftOver", "so@test.com");
    set_strategy(&config_path, "override")?;

    add_dirty_config(&repo_dir)?;

    // Run Switch
    // Run Switch
    let output = Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .current_dir(&repo_dir)
        .arg("soft_over")
        .output()
        .expect("Failed to run naj");

    println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success());

    let git_config = fs::read_to_string(repo_dir.join(".git/config"))?;

    // Check 1: No include
    assert!(!git_config.contains("path ="));
    assert!(!git_config.contains("soft_over.gitconfig"));

    // Check 2: Values are overwritten directly
    assert!(git_config.contains("name = SoftOver"));
    assert!(git_config.contains("email = so@test.com"));

    // Check 3: Extra keys are preserved (Soft override only touches keys in profile)
    assert!(git_config.contains("extra = StayHere"));

    Ok(())
}

#[test]
fn test_strategy_override_uppercase_hard() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, config_path, repo_dir) = setup_env()?;
    create_profile(&config_path, "hard_over", "HardOver", "ho@test.com");
    set_strategy(&config_path, "OVERRIDE")?;

    add_dirty_config(&repo_dir)?;

    // Run Switch
    // Run Switch
    let output = Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .current_dir(&repo_dir)
        .arg("hard_over")
        .output()
        .expect("Failed to run naj");

    println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success());

    let git_config = fs::read_to_string(repo_dir.join(".git/config"))?;

    // Check 1: No include
    assert!(!git_config.contains("path ="));

    // Check 2: Values are overwritten
    assert!(git_config.contains("name = HardOver"));

    // Check 3: Extra keys are REMOVED (Hard override wipes [user] section)
    assert!(!git_config.contains("extra = StayHere"));
    assert!(!git_config.contains("name = DirtyUser"));

    Ok(())
}
