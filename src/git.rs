use crate::config::{NajConfig, SwitchStrategy};
use crate::naj_debug;
use crate::sanitizer;
use crate::utils::expand_path;
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

enum Action {
    Setup,
    Exec,
    Switch,
}

pub fn run(config: &NajConfig, profile_id: &str, args: &[String], force: bool) -> Result<()> {
    let action = if args.is_empty() {
        Action::Switch
    } else if args[0] == "clone" || args[0] == "init" {
        Action::Setup
    } else {
        Action::Exec
    };

    match action {
        Action::Exec => run_exec(config, profile_id, args),
        Action::Switch => run_switch(config, profile_id, force),
        Action::Setup => run_setup(config, profile_id, args),
    }
}

// Helper to construct the full path to a profile's .gitconfig file.
fn get_profile_path(config: &NajConfig, id: &str) -> Result<PathBuf> {
    let profile_dir = expand_path(&config.profile_dir)?;
    let p = profile_dir.join(format!("{}.gitconfig", id));
    if !p.exists() {
        return Err(anyhow!("Profile '{}' not found at {:?}", id, p));
    }
    Ok(p)
}

fn is_mocking() -> bool {
    std::env::var("NAJ_MOCKING").is_ok()
}

// Execution helper that handles dry-runs during testing.
fn run_command(cmd: &mut Command) -> Result<()> {
    if is_mocking() {
        eprintln!("[DRY-RUN] {:?}", cmd);
        return Ok(());
    }
    let status = cmd.status().context("Failed to execute git command")?;
    if !status.success() {
        return Err(anyhow!("Git command exited with status: {}", status));
    }
    Ok(())
}

fn get_profile_dir(config: &NajConfig) -> Result<PathBuf> {
    expand_path(&config.profile_dir)
}

// Locates and removes existing Naj profile inclusions from the local git config
// to prevent configuration pollution or conflicts.
fn clean_existing_profiles(profile_dir: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(&["config", "--local", "--get-all", "include.path"])
        .output()?;

    if !output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let val = line.trim();
        let path_obj = Path::new(val);
        let match_path = val.contains(&profile_dir.to_string_lossy().to_string())
            || (val.contains("/profiles/") && val.ends_with(".gitconfig"));
        let match_name = path_obj
            .file_name()
            .map(|n| n.to_string_lossy().ends_with(".gitconfig"))
            .unwrap_or(false);

        if match_path || match_name {
            let mut cmd = Command::new("git");
            cmd.args(&["config", "--local", "--unset", "include.path", val]);
            if is_mocking() {
                eprintln!("[DRY-RUN] {:?}", cmd);
            } else {
                let _ = cmd.output();
            }
        }
    }
    Ok(())
}

fn apply_profile_override(profile_path: &Path) -> Result<()> {
    // Use git config -f to read values directly from the file, bypassing
    // any environment or global overrides for consistency.
    let output = Command::new("git")
        .args(&["config", "-f", &profile_path.to_string_lossy(), "--list"])
        .output()
        .with_context(|| format!("Failed to read profile config from {:?}", profile_path))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Git config read failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // For Override strategies, we manually inject values into the local config
    // to strictly enforce the profile's settings.
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let mut cmd = Command::new("git");
            cmd.args(&["config", "--local", key, value]);
            run_command(&mut cmd)?;
        }
    }

    Ok(())
}

fn run_exec(config: &NajConfig, profile_id: &str, args: &[String]) -> Result<()> {
    let profile_path = get_profile_path(config, profile_id)?;
    let mut cmd = Command::new("git");

    // 1. Sensitize defaults to prevent leakages if not explicitly covered by the profile
    for (k, v) in sanitizer::BLIND_INJECTIONS {
        cmd.args(&["-c", &format!("{}={}", k, v)]);
    }

    // 2. Attach profile via git's native include path for most operations
    cmd.args(&[
        "-c",
        &format!("include.path={}", profile_path.to_string_lossy()),
    ]);

    // 3. Force-inject profile values to ensure they override any local config
    // that might conflict with the base inclusion.
    if let Ok(entries) = read_profile_config(&profile_path) {
        for (k, v) in entries {
            cmd.args(&["-c", &format!("{}={}", k, v)]);
        }
    }

    cmd.args(args);
    run_command(&mut cmd)
}

