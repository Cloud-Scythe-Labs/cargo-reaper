use std::{io, path, process};

use crate::util::{self, BINARY_NAME, Colorize};

/// Launch the REAPER binary application. The current working directory takes priority,
/// but if the binary file is not on `$PATH`, the global default location will be used.
///
/// > Note: This function is platform agnostic
///
/// # Usage
///
/// This is run automatically when running the `cargo reaper run` command.
pub(crate) fn run(override_binary: Option<path::PathBuf>) -> anyhow::Result<()> {
    override_binary
        .inspect(|reaper| {
            println!(
                "{}: overridng REAPER executable path ({})",
                "warning".yellow().bold(),
                reaper.display()
            )
        })
        .or_else(|| which::which(BINARY_NAME).ok())
        .map_or_else(run_global_default, |reaper| {
            println!(
                "     {} REAPER executable ({})",
                "Running".green().bold(),
                reaper.display(),
            );

            process::Command::new(reaper)
                .stdin(process::Stdio::inherit())
                .stdout(process::Stdio::inherit())
                .stderr(process::Stdio::inherit())
                .status()
        })
        .map_err(|err| anyhow::anyhow!("While attempting to run REAPER executable: {err:?}"))?;

    Ok(())
}

/// Run the global default REAPER binary executable.
fn run_global_default() -> io::Result<process::ExitStatus> {
    util::os::locate_global_default().and_then(|reaper| {
        println!(
            "     {} global default REAPER executable ({})",
            "Running".green().bold(),
            reaper.display(),
        );

        process::Command::new(reaper)
            .stdin(process::Stdio::inherit())
            .stdout(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
            .status()
    })
}
