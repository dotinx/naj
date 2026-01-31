// Sections that often contain personal identity or security settings (e.g. GPG)
// and should be cleared during Hard strategy switches for privacy.
pub const BLACKLIST_SECTIONS: &[&str] = &["user", "author", "committer", "gpg"];

// Specific keys that might leak sensitive paths or enforce signing protocols
// that should be disabled when switching to a different project context.
pub const BLACKLIST_KEYS: &[&str] = &[
    "core.sshCommand",
    "commit.gpgsign",
    "tag.gpgsign",
    "http.cookieFile",
];

// Safety defaults to prevent unintended leaks of the global system identity
// if a profile is incomplete or improperly configured.
#[allow(dead_code)]
pub const BLIND_INJECTIONS: &[(&str, &str)] = &[
    ("user.name", ""),
    ("user.email", ""),
    ("user.signingkey", ""),
    ("core.sshCommand", ""),
    ("gpg.format", "openpgp"),
    ("gpg.ssh.program", "ssh-keygen"),
    ("gpg.program", "gpg"),
    ("commit.gpgsign", "false"),
    ("tag.gpgsign", "false"),
];
