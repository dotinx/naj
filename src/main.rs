use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use std::io;

mod config;
mod git;
mod manage;
mod sanitizer;
mod utils;

// --- 1. 定义 CLI 结构体 (带详细文档) ---

#[derive(Parser)]
#[command(name = "naj")]
#[command(version)] // 自动从 Cargo.toml 读取版本
#[command(author = "Ringo")]
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

// --- 2. Main 函数 ---

fn main() -> Result<()> {
    let cli = Cli::parse();

    // 🚀 优先处理补全生成 (不加载配置，速度最快)
    if let Some(shell) = cli.completion {
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut io::stdout());
        return Ok(());
    }

    // 加载配置
    let config = config::load_config()?;

    // 处理 List
    if cli.list {
        manage::list_profiles(&config)?;
        return Ok(());
    }

    // 处理 Create
    if let Some(args) = cli.create {
        if args.len() == 3 {
            manage::create_profile(&config, &args[0], &args[1], &args[2])?;
        }
        return Ok(());
    }

    // 处理 Remove
    if let Some(id) = cli.remove {
        manage::remove_profile(&config, &id)?;
        return Ok(());
    }

    // 处理核心逻辑: Switch / Setup / Exec
    if let Some(profile_id) = cli.profile_id {
        // 把 profile_id 和剩下的 git_args 传给 git::run
        git::run(&config, &profile_id, &cli.git_args, cli.force)?;
    } else {
        // 如果没有 profile_id 也没有 flag，打印帮助
        if !cli.list && cli.create.is_none() && cli.remove.is_none() && cli.completion.is_none() {
            use clap::CommandFactory;
            Cli::command().print_help()?;
        }
    }

    Ok(())
}
