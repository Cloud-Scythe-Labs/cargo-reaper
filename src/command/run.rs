use std::{io, path, process};

use crate::util::Colorize;

/// The REAPER executable binary name.
pub(crate) const BINARY_NAME: &str = "reaper";

/// Launch the REAPER binary application. The current working directory takes priority,
/// but if the binary file is not on `$PATH`, the global default location will be used.
///
/// > Note: This function is platform agnostic
///
/// # Usage
///
/// This is run automatically when running the `cargo reaper run` command.
fn _run<G>(
    binary_name: &str,
    override_binary: Option<path::PathBuf>,
    run_global_default: G,
) -> anyhow::Result<()>
where
    G: FnOnce() -> io::Result<process::ExitStatus>,
{
    override_binary
        .inspect(|reaper| {
            println!(
                "{}: overridng REAPER executable path ({})",
                "warning".yellow().bold(),
                reaper.display()
            )
        })
        .or_else(|| which::which(binary_name).ok())
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

#[cfg(target_os = "windows")]
pub(crate) fn run(override_binary: Option<path::PathBuf>) -> anyhow::Result<()> {
    _run(BINARY_NAME, override_binary, || {
        #[cfg(target_arch = "x86_64")]
        const GLOBAL_DEFAULT_PATH: &str = r"C:\Program Files\REAPER (x64)\reaper.exe";

        #[cfg(target_arch = "x86")]
        const GLOBAL_DEFAULT_PATH: &str = r"C:\Program Files (x86)\REAPER\reaper.exe";

        #[cfg(target_arch = "aarch64")]
        const GLOBAL_DEFAULT_PATH: &str = r"C:\Program Files\REAPER (ARM64)\reaper.exe";

        let reaper = path::PathBuf::from(GLOBAL_DEFAULT_PATH);
        if reaper.exists() {
            println!(
                "     {} global default REAPER executable ({})",
                "Running".green().bold(),
                reaper.display(),
            );

            return process::Command::new(reaper)
                .stdin(process::Stdio::inherit())
                .stdout(process::Stdio::inherit())
                .stderr(process::Stdio::inherit())
                .status();
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Unable to locate REAPER executable. Is REAPER installed?\n\nTip: Try overriding the default executable path with `--exec <PATH>`.",
        ))
    })
}

#[cfg(target_os = "linux")]
pub(crate) fn run(override_binary: Option<path::PathBuf>) -> anyhow::Result<()> {
    _run(BINARY_NAME, override_binary, || {
        which::which_global(BINARY_NAME)
            .map_err(|err| io::Error::new(io::ErrorKind::NotFound, err))
            .and_then(|reaper| {
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
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn run(override_binary: Option<path::PathBuf>) -> anyhow::Result<()> {
    _run(BINARY_NAME, override_binary, || {
        const GLOBAL_DEFAULT_ARGS: &[&str; 2] = &["-a", "REAPER"];
        println!(
            "     {} global default REAPER executable (/Applications/REAPER.app)",
            "Running".green().bold(),
        );

        process::Command::new("open")
            .args(GLOBAL_DEFAULT_ARGS)
            .spawn()?
            .wait()
    })
}
