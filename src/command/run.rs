use std::{io, path, process, thread, time};

use nix::{
    sys::signal::{Signal, killpg},
    unistd::{Pid, setpgid},
};

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

use crate::util::{self, BINARY_NAME, Colorize};

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
            || run_global_default(project.as_ref(), timeout.as_ref()),
            |reaper| {
                println!(
                    "     {} REAPER executable ({})",
                    "Running".green().bold(),
                    reaper.display(),
                );

                timeout.map_or_else(
                    || run_reaper(&reaper, project.as_ref())?.wait(),
                    |timeout| {
                        let (mut reaper, start) = run_reaper(&reaper, project.as_ref())
                            .map(|reaper| (reaper, time::Instant::now()))?;

                        loop {
                            match reaper.try_wait()? {
                                Some(status) => break Ok(status),
                                None if start.elapsed() >= timeout => {
                                    kill_process_group(&reaper)?;
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

#[cfg(target_os = "linux")]
pub(crate) fn run_headless(
    override_binary: Option<path::PathBuf>,
    project: Option<path::PathBuf>,
    display: String,
    window_title: Option<String>,
    timeout: Option<time::Duration>,
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
                    timeout.as_ref(),
                )
            },
            |reaper| {
                println!(
                    "     {} REAPER executable ({})",
                    "Running".green().bold(),
                    reaper.display(),
                );

                timeout.map_or_else(
                    || run_reaper_headless(&reaper, project.as_ref(), &display)?.wait(),
                    |timeout| {
                        let (mut reaper, start) =
                            run_reaper_headless(&reaper, project.as_ref(), &display)
                                .map(|reaper| (reaper, time::Instant::now()))?;

                        loop {
                            if let Some(window_title) = &window_title {
                                const XDOTOOL: &str = "xdotool";
                                const XDOTOOL_ARGS: &[&str; 2] = &["search", "--name"];
                                if dbg!(
                                    process::Command::new(XDOTOOL)
                                        .args(XDOTOOL_ARGS)
                                        .arg(window_title)
                                        .env("DISPLAY", &display)
                                )
                                .output()
                                .map(|output| output.status.success())
                                .unwrap_or(false)
                                {
                                    kill_process_group(&reaper)?;
                                    reaper.wait()?;
                                    process::exit(0);
                                }
                            }
                            match reaper.try_wait()? {
                                Some(_) if window_title.is_some() => process::exit(1),
                                Some(status) => break Ok(status),
                                None if start.elapsed() >= timeout => {
                                    kill_process_group(&reaper)?;
                                    let status = reaper.wait();
                                    if window_title.is_some() {
                                        process::exit(1);
                                    }
                                    break status;
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
) -> io::Result<process::ExitStatus> {
    util::os::locate_global_default().and_then(|reaper| {
        println!(
            "     {} global default REAPER executable ({})",
            "Running".green().bold(),
            reaper.display(),
        );

        timeout.map_or_else(
            || run_reaper(&reaper, project)?.wait(),
            |timeout| {
                let (mut reaper, start) =
                    run_reaper(&reaper, project).map(|reaper| (reaper, time::Instant::now()))?;

                loop {
                    match reaper.try_wait()? {
                        Some(status) => break Ok(status),
                        None if start.elapsed() >= *timeout => {
                            kill_process_group(&reaper)?;
                            break reaper.wait();
                        }
                        None => thread::sleep(time::Duration::from_secs(1)),
                    }
                }
            },
        )
    })
}

#[cfg(target_os = "linux")]
fn run_global_default_headless(
    project: Option<&path::PathBuf>,
    display: &str,
    window_title: Option<&str>,
    timeout: Option<&time::Duration>,
) -> io::Result<process::ExitStatus> {
    util::os::locate_global_default().and_then(|reaper| {
        println!(
            "     {} global default REAPER executable ({})",
            "Running".green().bold(),
            reaper.display(),
        );

        timeout.map_or_else(
            || run_reaper_headless(&reaper, project, display)?.wait(),
            |timeout| {
                let (mut reaper, start) = run_reaper_headless(&reaper, project, display)
                    .map(|reaper| (reaper, time::Instant::now()))?;

                loop {
                    if let Some(window_title) = &window_title {
                        const XDOTOOL: &str = "xdotool";
                        const XDOTOOL_ARGS: &[&str; 2] = &["search", "--name"];
                        if process::Command::new(XDOTOOL)
                            .args(XDOTOOL_ARGS)
                            .arg(window_title)
                            .env("DISPLAY", display)
                            .output()
                            .map(|output| output.status.success())
                            .unwrap_or(false)
                        {
                            kill_process_group(&reaper)?;
                            reaper.wait()?;
                            process::exit(0);
                        }
                    }
                    match reaper.try_wait()? {
                        Some(_) if window_title.is_some() => process::exit(1),
                        Some(status) => break Ok(status),
                        None if start.elapsed() >= *timeout => {
                            kill_process_group(&reaper)?;
                            let status = reaper.wait();
                            if window_title.is_some() {
                                process::exit(1);
                            }
                            break status;
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
) -> io::Result<process::Child> {
    process::Command::new(reaper)
        .args(project.iter())
        .stdin(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .spawn()
}

#[cfg(target_os = "linux")]
fn run_reaper_headless(
    reaper: &path::PathBuf,
    project: Option<&path::PathBuf>,
    display: &str,
) -> io::Result<process::Child> {
    const XVFB_RUN: &str = "xvfb-run";
    const XVFB_RUN_ARGS: &[&str; 1] = &["-a"];

    unsafe {
        process::Command::new(XVFB_RUN)
            .args(XVFB_RUN_ARGS)
            .arg(reaper)
            .args(project.iter())
            .env("DISPLAY", display)
            .pre_exec(|| {
                setpgid(Pid::from_raw(0), Pid::from_raw(0))
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
            })
            .stdin(process::Stdio::inherit())
            .stdout(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
            .spawn()
    }
}

#[cfg(target_os = "linux")]
fn kill_process_group(child: &process::Child) -> std::io::Result<()> {
    let pid = Pid::from_raw(child.id() as i32);
    killpg(pid, Signal::SIGKILL).map_err(|err| io::Error::new(io::ErrorKind::Other, err))
}
