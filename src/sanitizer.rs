pub const BLACKLIST_SECTIONS: &[&str] = &["user", "author", "committer", "gpg"];

pub const BLACKLIST_KEYS: &[&str] = &[
    "core.sshCommand",
    "commit.gpgsign",
    "tag.gpgsign",
    "http.cookieFile",
];

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
