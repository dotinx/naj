use crate::config::NajConfig;
use crate::sanitizer;
use crate::utils::expand_path;
use anyhow::{Result, anyhow, Context};
use std::process::Command;
use std::path::{Path, PathBuf};

enum Action {
    Setup,
    Exec,
    Switch,
}

pub fn run(config: &NajConfig, profile_id: &str, args: &[String], force: bool) -> Result<()> {
    // Determine Action
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

fn run_command(cmd: &mut Command) -> Result<()> {
    if is_mocking() {
        eprintln!("[DRY-RUN] {:?}", cmd);
        return Ok(());
    }

    // inherit stdio
    // But cmd is passed in.
    // The requirement says "The process MUST inherit stdin/stdout/stderr"
    // We assume the caller sets that up or we do it here?
    // Command default is NOT inherit.
    // We should set it before calling this wrapper or inside.
    // Let's set it inside but we need to modify cmd.
    // actually, std::process::Command methods like spawn or status need to be called.
    
    // We can't easily iterate args from Command generic debug, but checking env var is enough.
    // Let's assume the caller constructs the command and we run it.
    
    let status = cmd.status().context("Failed to execute git command")?;
    if !status.success() {
        // We might not want to error hard if git fails, just propagate exit code?
        // But anyhow::Result implies error.
        // For CLI tools, usually we want to return the exact exit code.
        // But for now, let's just return Error if failed.
        return Err(anyhow!("Git command exited with status: {}", status));
    }
    Ok(())
}

fn get_profile_dir(config: &NajConfig) -> Result<PathBuf> {
    expand_path(&config.profile_dir)
}

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
        // Check if the path belongs to our profile directory
        // We use string containment as a heuristic since paths might be canonicalized differently
        // but typically we add absolute paths.
        let is_naj_profile = val.contains(&profile_dir.to_string_lossy().to_string()) 
                              || (val.contains("/profiles/") && val.ends_with(".gitconfig"));

        if is_naj_profile {
            let mut cmd = Command::new("git");
            cmd.args(&["config", "--local", "--unset", "include.path", val]);
            // We tolerate failure here (e.g. if key doesn't exist anymore for some reason)
            if is_mocking() {
                eprintln!("[DRY-RUN] {:?}", cmd);
            } else {
                let _ = cmd.output();
            }
        }
    }
    
    Ok(())
}

fn run_exec(config: &NajConfig, profile_id: &str, args: &[String]) -> Result<()> {
    let profile_path = get_profile_path(config, profile_id)?;
    let injections = sanitizer::BLIND_INJECTIONS;
    
    let mut cmd = Command::new("git");
    
    // Layer 1: Sanitization
    for (k, v) in injections {
        cmd.arg("-c").arg(format!("{}={}", k, v));
    }
    
    // Layer 2: Profile
    cmd.arg("-c").arg(format!("include.path={}", profile_path.to_string_lossy()));
    
    // Layer 3: User Command
    cmd.args(args);
    
    cmd.stdin(std::process::Stdio::inherit())
       .stdout(std::process::Stdio::inherit())
       .stderr(std::process::Stdio::inherit());

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
    // Use absolute path for include to avoid issues if we move around?
    // The requirement just says <PATH_TO_PROFILE>.
    // Usually absolute path is best for git config include.path.
    let abs_profile_path = if profile_path.is_absolute() {
        profile_path
    } else {
        std::env::current_dir()?.join(profile_path)
    };
    
    // Strategy determination
    let strategy = if force {
        "HARD".to_string()
    } else {
        config.strategies.switch.to_uppercase()
    };
    
    if strategy == "HARD" {
        // Remove sections
        let sections = sanitizer::BLACKLIST_SECTIONS;
        for section in sections {
            let mut cmd = Command::new("git");
            cmd.args(&["config", "--remove-section", section]);
            // Ignore errors
            if is_mocking() {
                eprintln!("[DRY-RUN] {:?}", cmd);
            } else {
                let _ = cmd.output(); 
            }
        }
        
        // Unset keys
        let keys = sanitizer::BLACKLIST_KEYS;
        for key in keys {
             let mut cmd = Command::new("git");
             cmd.args(&["config", "--unset", key]);
             if is_mocking() {
                 eprintln!("[DRY-RUN] {:?}", cmd);
             } else {
                 let _ = cmd.output();
             }
        }
    }
    
    // 0. Clean existing naj profiles
    let profiles_dir = get_profile_dir(config)?;
    clean_existing_profiles(&profiles_dir)?;

    // 1. Add new profile
    let path_str = abs_profile_path.to_string_lossy();
    let mut cmd = Command::new("git");
    cmd.args(&["config", "--local", "--add", "include.path", &path_str]);
    run_command(&mut cmd)?;
    println!("Switched to profile '{}'", profile_id);
    
    warn_if_dirty_config(profile_id)?;

    Ok(())
}

