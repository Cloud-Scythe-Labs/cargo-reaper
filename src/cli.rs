use std::path;

pub(crate) use clap::Parser;
use clap::{Subcommand, ValueHint};

#[derive(Debug, Parser)]
#[command(
    name = "cargo-reaper",
    version,
    about = "A Cargo plugin for developing REAPER extension plugins with Rust.",
    long_about = "`cargo-reaper` is a convenience wrapper around Cargo that adds a post-build hook to streamline REAPER extension development. It automatically renames the compiled plugin to include the required `reaper_` prefix and symlinks it to REAPER’s `UserPlugins` directory.

By default, Cargo prefixes dynamic libraries with `lib`, which REAPER does not recognize. Manually renaming the plugin and keeping the `UserPlugins` directory up-to-date can be tedious -- `cargo-reaper` takes care of all that for you, across all supported platforms."
)]
pub struct CargoReaperArgs {
    #[command(subcommand)]
    pub(crate) command: CargoReaperCommand,
}
#[derive(Debug, Clone, Subcommand)]
pub enum CargoReaperCommand {
    /// Create a new REAPER extension plugin from a template at <PATH>.
    New { path: path::PathBuf },

    /// List available extension plugin(s).
    List,

    /// Compile REAPER extension plugin(s).
    Build {
        /// Do not symlink the plugin(s) to the `UserPlugins` directory.
        #[arg(long)]
        no_symlink: bool,

        /// Arguments to forward to the `cargo build` invocation.
        #[arg(allow_hyphen_values = true, trailing_var_arg = true, num_args = 0.., value_name = "CARGO_BUILD_ARGS")]
        args: Vec<String>,
    },

    /// Compile and run REAPER extension plugin(s).
    Run {
        /// Override the REAPER executable file path. By default, the REAPER executable found on
        /// `$PATH` will be used. If the REAPER exectuable can't be found in the current working
        /// directory, the default global installation path will be used instead.
        #[arg(long, short = 'e', value_name = "REAPER", value_hint = ValueHint::ExecutablePath)]
        exec: Option<path::PathBuf>,

        /// Arguments to forward to the `cargo build` invocation.
        #[arg(allow_hyphen_values = true, trailing_var_arg = true, num_args = 0.., value_name = "CARGO_BUILD_ARGS")]
        args: Vec<String>,
    },

    /// Remove plugin(s) from the `UserPlugins` directory that cargo-reaper has generated in the past.
    Clean {
        /// Clean plugin(s) by key.
        #[arg(long = "plugin", short = 'p', value_name = "PLUGIN_KEY")]
        plugins: Vec<String>,

        /// Display what would be deleted without deleting anything.
        #[arg(long, short = 'n')]
        dry_run: bool,

        /// Remove artifacts that cargo-reaper has generated in the past.
        #[arg(long, short = 'a', default_value = "false")]
        remove_artifacts: bool,
    },
}
