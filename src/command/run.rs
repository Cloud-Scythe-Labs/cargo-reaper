use std::{io, path, process, thread, time};

use crate::{
    cli,
    util::{self, BINARY_NAME, Colorize},
};

/// Launch the REAPER binary application. The current working directory takes priority,
/// but if the binary file is not on `$PATH`, the global default location will be used.
///
/// > Note: This function is platform agnostic
///
/// # Usage
///
/// This is run automatically when running the `cargo reaper run` command.
pub(crate) fn run(
    override_binary: Option<path::PathBuf>,
    project: Option<path::PathBuf>,
    timeout: Option<time::Duration>,
    stdin: cli::Stdio,
    stdout: cli::Stdio,
    stderr: cli::Stdio,
) -> anyhow::Result<()> {
    override_binary
        .inspect(|reaper| {
            println!(
                "{}: overridng REAPER executable path ({})",
                "warning".yellow().bold(),
                reaper.display()
            )
        })
        .or_else(|| which::which(BINARY_NAME).ok())
        .map_or_else(
            || run_global_default(project.as_ref(), timeout.as_ref(), stdin, stdout, stderr),
            |reaper| {
                println!(
                    "     {} REAPER executable ({})",
                    "Running".green().bold(),
                    reaper.display(),
                );

                timeout.map_or_else(
                    || {
                        run_reaper(
                            &reaper,
                            project.as_ref(),
                            stdin.into(),
                            stdout.into(),
                            stderr.into(),
                        )?
                        .wait()
                    },
                    |timeout| {
                        let (mut reaper, start) = run_reaper(
                            &reaper,
                            project.as_ref(),
                            stdin.into(),
                            stdout.into(),
                            stderr.into(),
                        )
                        .map(|reaper| (reaper, time::Instant::now()))?;

                        loop {
                            match reaper.try_wait()? {
                                Some(status) => break Ok(status),
                                None if start.elapsed() >= timeout => {
                                    reaper.kill()?;
                                    break reaper.wait();
                                }
                                None => thread::sleep(time::Duration::from_secs(1)),
                            }
                        }
                    },
                )
            },
        )
        .map_err(|err| anyhow::anyhow!("While attempting to run REAPER executable: {err:?}"))?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[cfg(target_os = "linux")]
pub(crate) fn run_headless(
    override_binary: Option<path::PathBuf>,
    project: Option<path::PathBuf>,
    display: String,
    window_title: Option<String>,
    keep_going: bool,
    timeout: Option<time::Duration>,
    stdin: cli::Stdio,
    stdout: cli::Stdio,
    stderr: cli::Stdio,
) -> anyhow::Result<()> {
    override_binary
        .inspect(|reaper| {
            println!(
                "{}: overridng REAPER executable path ({})",
                "warning".yellow().bold(),
                reaper.display()
            )
        })
        .or_else(|| which::which(BINARY_NAME).ok())
        .map_or_else(
            || {
                run_global_default_headless(
                    project.as_ref(),
                    &display,
                    window_title.as_deref(),
                    keep_going,
                    timeout.as_ref(),
                    stdin,
                    stdout,
                    stderr,
                )
            },
            |reaper| {
                println!(
                    "     {} REAPER executable ({})",
                    "Running".green().bold(),
                    reaper.display(),
                );

                timeout.map_or_else(
                    || {
                        run_reaper_headless(
                            &reaper,
                            project.as_ref(),
                            &display,
                            stdin,
                            stdout,
                            stderr,
                        )
                        .and_then(|(mut xvfb, mut reaper)| reaper.wait().and_then(|_| xvfb.wait()))
                    },
                    |timeout| {
                        let ((mut xvfb, mut reaper), start) = run_reaper_headless(
                            &reaper,
                            project.as_ref(),
                            &display,
                            stdin,
                            stdout,
                            stderr,
                        )
                        .map(|(xvfb, reaper)| ((xvfb, reaper), time::Instant::now()))?;

                        let mut exit_code: i32 = keep_going.then_some(1).unwrap_or_default();

                        loop {
                            if let Some(window_title) = &window_title {
                                const XDOTOOL: &str = "xdotool";
                                const XDOTOOL_ARGS: &[&str; 2] = &["search", "--name"];
                                if exit_code != 0
                                    && process::Command::new(XDOTOOL)
                                        .args(XDOTOOL_ARGS)
                                        .arg(window_title)
                                        .env("DISPLAY", &display)
                                        .output()
                                        .map(|output| output.status.success())
                                        .unwrap_or(false)
                                {
                                    if keep_going {
                                        exit_code = 0;
                                    } else {
                                        kill_and_exit(&mut reaper, &mut xvfb, 0)?;
                                    }
                                }
                            }
                            match reaper.try_wait()? {
                                Some(_) if window_title.is_some() => {
                                    kill_and_exit(&mut reaper, &mut xvfb, exit_code)?;
                                }
                                Some(status) => break Ok(status),
                                None if start.elapsed() >= timeout => {
                                    kill_and_exit(&mut reaper, &mut xvfb, exit_code)?;
                                }
                                None => thread::sleep(time::Duration::from_secs(1)),
                            }
                        }
                    },
                )
            },
        )
        .map_err(|err| anyhow::anyhow!("While attempting to run REAPER executable: {err:?}"))?;

    Ok(())
}

/// Run the global default REAPER binary executable.
fn run_global_default(
    project: Option<&path::PathBuf>,
    timeout: Option<&time::Duration>,
    stdin: cli::Stdio,
    stdout: cli::Stdio,
    stderr: cli::Stdio,
) -> io::Result<process::ExitStatus> {
    util::os::locate_global_default().and_then(|reaper| {
        println!(
            "     {} global default REAPER executable ({})",
            "Running".green().bold(),
            reaper.display(),
        );

        timeout.map_or_else(
            || run_reaper(&reaper, project, stdin.into(), stdout.into(), stderr.into())?.wait(),
            |timeout| {
                let (mut reaper, start) =
                    run_reaper(&reaper, project, stdin.into(), stdout.into(), stderr.into())
                        .map(|reaper| (reaper, time::Instant::now()))?;

                loop {
                    match reaper.try_wait()? {
                        Some(status) => break Ok(status),
                        None if start.elapsed() >= *timeout => {
                            reaper.kill()?;
                            break reaper.wait();
                        }
                        None => thread::sleep(time::Duration::from_secs(1)),
                    }
                }
            },
        )
    })
}

#[allow(clippy::too_many_arguments)]
#[cfg(target_os = "linux")]
fn run_global_default_headless(
    project: Option<&path::PathBuf>,
    display: &str,
    window_title: Option<&str>,
    keep_going: bool,
    timeout: Option<&time::Duration>,
    stdin: cli::Stdio,
    stdout: cli::Stdio,
    stderr: cli::Stdio,
) -> io::Result<process::ExitStatus> {
    util::os::locate_global_default().and_then(|reaper| {
        println!(
            "     {} global default REAPER executable ({})",
            "Running".green().bold(),
            reaper.display(),
        );

        timeout.map_or_else(
            || {
                run_reaper_headless(&reaper, project, display, stdin, stdout, stderr)
                    .and_then(|(mut xvfb, mut reaper)| reaper.wait().and_then(|_| xvfb.wait()))
            },
            |timeout| {
                let ((mut xvfb, mut reaper), start) =
                    run_reaper_headless(&reaper, project, display, stdin, stdout, stderr)
                        .map(|(xvfb, reaper)| ((xvfb, reaper), time::Instant::now()))?;

                let mut exit_code: i32 = keep_going.then_some(1).unwrap_or_default();

                loop {
                    if let Some(window_title) = &window_title {
                        const XDOTOOL: &str = "xdotool";
                        const XDOTOOL_ARGS: &[&str; 2] = &["search", "--name"];
                        if exit_code != 0
                            && process::Command::new(XDOTOOL)
                                .args(XDOTOOL_ARGS)
                                .arg(window_title)
                                .env("DISPLAY", display)
                                .output()
                                .map(|output| output.status.success())
                                .unwrap_or(false)
                        {
                            if keep_going {
                                exit_code = 0;
                            } else {
                                kill_and_exit(&mut reaper, &mut xvfb, 0)?;
                            }
                        }
                    }
                    match reaper.try_wait()? {
                        Some(_) if window_title.is_some() => {
                            kill_and_exit(&mut reaper, &mut xvfb, exit_code)?;
                        }
                        Some(status) => break Ok(status),
                        None if start.elapsed() >= *timeout => {
                            kill_and_exit(&mut reaper, &mut xvfb, exit_code)?;
                        }
                        None => thread::sleep(time::Duration::from_secs(1)),
                    }
                }
            },
        )
    })
}

fn run_reaper(
    reaper: &path::PathBuf,
    project: Option<&path::PathBuf>,
    stdin: process::Stdio,
    stdout: process::Stdio,
    stderr: process::Stdio,
) -> io::Result<process::Child> {
    process::Command::new(reaper)
        .args(project.iter())
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("Command `{}` failed: {}", reaper.display(), err),
            )
        })
}

