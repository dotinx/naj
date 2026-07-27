use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;

// The completion path exits before loading config, so these tests run
// without NAJ_CONFIG_PATH isolation.

#[test]
fn test_zsh_completion_delegates_to_git() {
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .args(["--completion", "zsh"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("_normal")
                .and(predicates::str::contains("naj -l"))
                .and(predicates::str::contains("compdef _naj naj")),
        );
}

#[test]
fn test_bash_completion_delegates_to_git() {
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .args(["--completion", "bash"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("complete -F _naj naj")
                .and(predicates::str::contains("naj -l"))
                .and(predicates::str::contains("_git")),
        );
}

#[test]
fn test_fish_completion_delegates_to_git() {
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .args(["--completion", "fish"])
        .assert()
        .success()
        .stdout(
            // Delegation goes through `complete -C` (fish's `-w git` wrap
            // cannot skip the profile token).
            predicates::str::contains("complete -C")
                .and(predicates::str::contains("naj -l"))
                .and(predicates::str::contains("complete -c naj")),
        );
}

#[test]
fn test_powershell_completion_falls_back_to_static_generation() {
    // Shells without a hand-written script keep clap_complete's output.
    Command::new(env!("CARGO_BIN_EXE_naj"))
        .args(["--completion", "powershell"])
        .assert()
        .success()
        .stdout(predicates::str::contains("naj"));
}
