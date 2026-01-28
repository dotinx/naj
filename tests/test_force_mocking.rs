use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_switch_force_mocking() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config");
    let repo_dir = temp_dir.path().join("repo");
    
    fs::create_dir_all(&repo_dir)?;
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_dir)
        .output()?;

    // Create profile
    Command::new(env!("CARGO_BIN_EXE_gosh"))
        .env("GOSH_CONFIG_PATH", &config_path)
        .args(&["-c", "User", "u@e.com", "mock_test"])
        .assert()
        .success();

    // Run force switch with mocking
    Command::new(env!("CARGO_BIN_EXE_gosh"))
        .env("GOSH_CONFIG_PATH", &config_path)
        .env("GOSH_MOCKING", "1")
        .current_dir(&repo_dir)
        .args(&["mock_test", "-f"])
        .assert()
        .success()
        // Check for dry-run output of cleanup commands
        .stderr(predicates::str::contains("config"))
        .stderr(predicates::str::contains("--remove-section"))
        .stderr(predicates::str::contains("user"));

    Ok(())
}