#[cfg(target_os = "linux")]
fn run_reaper_headless(
    reaper: &path::PathBuf,
    project: Option<&path::PathBuf>,
    display: &str,
    stdin: cli::Stdio,
    stdout: cli::Stdio,
    stderr: cli::Stdio,
) -> io::Result<(process::Child, process::Child)> {
    const XVFB: &str = "Xvfb";
    const XVFB_ARGS: &[&str; 5] = &["-screen", "0", "1024x768x24", "-nolisten", "tcp"];

    process::Command::new(XVFB)
        .arg(display)
        .args(XVFB_ARGS)
        .env("DISPLAY", display)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map_err(|err| io::Error::new(err.kind(), format!("Command `{}` failed: {}", XVFB, err)))
        .and_then(|xvfb| {
            Ok((
                xvfb,
                process::Command::new(reaper)
                    .args(project.iter())
                    .env("DISPLAY", display)
                    .stdin(stdin)
                    .stdout(stdout)
                    .stderr(stderr)
                    .spawn()
                    .map_err(|err| {
                        io::Error::new(
                            err.kind(),
                            format!("Command `{}` failed: {}", reaper.display(), err),
                        )
                    })?,
            ))
        })
}

fn kill_and_exit(
    reaper: &mut process::Child,
    xvfb: &mut process::Child,
    exit_code: i32,
) -> io::Result<()> {
    reaper
        .kill()
        .and_then(|_| reaper.wait())
        .and_then(|_| xvfb.kill())
        .and_then(|_| xvfb.wait())?;
    process::exit(exit_code);
}
