// Hand-written completion scripts for zsh/bash/fish.
//
// clap_complete's static generation can only complete flags, but naj's
// grammar is `naj <PROFILE_ID> [GIT_ARGS]...` — the interesting completions
// are dynamic. These scripts complete profile IDs (from `naj -l`) at the
// first position and then delegate to git's native completion so subcommands,
// refs, and paths are completed exactly as they are for `git` itself.
//
// Shells without a custom script (powershell, elvish) fall back to
// clap_complete's static generation.

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

const ZSH: &str = r#"#compdef naj
# naj(1) completion for zsh.
# Position 1 completes profile IDs (dynamic) and flags; later positions
# delegate to git's completion by rewriting the words to `git <args...>`.
#
# Usage: source <(naj --completion zsh)

_naj() {
    if (( CURRENT == 2 )); then
        local -a profiles
        profiles=(${(f)"$(command naj -l 2>/dev/null)"})
        _arguments -s \
            '(-c --create)'{-c,--create}'[Create a new profile: NAME EMAIL ID]' \
            '(-l --list)'{-l,--list}'[List all available profiles]' \
            '(-r --remove)'{-r,--remove}'[Remove a profile by ID]:profile id:($profiles)' \
            '(-f --force)'{-f,--force}'[Force switch strategy (hard clean)]' \
            '--completion[Generate shell completion script]:shell:(bash elvish fish powershell zsh)' \
            '(-h --help)'{-h,--help}'[Print help]' \
            '(-V --version)'{-V,--version}'[Print version]' \
            "1:profile id:($profiles)"
        return
    fi

    # Delegate: "naj <profile> <git args...>" -> "git <git args...>".
    # The current word (possibly empty) is preserved by the 3..-1 slice.
    # _normal re-dispatches on words[1], i.e. to _git with the correct
    # service context (the same trick _sudo uses).
    words=( git "${(@)words[3,-1]}" )
    (( CURRENT -= 1 ))
    _normal
}

compdef _naj naj
"#;

const BASH: &str = r#"# naj(1) completion for bash.
# First argument completes profile IDs (dynamic) and flags; subsequent
# arguments delegate to git's bash completion (_git) by rewriting
# COMP_WORDS to `git <args...>`.
#
# Requires git's bash completion to be loaded (the default on most
# distributions); without it, positions after the profile silently
# fall back to default completion.
#
# Usage: source <(naj --completion bash)

_naj() {
    local cur
    cur="${COMP_WORDS[COMP_CWORD]}"

    if (( COMP_CWORD == 1 )); then
        COMPREPLY=( $(compgen -W "\
-c --create -l --list -r --remove -f --force \
--completion -h --help -V --version \
$(naj -l 2>/dev/null)" -- "$cur") )
        return
    fi

    # Delegate: "naj <profile> <git args...>" -> "git <git args...>".
    # Modern git bash-completion exposes __git_func_wrap/__git_main;
    # older versions (and plain git-completion.bash) define _git.
    if declare -F __git_func_wrap >/dev/null 2>&1 && declare -F __git_main >/dev/null 2>&1; then
        COMP_WORDS=( git "${COMP_WORDS[@]:2}" )
        COMP_CWORD=$(( COMP_CWORD - 1 ))
        __git_func_wrap __git_main
    elif declare -F _git >/dev/null 2>&1; then
        COMP_WORDS=( git "${COMP_WORDS[@]:2}" )
        COMP_CWORD=$(( COMP_CWORD - 1 ))
        _git
    fi
}

complete -F _naj naj
"#;

const FISH: &str = r#"# naj(1) completion for fish.
# The first argument completes profile IDs (dynamic) and flags; once a
# valid profile is present, further arguments are delegated to git's own
# completions by rewriting the command line as `git <args...>` and asking
# `complete -C` for candidates. (fish's `-w git` wrap cannot skip the
# profile token, so delegation has to go through `complete -C`.)
#
# Usage: naj --completion fish | source

# Drop any previously registered definitions so re-sourcing upgrades cleanly.
complete -c naj -e 2>/dev/null

function __naj_profiles
    naj -l 2>/dev/null
end

function __naj_has_profile
    # True once the second token is an existing profile ID. Checking
    # against the actual profile list (instead of token count) keeps
    # partial profile names (`naj wor<TAB>`) on profile completion.
    set -l tokens (commandline -opc)
    test (count $tokens) -ge 2; and contains -- $tokens[2] (__naj_profiles)
end

function __naj_git_complete
    # Delegate: "naj <profile> <rest...>" -> "git <rest...>".
    set -l line (commandline -cp)
    set -l gitline (string replace -r '^\s*\S+\s+\S+' git -- "$line")
    complete -C "$gitline"
end

complete -c naj -n 'not __naj_has_profile' -f -a '(__naj_profiles)' -d 'Profile ID'
complete -c naj -n 'not __naj_has_profile' -s c -l create -d 'Create a new profile: NAME EMAIL ID'
complete -c naj -n 'not __naj_has_profile' -s l -l list -d 'List all available profiles'
complete -c naj -n 'not __naj_has_profile' -s r -l remove -x -a '(__naj_profiles)' -d 'Remove a profile by ID'
complete -c naj -n 'not __naj_has_profile' -s f -l force -d 'Force switch strategy (hard clean)'
complete -c naj -n 'not __naj_has_profile' -l completion -x -a 'bash elvish fish powershell zsh' -d 'Generate shell completion script'
complete -c naj -n '__naj_has_profile' -fa '(__naj_git_complete)'
"#;

pub fn print_completion(shell: Shell) -> Result<()> {
    match shell {
        Shell::Zsh => print!("{}", ZSH),
        Shell::Bash => print!("{}", BASH),
        Shell::Fish => print!("{}", FISH),
        other => {
            let mut cmd = crate::Cli::command();
            let name = cmd.get_name().to_string();
            generate(other, &mut cmd, name, &mut io::stdout());
        }
    }
    Ok(())
}
