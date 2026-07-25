use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use std::io;

mod config;
mod git;
mod manage;
mod sanitizer;
mod utils;

// --- 1. CLI definition (with detailed help docs) ---

#[derive(Parser)]
#[command(name = "naj")]
#[command(version)] // Read version from Cargo.toml
#[command(author)] // Read authors from Cargo.toml
#[command(about = "A secure, idempotent Git identity switcher.")]
#[command(
    long_about = "Naj (/*ŋˤajʔ/ 'I/Me') helps you manage multiple Git identities (Work, Personal, Open Source) without messing up your local config or SSH keys.\n\nIt ensures that the correct email, signing key, and SSH command are used for every commit."
)]
struct Cli {
    /// The Profile ID to switch to (e.g., 'work', 'personal').
    ///
    /// If arguments are provided after this ID, they are passed to git.
    /// Example: `naj work commit -m "fix"`
    #[arg(value_name = "PROFILE_ID")]
    profile_id: Option<String>,

    /// Git arguments to execute immediately after switching.
    ///
    /// If provided, naj runs in 'Exec' mode (temporary switch).
    #[arg(
        value_name = "GIT_ARGS",
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    git_args: Vec<String>,

    /// Create a new profile interactively or with arguments.
    ///
    /// Usage: --create <NAME> <EMAIL> <ID>
    #[arg(short, long, num_args = 3, value_names = ["NAME", "EMAIL", "ID"])]
    create: Option<Vec<String>>,

    /// List all available profiles.
    #[arg(short, long)]
    list: bool,

    /// Remove a profile by ID.
    #[arg(short, long, value_name = "ID")]
    remove: Option<String>,

    /// Force switch strategy (Perform Hard Clean).
    ///
    /// This will aggressively sanitize .git/config (removing [user], [author], etc.)
    /// before applying the profile. Use this if you have "Frankenstein" config.
    #[arg(short, long)]
    force: bool,

    /// Generate shell completion script.
    ///
    /// Usage: source <(naj --completion zsh)
    #[arg(long, value_enum, value_name = "SHELL")]
    completion: Option<Shell>,
}

// --- 2. Main entrypoint ---

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle completion generation first (no config load — fastest path).
    if let Some(shell) = cli.completion {
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut io::stdout());
        return Ok(());
    }

    // Load configuration
    let config = config::load_config()?;

    // Handle list
    if cli.list {
        manage::list_profiles(&config)?;
        return Ok(());
    }

    // Handle create
    if let Some(args) = cli.create {
        if args.len() == 3 {
            manage::create_profile(&config, &args[0], &args[1], &args[2])?;
        }
        return Ok(());
    }

    // Handle remove
    if let Some(id) = cli.remove {
        manage::remove_profile(&config, &id)?;
        return Ok(());
    }

    // Core logic: Switch / Setup / Exec
    if let Some(profile_id) = cli.profile_id {
        // Pass profile_id and the remaining git_args to git::run
        git::run(&config, &profile_id, &cli.git_args, cli.force)?;
    } else {
        // No profile_id and no flag: print help
        if !cli.list && cli.create.is_none() && cli.remove.is_none() && cli.completion.is_none() {
            use clap::CommandFactory;
            Cli::command().print_help()?;
        }
    }

    Ok(())
}
