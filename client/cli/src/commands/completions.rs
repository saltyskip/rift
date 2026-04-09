use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::error::CliError;

pub fn run(shell: Shell) -> Result<(), CliError> {
    let mut cmd = crate::Cli::command();
    generate(shell, &mut cmd, "rift", &mut std::io::stdout());
    Ok(())
}
