use std::env;

use crate::{
    cli::{CargoReaperArgs, CargoReaperCommand, CommandFactory, FromArgMatches},
    command::{build::build, clean::clean, link::link, list::list, new::new, run::run},
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

    let cmd = CargoReaperArgs::command().after_help(
        // TODO: Check to see if the default location is available if not on $PATH
        which::which("reaper")
            .map(|reaper| format!("{}\n  {}", "\x1b[4mREAPER:\x1b[0m", reaper.display()))
            .unwrap_or_default(),
    );

    let args = CargoReaperArgs::from_arg_matches(&cmd.get_matches_from(args)).unwrap();

    match args.command {
        CargoReaperCommand::New { path } => new(path).await,
        CargoReaperCommand::List => list(),
        CargoReaperCommand::Build { no_symlink, args } => build(no_symlink, args),
        CargoReaperCommand::Link { paths } => link(paths),
        CargoReaperCommand::Run { exec, args } => build(false, args).and_then(|_| run(exec)),
        CargoReaperCommand::Clean {
            plugins,
            dry_run,
            remove_artifacts,
        } => clean(&plugins, dry_run, remove_artifacts),
    }
}
