use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_initialization() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config");

    // Run naj with NAJ_CONFIG_PATH set to temp dir
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_naj"));
    cmd.env("NAJ_CONFIG_PATH", &config_path)
        .arg("-l") // Trigger config load using list flag (not positional "list" profile)
        .assert()
        .success();

    // Verify config file exists
    assert!(config_path.join("config.toml").exists());
    assert!(config_path.join("profiles").exists());

    Ok(())
}

#[test]
fn test_profile_creation_and_listing() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_naj"));
    cmd.env("NAJ_CONFIG_PATH", config_path)
        .args(&["-c", "Test User", "test@example.com", "test_user"])
        .assert()
        .success();

    // Verify profile file
    let profile_path = config_path.join("profiles").join("test_user.gitconfig");
    assert!(profile_path.exists());
    let content = fs::read_to_string(profile_path)?;
    assert!(content.contains("name = Test User"));
    assert!(content.contains("email = test@example.com"));

    // Verify list
    let mut cmd_list = Command::new(env!("CARGO_BIN_EXE_naj"));
    cmd_list
        .env("NAJ_CONFIG_PATH", config_path)
        .arg("-l")
        .assert()
        .success()
        .stdout(predicates::str::contains("test_user"));

    Ok(())
}

#[test]
fn test_duplicate_creation_failure() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path();

    // Create first
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", config_path)
        .args(&["-c", "User", "u@e.com", "dup_test"])
        .assert()
        .success();

    // Create duplicate
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", config_path)
        .args(&["-c", "User2", "u2@e.com", "dup_test"])
        .assert()
        .failure(); // Should fail

    Ok(())
}

#[test]
fn test_remove_profile() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path();
    let profile_path = config_path.join("profiles").join("rem_test.gitconfig");

    // Create
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", config_path)
        .args(&["-c", "User", "u@e.com", "rem_test"])
        .assert()
        .success();
    assert!(profile_path.exists());

    // Remove
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", config_path)
        .args(&["-r", "rem_test"])
        .assert()
        .success();
    assert!(!profile_path.exists());

    // Remove non-existent
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", config_path)
        .args(&["-r", "rem_test"])
        .assert()
        .failure();

    Ok(())
}

#[test]
fn test_exec_dry_run_injection_strict() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path();

    // Create a profile first
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", config_path)
        .args(&["-c", "Test", "test@e.com", "p1"])
        .assert()
        .success();

    // Run exec with mocking
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_naj"));
    cmd.env("NAJ_CONFIG_PATH", config_path)
        .env("NAJ_MOCKING", "1")
        .args(&["p1", "commit", "-m", "foo"])
        .assert()
        .success()
        .stderr(predicates::str::contains("user.name="))
        .stderr(predicates::str::contains("user.email="))
        .stderr(predicates::str::contains("user.signingkey="))
        .stderr(predicates::str::contains("core.sshCommand="))
        .stderr(predicates::str::contains("commit.gpgsign=false"))
        .stderr(predicates::str::contains("include.path="))
        .stderr(predicates::str::contains("p1.gitconfig"));

    Ok(())
}

#[test]
fn test_switch_mode_persistent() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config");
    let repo_dir = temp_dir.path().join("repo");
    let git_dir = repo_dir.join(".git");

    fs::create_dir_all(&repo_dir)?;

    // Init valid git repo
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_dir)
        .output()?;

    // Create profile
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .args(&["-c", "Switch User", "s@e.com", "switch_test"])
        .assert()
        .success();

    // Switch
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .current_dir(&repo_dir)
        .arg("switch_test")
        .assert()
        .success();

    // Verify .git/config
    let git_config = fs::read_to_string(git_dir.join("config"))?;
    assert!(git_config.contains("[include]"));
    assert!(git_config.contains("path ="));
    assert!(git_config.contains("switch_test.gitconfig"));

    Ok(())
}

#[test]
fn test_switch_force_mode_sanitization() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config");
    let repo_dir = temp_dir.path().join("repo");
    let git_dir = repo_dir.join(".git");

    fs::create_dir_all(&repo_dir)?;
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_dir)
        .output()?;

    // Pre-fill with dirty data
    // We append to the config created by git init
    let config_file = git_dir.join("config");
    let mut current_config = fs::read_to_string(&config_file)?;
    current_config.push_str(
        r#"[user]
    name = OldName
    email = old@example.com
"#,
    );
    fs::write(&config_file, current_config)?;

    // Create profile
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .args(&["-c", "Force User", "f@e.com", "force_test"])
        .assert()
        .success();

    // Force Switch
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .current_dir(&repo_dir)
        .args(&["force_test", "-f"])
        .assert()
        .success();

    // Verify .git/config
    let git_config = fs::read_to_string(git_dir.join("config"))?;

    // Should NOT contain user section
    assert!(!git_config.contains("[user]"));
    assert!(!git_config.contains("OldName"));

    // Should contain include
    assert!(git_config.contains("[include]"));
    assert!(git_config.contains("force_test.gitconfig"));

    Ok(())
}

#[test]
fn test_setup_mode_local_clone() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config");
    let source_repo = temp_dir.path().join("source");

    // 1. Create a dummy source repo locally
    fs::create_dir_all(&source_repo)?;
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&source_repo)
        .output()?;
    // Git allows cloning an empty repo, so this is enough

    // 2. Create Profile
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .args(&["-c", "CloneUser", "c@e.com", "clone_test"])
        .assert()
        .success();

    // 3. Run Naj Clone (Setup Mode)
    // We clone from local source to a folder named "dest_repo"
    // Naj should:
    // a) Run git clone
    // b) Infer directory is "dest_repo"
    // c) Run switch logic on "dest_repo"
    let dest_repo_name = "dest_repo";

    Command::new(env!("CARGO_BIN_EXE_naj"))
        .env("NAJ_CONFIG_PATH", &config_path)
        .current_dir(temp_dir.path()) // Execute in temp root
        .args(&[
            "clone_test",
            "clone",
            source_repo.to_str().unwrap(),
            dest_repo_name,
        ])
        .assert()
        .success();

    // 4. Verify Result
    let dest_git_config = temp_dir
        .path()
        .join(dest_repo_name)
        .join(".git")
        .join("config");

    assert!(dest_git_config.exists(), "Cloned repo config should exist");

    let content = fs::read_to_string(dest_git_config)?;
    // Verify that naj automatically switched the profile after cloning
    assert!(content.contains("[include]"));
    assert!(content.contains("clone_test.gitconfig"));

    Ok(())
}
