pub const BLACKLIST_SECTIONS: &[&str] = &["user", "author", "committer", "gpg"];

pub const BLACKLIST_KEYS: &[&str] = &[
    "core.sshCommand",
    "commit.gpgsign",
    "tag.gpgsign",
    "http.cookieFile",
];

pub fn get_blind_injections() -> Vec<(&'static str, &'static str)> {
    vec![
        ("user.name", ""),
        ("user.email", ""),
        ("user.signingkey", ""),
        ("core.sshCommand", ""),
        ("gpg.format", "openpgp"),
        ("gpg.ssh.program", "ssh-keygen"),
        ("gpg.program", "gpg"),
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
    ]
}