fn run_setup(config: &NajConfig, profile_id: &str, args: &[String]) -> Result<()> {
    // 1. Pass raw args to git (no injection)
    let mut cmd = Command::new("git");
    cmd.args(args);
    cmd.stdin(std::process::Stdio::inherit())
       .stdout(std::process::Stdio::inherit())
       .stderr(std::process::Stdio::inherit());
       
    // Check if dry run? setup actions have side effects (creating dirs).
    // If mocking, we print validation and skip real execution of git?
    
    if is_mocking() {
        eprintln!("[DRY-RUN] {:?}", cmd);
        // We can't really continue to infer directory if we don't run it?
        // But for testing logic, we might want to see what happens next.
        // However, if git clone doesn't run, the dir won't exist for switch mode check.
        return Ok(());
    }

    let status = cmd.status().context("Failed to execute git setup command")?;
    if !status.success() {
        return Err(anyhow!("Git setup command failed"));
    }

    // 2. Infer target directory
    // Last arg analysis
    if let Some(last_arg) = args.last() {
        let target_dir = if !is_git_url(last_arg) && args.len() > 1 {
            // Case A: Explicit Directory
            // "If the last argument does not look like a Git URL ... treat it as the Target Directory"
            // Wait, "clone <url> <dir>" -> last arg is dir.
            // "clone <url>" -> last arg is url.
            // So we check if it LOOKS like a URL.
            PathBuf::from(last_arg)
        } else {
            // Case B: Implicit Directory
            // Extract basename
            let url = last_arg; 
            // Remove checks for safety?
             extract_basename(url)
        };
        
        println!("Detected target directory: {:?}", target_dir);
        
        // 3. Switch mode on new directory
        let cwd = std::env::current_dir()?;
        let repo_path = cwd.join(&target_dir);
        
        if repo_path.exists() && repo_path.join(".git").exists() {
            std::env::set_current_dir(&repo_path)?;
            // Force Hard switch
            run_switch(config, profile_id, true)?;
        } else {
             eprintln!("Warning: Could not find repository at {:?} to apply profile", repo_path);
        }
    }

    Ok(())
}

fn is_git_url(s: &str) -> bool {
    s.starts_with("http://") || 
    s.starts_with("https://") || 
    s.starts_with("git@") || 
    s.starts_with("ssh://") ||
    s.contains('@') || // scp-like syntax user@host:path
    s.contains(':')    // scp-like syntax host:path (but http has : too)
    // The requirement says: "(does not start with http://, https://, git@, ssh://, and contains no @ or :)"
}

fn extract_basename(url: &str) -> PathBuf {
    // Remove trailing slash
    let mut s = url.trim_end_matches('/');
    // Remove .git suffix
    if s.ends_with(".git") {
        s = &s[..s.len() - 4];
    }
    // Basename
    let path = Path::new(s);
    path.file_name().map(|n| PathBuf::from(n)).unwrap_or_else(|| PathBuf::from("repo"))
}

fn warn_if_dirty_config(profile_id: &str) -> Result<()> {
    let config_path = Path::new(".git/config");
    if config_path.exists() {
        let content = std::fs::read_to_string(config_path)?;
        
        // Check for sections that typically contain identity or dangerous settings
        // We look for [user], [author], [gpg] sections, or specific keys like sshCommand/gpgsign
        let is_dirty = content.contains("[user]") 
            || content.contains("[author]") 
            || content.contains("[gpg]")
            || content.contains("sshCommand")
            || content.contains("gpgsign");

        if is_dirty {
             println!("\n⚠️  WARNING: Dirty Local Config Detected!");
             println!("   Your .git/config contains hardcoded '[user]' or '[core]' settings.");
             println!("   These settings (like signing keys) are merging with your profile");
             println!("   and causing a \"Frankenstein Identity\".");
             println!("");
             println!("   👉 Fix it: Run 'naj {} -f' to force clean.\n", profile_id);
        }
    }
    Ok(())
}
