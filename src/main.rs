mod cli;
mod config;
mod manage;
mod sanitizer;
mod git;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::load_config()?;

    if let Some(manage_flags) = &cli.manage {
        if let Some(create_args) = &manage_flags.create {
            manage::create_profile(&config, &create_args[0], &create_args[1], &create_args[2])?;
        } else if let Some(id) = &manage_flags.remove {
            manage::remove_profile(&config, id)?;
        } else if let Some(id) = &manage_flags.edit {
            manage::edit_profile(&config, id)?;
        } else if manage_flags.list {
            manage::list_profiles(&config)?;
        }
    } else {
        // Operation mode
        let profile_id = cli.profile_id.as_ref().expect("Profile ID required in operation mode");
        let git_args = &cli.git_args;
        git::run(&config, profile_id, git_args, cli.force)?;
    }

    Ok(())
}
