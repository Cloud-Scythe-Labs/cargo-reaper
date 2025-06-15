use std::{path, time};

pub(crate) use clap::{CommandFactory, FromArgMatches};
use clap::{Parser, ValueHint};

#[derive(Debug, Parser)]
#[command(
    name = "cargo-reaper",
    version,
    author,
    about = "A Cargo plugin for developing REAPER extension plugins with Rust.",
    long_about = "`cargo-reaper` is a convenience wrapper around Cargo that adds a post-build hook to streamline REAPER extension development. It automatically renames the compiled plugin to include the required `reaper_` prefix and symlinks it to REAPER’s `UserPlugins` directory.

By default, Cargo prefixes dynamic libraries with `lib`, which REAPER does not recognize. Manually renaming the plugin and keeping the `UserPlugins` directory up-to-date can be tedious -- `cargo-reaper` takes care of all that for you, across all supported platforms."
)]
pub struct CargoReaperArgs {
    #[command(subcommand)]
    pub(crate) command: CargoReaperCommand,
}
impl CargoReaperArgs {
    /// Creates the `clap::Command::after_help` message which shows the detected path
    /// to a REAPER binary executable, if any.
    pub(crate) fn reaper_help_heading(reaper_bin_path: Option<&path::Path>) -> String {
        format!(
            "{}\n  {}",
            "\x1b[4mREAPER:\x1b[0m",
            crate::util::ReaperBinaryPath(reaper_bin_path)
        )
    }
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum CargoReaperCommand {
    #[allow(rustdoc::invalid_html_tags)] // rustdoc things <PATH> is an HTML tag...
    /// Create a new REAPER extension plugin from a template at <PATH>.
    New { path: path::PathBuf },

    /// List available extension plugin(s).
    List,

    /// Compile REAPER extension plugin(s).
    Build {
        /// Do not symlink plugin(s) to the `UserPlugins` directory.
        #[arg(long)]
        no_symlink: bool,

        /// Arguments to forward to the `cargo build` invocation.
        #[arg(allow_hyphen_values = true, trailing_var_arg = true, num_args = 0.., value_name = "CARGO_BUILD_ARGS")]
        args: Vec<String>,
    },

    /// Symlink plugin(s) to the `UserPlugins` directory.
    Link {
        /// Create symlink(s) by path.
        #[arg(value_name = "PLUGIN_PATH", value_hint = ValueHint::FilePath, required = true, num_args = 1..)]
        paths: Vec<path::PathBuf>,
    },

    /// Compile and run REAPER extension plugin(s).
    Run {
        /// Override the REAPER executable file path. By default, the REAPER executable found on
        /// `$PATH` will be used. If the REAPER exectuable can't be found in the current working
        /// directory, the default global installation path will be used instead.
        #[arg(
            long = "exec",
            short = 'e',
            value_name = "REAPER",
            value_hint = ValueHint::ExecutablePath
        )]
        reaper: Option<path::PathBuf>,

        /// Open a specific REAPER project file.
        #[arg(
            long = "open-project",
            alias = "open",
            short = 'o',
            value_name = "PROJECT",
            value_hint = ValueHint::FilePath
        )]
        project: Option<path::PathBuf>,

        /// Do not build plugin(s) before running REAPER.
        #[arg(long, conflicts_with = "args")]
        no_build: bool,

        /// Run REAPER in a headless environment.
        #[cfg(target_os = "linux")]
        #[arg(long)]
        headless: bool,

        /// The virtual display that should be used for the headless environment.
        #[cfg(target_os = "linux")]
        #[arg(long, default_value = ":99", requires = "headless")]
        display: String,

        /// Locate a window based on its title and exit with status code 0 if found.
        #[cfg(target_os = "linux")]
        #[arg(
            long = "locate-window",
            short = 'w',
            value_name = "TITLE",
            requires = "headless"
        )]
        window_title: Option<String>,

        /// The amount of time to wait before closing REAPER, in human-readable format (e.g. 10s, 2m, 1h).
        #[arg(
            long,
            short = 't',
            value_name = "DURATION",
            value_parser = humantime::parse_duration
        )]
        timeout: Option<time::Duration>,

        /// Arguments to forward to the `cargo build` invocation.
        #[arg(
            allow_hyphen_values = true,
            trailing_var_arg = true,
            num_args = 0..,
            value_name = "CARGO_BUILD_ARGS",
            conflicts_with = "skip_build_phase"
        )]
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
