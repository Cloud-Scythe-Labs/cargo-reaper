use std::env;

use crate::{
    cli::{CargoReaperArgs, CargoReaperCommand, CommandFactory, FromArgMatches},
    command::{
        build::build, clean::clean, link::link, list::list, new::new, run::run, run::run_headless,
    },
    util::BINARY_NAME,
};

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

    let cmd = CargoReaperArgs::command().after_help(CargoReaperArgs::reaper_help_heading(
        which::which(BINARY_NAME)
            .or_else(|_| util::os::locate_global_default())
            .ok()
            .as_deref(),
    ));

    let args = CargoReaperArgs::from_arg_matches(&cmd.get_matches_from(args)).unwrap();

    match args.command {
        CargoReaperCommand::New { path } => new(path).await,
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
            timeout,
            args,
        } if headless => (!no_build)
            .then(|| build(false, args))
            .transpose()
            .and_then(|_| run_headless(reaper, project, display, window_title, timeout)),
        CargoReaperCommand::Run {
            reaper,
            project,
            no_build,
            timeout,
            args,
            ..
        } => (!no_build)
            .then(|| build(false, args))
            .transpose()
            .and_then(|_| run(reaper, project, timeout)),
        CargoReaperCommand::Clean {
            plugins,
            dry_run,
            remove_artifacts,
        } => clean(&plugins, dry_run, remove_artifacts),
    }
}