fn run_switch(config: &NajConfig, profile_id: &str, force: bool) -> Result<()> {
    let status = Command::new("git")
        .args(&["rev-parse", "--is-inside-work-tree"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    let is_git_repo = status.map(|s| s.success()).unwrap_or(false);
    if !is_git_repo {
        return Err(anyhow!("Not a git repository"));
    }

    let profile_path = get_profile_path(config, profile_id)?;
    let abs_profile_path = if profile_path.is_absolute() {
        profile_path
    } else {
        std::env::current_dir()?.join(profile_path)
    };

    // 1. Resolve Effective Strategy
    let base_strategy = config.strategies.switch;
    let effective_strategy = match (force, base_strategy) {
        (true, SwitchStrategy::IncludeSoft) => SwitchStrategy::IncludeHard,
        (true, SwitchStrategy::OverrideSoft) => SwitchStrategy::OverrideHard,
        (false, s) => s,
        _ => SwitchStrategy::IncludeHard, // Fallback
    };

    // Log the resolved strategy for trace visibility in debug mode
    naj_debug!(
        "Strategy Resolution: Base={:?}, Force={}, Effective={:?}",
        base_strategy,
        force,
        effective_strategy
    );

    // Hard strategies require a clean slate to ensure security and privacy
    let should_sanitize = matches!(
        effective_strategy,
        SwitchStrategy::IncludeHard | SwitchStrategy::OverrideHard
    );

    naj_debug!("Should Sanitize? {}", should_sanitize);

    if should_sanitize {
        // Remove sections
        let sections = sanitizer::BLACKLIST_SECTIONS;
        for section in sections {
            let mut cmd = Command::new("git");

            // Explicitly target local config and dereference section name for type safety
            cmd.args(&["config", "--local", "--remove-section", *section]);

            if is_mocking() {
                eprintln!("[DRY-RUN] {:?}", cmd);
            } else {
                naj_debug!("Executing sanitize: {:?}", cmd);
                let output = cmd
                    .output()
                    .context(format!("Failed to attempt removing section {}", section))?;

                // Exit code 1 means "section not found" (benign).
                // We only care about other errors.
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    // Git exit code 1 usually means a section or key was not found,
                    // which is expected if the config is already clean.
                    let is_benign =
                        output.status.code() == Some(1) || stderr.contains("no such section");

                    if !is_benign {
                        return Err(anyhow!(
                            "Failed to remove section '{}': {}",
                            section,
                            stderr.trim()
                        ));
                    }
                }
            }
        }

        // Wipe 'include' section to prevent residual profile links in Hard mode
        let mut cmd = Command::new("git");
        cmd.args(&["config", "--local", "--remove-section", "include"]);
        if is_mocking() {
            eprintln!("[DRY-RUN] {:?}", cmd);
        } else {
            // Success is not critical here as a missing section is already 'clean'
            let _ = cmd.output();
        }

        // Unset keys
        let keys = sanitizer::BLACKLIST_KEYS;
        for key in keys {
            let mut cmd = Command::new("git");
            cmd.args(&["config", "--local", "--unset-all", *key]); // 👈 deref here too

            if is_mocking() {
                eprintln!("[DRY-RUN] {:?}", cmd);
            } else {
                let _ = cmd.output();
            }
        }
    }

    // Clean orphaned Naj profile references before applying a new one
    let profiles_dir = get_profile_dir(config)?;
    clean_existing_profiles(&profiles_dir)?;

    match effective_strategy {
        SwitchStrategy::IncludeSoft | SwitchStrategy::IncludeHard => {
            let path_str = abs_profile_path.to_string_lossy();
            let mut cmd = Command::new("git");
            cmd.args(&["config", "--local", "--add", "include.path", &path_str]);
            run_command(&mut cmd)?;
        }
        SwitchStrategy::OverrideSoft | SwitchStrategy::OverrideHard => {
            apply_profile_override(&abs_profile_path)?;
        }
    }
    println!("Switched to profile '{}'", profile_id);

    warn_if_dirty_config(profile_id, effective_strategy)?;

    Ok(())
}

fn run_setup(config: &NajConfig, profile_id: &str, args: &[String]) -> Result<()> {
    // Execute the base command (init/clone) before applying Naj customization
    let mut cmd = Command::new("git");
    cmd.args(args);
    run_command(&mut cmd)?;

    if args.is_empty() {
        return Ok(());
    }

    let command = &args[0];

    // 2. Switch context if needed
    if command == "init" {
        // Init happens in current dir
        run_switch(config, profile_id, false)?;
    } else if command == "clone" {
        // Parse target directory from clone arguments while ignoring flags (e.g., --depth)
        let mut url = None;
        let mut explicit_dir = None;

        // Skip 'clone'
        for arg in args.iter().skip(1) {
            if arg.starts_with('-') {
                continue;
            }
            if url.is_none() {
                url = Some(arg);
            } else if explicit_dir.is_none() {
                explicit_dir = Some(arg);
            }
        }

        let target_dir = if let Some(dir) = explicit_dir {
            PathBuf::from(dir)
        } else if let Some(u) = url {
            extract_basename(u)
        } else {
            // Default name if it cannot be inferred from the URL
            PathBuf::from("repo")
        };

        if target_dir.exists() && target_dir.is_dir() {
            std::env::set_current_dir(&target_dir)?;
            run_switch(config, profile_id, false)?;
        }
    }

    Ok(())
}

fn read_profile_config(profile_path: &Path) -> Result<Vec<(String, String)>> {
    let output = Command::new("git")
        .args(&["config", "-f", &profile_path.to_string_lossy(), "--list"])
        .output()
        .with_context(|| format!("Failed to read profile config from {:?}", profile_path))?;

    if !output.status.success() {
        return Err(anyhow!("Git config read failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=') {
            entries.push((key.to_string(), value.to_string()));
        }
    }
    Ok(entries)
}

fn extract_basename(url: &str) -> PathBuf {
    let mut s = url.trim_end_matches('/');
    if s.ends_with(".git") {
        s = &s[..s.len() - 4];
    }
    Path::new(s)
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("repo"))
}

fn warn_if_dirty_config(_profile_id: &str, strategy: SwitchStrategy) -> Result<()> {
    // Check for local configuration that might leak identity or signing info
    // when using an 'Include' strategy.
    let config_path = Path::new(".git/config");
    if config_path.exists() {
        let content = std::fs::read_to_string(config_path)?;
        let check_user_block = matches!(
            strategy,
            SwitchStrategy::IncludeSoft | SwitchStrategy::IncludeHard
        );
        let mut is_dirty = false;
        if check_user_block && (content.contains("[user]") || content.contains("[author]")) {
            is_dirty = true;
        }
        if content.contains("[gpg]")
            || content.contains("sshCommand")
            || content.contains("gpgsign")
        {
            is_dirty = true;
        }
        if is_dirty {
            println!("\n⚠️  WARNING: Dirty Local Config Detected!");
            // ...
        }
    }
    Ok(())
}
