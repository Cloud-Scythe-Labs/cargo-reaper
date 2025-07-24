use std::{env, io};

use crate::{
    cli::{CargoReaperArgs, CargoReaperCommand, CommandFactory, FromArgMatches, TERM_STYLE},
    command::{build::build, clean::clean, link::link, list::list, new::new, run::run},
    util::BINARY_NAME,
};

#[cfg(target_os = "linux")]
use crate::command::run::run_headless;

pub(crate) mod cli;
pub(crate) mod command;
pub(crate) mod config;
pub(crate) mod error;
pub(crate) mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = env::args().collect::<Vec<_>>();

    // If invoked by Cargo as `cargo reaper`, strip the inserted "reaper" argument
    if args.get(1).map(String::as_str) == Some("reaper") {
        args.remove(1);
    }

    let cmd = CargoReaperArgs::command().styles(TERM_STYLE).after_help(
        CargoReaperArgs::reaper_help_heading(
            which::which(BINARY_NAME)
                .or_else(|_| util::os::locate_global_default())
                .ok()
                .as_deref(),
        ),
    );

    let args = CargoReaperArgs::from_arg_matches(&cmd.clone().get_matches_from(args)).unwrap();

    match args.command {
        CargoReaperCommand::New { template, path } => new(template, path),
        CargoReaperCommand::List => list(),
        CargoReaperCommand::Build { no_symlink, args } => build(no_symlink, args),
        CargoReaperCommand::Link { paths } => link(paths),
        #[cfg(target_os = "linux")]
        CargoReaperCommand::Run {
            reaper,
            project,
            no_build,
            headless,
            display,
            window_title,
            keep_going,
            timeout,
            stdin,
            stdout,
            stderr,
            args,
        } if headless => (!no_build)
            .then(|| build(false, args))
            .transpose()
            .and_then(|_| {
                run_headless(
                    reaper,
                    project,
                    display,
                    window_title,
                    keep_going,
                    timeout,
                    stdin,
                    stdout,
                    stderr,
                )
            }),
        CargoReaperCommand::Run {
            reaper,
            project,
            no_build,
            timeout,
            stdin,
            stdout,
            stderr,
            args,
            ..
        } => (!no_build)
            .then(|| build(false, args))
            .transpose()
            .and_then(|_| run(reaper, project, timeout, stdin, stdout, stderr)),
        CargoReaperCommand::Clean {
            plugins,
            dry_run,
            remove_artifacts,
        } => clean(&plugins, dry_run, remove_artifacts),
        CargoReaperCommand::Completions { shell } => {
            let bin_name = cmd.get_name().to_string();
            let mut cmd = cmd;
            clap_complete::generate(shell, &mut cmd, bin_name, &mut io::stdout());

            Ok(())
        }
    }
}
