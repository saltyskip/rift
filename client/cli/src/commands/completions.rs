//! `rift completions [shell]` — emit shell completion script, or print
//! install instructions when no shell is given.
//!
//! Bare `rift completions` is the entry-point most users land on — they
//! don't remember the flag, they just type the command. So we detect
//! `$SHELL` and print the exact one-liner to wire it up. Bash, zsh, fish,
//! PowerShell, and elvish are all `clap_complete::Shell` variants;
//! anything we can't recognize falls back to a "pick one of these" hint.

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::error::CliError;

pub fn run(shell: Option<Shell>) -> Result<(), CliError> {
    match shell {
        Some(s) => {
            let mut cmd = crate::Cli::command();
            generate(s, &mut cmd, "rift", &mut std::io::stdout());
            Ok(())
        }
        None => {
            print_install_hint();
            Ok(())
        }
    }
}

fn print_install_hint() {
    match detect_shell() {
        Some(s) => print_shell_instructions(s),
        None => {
            eprintln!(
                "Couldn't detect your shell from $SHELL. Pick one:\n\n  rift completions bash\n  rift completions zsh\n  rift completions fish\n  rift completions powershell\n  rift completions elvish"
            );
        }
    }
}

fn detect_shell() -> Option<Shell> {
    // Match the program name, not a substring — otherwise `/usr/bin/bash`
    // and `/usr/bin/dash` both match `bash` via `.contains`.
    let shell_env = std::env::var("SHELL").ok()?;
    let name = shell_env.rsplit('/').next()?;
    match name {
        "bash" => Some(Shell::Bash),
        "zsh" => Some(Shell::Zsh),
        "fish" => Some(Shell::Fish),
        "elvish" => Some(Shell::Elvish),
        "pwsh" | "powershell" => Some(Shell::PowerShell),
        _ => None,
    }
}

fn print_shell_instructions(shell: Shell) {
    match shell {
        Shell::Zsh => {
            eprintln!(
                "Detected zsh. Install completions with:\n\n  mkdir -p ~/.rift/completions\n  rift completions zsh > ~/.rift/completions/_rift\n\nThen add to ~/.zshrc:\n\n  fpath=(~/.rift/completions $fpath)\n  autoload -U compinit && compinit\n\nRestart your shell (or run `exec zsh`) to pick it up."
            );
        }
        Shell::Bash => {
            eprintln!(
                "Detected bash. Install completions with:\n\n  mkdir -p ~/.rift/completions\n  rift completions bash > ~/.rift/completions/rift.bash\n\nThen add to ~/.bashrc:\n\n  source ~/.rift/completions/rift.bash\n\nRestart your shell (or run `source ~/.bashrc`) to pick it up."
            );
        }
        Shell::Fish => {
            eprintln!(
                "Detected fish. Install completions with:\n\n  mkdir -p ~/.config/fish/completions\n  rift completions fish > ~/.config/fish/completions/rift.fish\n\nFish auto-loads completions from that path — restart your shell to pick it up."
            );
        }
        Shell::PowerShell => {
            eprintln!(
                "Detected PowerShell. Install completions with:\n\n  rift completions powershell | Out-String | Invoke-Expression\n\nAdd that line to your $PROFILE to make it permanent."
            );
        }
        Shell::Elvish => {
            eprintln!(
                "Detected elvish. Install completions with:\n\n  rift completions elvish > ~/.config/elvish/lib/rift.elv\n  echo 'use rift' >> ~/.config/elvish/rc.elv"
            );
        }
        // `Shell` is non-exhaustive; future variants land here without
        // breaking the build, just without bespoke instructions.
        other => {
            eprintln!(
                "Detected {other:?}. Generate completions with:\n\n  rift completions {} > <somewhere on your shell's completion path>",
                shell_name(other)
            );
        }
    }
}

fn shell_name(s: Shell) -> &'static str {
    match s {
        Shell::Bash => "bash",
        Shell::Zsh => "zsh",
        Shell::Fish => "fish",
        Shell::PowerShell => "powershell",
        Shell::Elvish => "elvish",
        _ => "<shell>",
    }
}
