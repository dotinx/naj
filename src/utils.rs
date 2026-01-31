use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub fn expand_path(path_str: &str) -> Result<PathBuf> {
    // Support standard tilde expansion for user convenience,
    // but limit to simple ~/ or ~\ patterns to avoid complex shell expansion logic.
    if path_str.starts_with('~') {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;

        if path_str == "~" {
            return Ok(home);
        }

        if path_str.starts_with("~/") || path_str.starts_with("~\\") {
            let remainder = &path_str[2..];
            return Ok(home.join(remainder));
        }
    }

    Ok(PathBuf::from(path_str))
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! naj_debug {
    ($($arg:tt)*) => {
        if std::env::var("NAJ_DEBUG").is_ok() {
            eprint!("[DEBUG] ");
            eprintln!($($arg)*);
        }
    }
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! naj_debug {
    ($($arg:tt)*) => {};
}
