use anyhow::{anyhow, bail, Result};
use std::path::{Path, PathBuf};

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

// Profile IDs become filenames (`{id}.gitconfig`), so they must be strict
// single-component names: 1-64 chars, starting with a letter or digit,
// followed by letters, digits, '.', '_' or '-'. The whitelist rejects
// '/' and '\' outright, and the first-character rule rejects "." / ".."
// and flag-looking names, eliminating path traversal via create/remove/exec.
pub fn validate_profile_id(id: &str) -> Result<()> {
    const MAX_LEN: usize = 64;

    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        bail!("Invalid profile ID: must not be empty");
    };
    if !first.is_ascii_alphanumeric() {
        bail!(
            "Invalid profile ID '{}': must start with a letter or digit",
            id
        );
    }
    if id.len() > MAX_LEN {
        bail!("Invalid profile ID '{}': must be at most {} bytes", id, MAX_LEN);
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
        bail!(
            "Invalid profile ID '{}': only letters, digits, '.', '_' and '-' are allowed",
            id
        );
    }
    Ok(())
}

// Build `{profile_dir}/{id}.gitconfig` after validating the ID. Does not
// check existence: profile creation requires the path to NOT exist yet.
pub fn profile_path(profile_dir: &Path, id: &str) -> Result<PathBuf> {
    validate_profile_id(id)?;
    Ok(profile_dir.join(format!("{}.gitconfig", id)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_profile_id_accepts_normal_ids() {
        for id in ["work", "personal", "a.b_c-d", "0", "OSS", "x"] {
            assert!(validate_profile_id(id).is_ok(), "should accept: {}", id);
        }
    }

    #[test]
    fn validate_profile_id_rejects_traversal_and_separators() {
        for id in ["../evil", "..", "a/b", "a\\b", "/abs", "foo/../bar"] {
            assert!(validate_profile_id(id).is_err(), "should reject: {}", id);
        }
    }

    #[test]
    fn validate_profile_id_rejects_bad_shapes() {
        // Empty, dot-prefixed (hidden / "." / ".."), dash-prefixed (flag-looking)
        for id in ["", ".hidden", "-f", "--force"] {
            assert!(validate_profile_id(id).is_err(), "should reject: {:?}", id);
        }
        // Overlong
        let long = "a".repeat(65);
        assert!(validate_profile_id(&long).is_err());
    }

    #[test]
    fn profile_path_stays_inside_profile_dir() {
        let dir = Path::new("/tmp/naj-profiles");
        let p = profile_path(dir, "work").unwrap();
        assert_eq!(p, dir.join("work.gitconfig"));
        assert!(profile_path(dir, "../escape").is_err());
    }
}
