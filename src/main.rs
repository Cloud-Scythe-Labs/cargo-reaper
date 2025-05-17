use std::env;

use crate::{
    cli::{CargoReaperArgs, CargoReaperCommand, Parser},
    command::{build::build, clean::clean, list::list, new::new, run::run},
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

    let args = CargoReaperArgs::parse_from(args);

    match args.command {
        CargoReaperCommand::New { path } => new(path).await,
        CargoReaperCommand::List => list(),
        CargoReaperCommand::Build { no_symlink, args } => build(no_symlink, args),
        CargoReaperCommand::Run { exec, args } => build(false, args).and_then(|_| run(exec)),
        CargoReaperCommand::Clean {
            plugins,
            dry_run,
            remove_artifacts,
        } => clean(&plugins, dry_run, remove_artifacts),
    }
}
