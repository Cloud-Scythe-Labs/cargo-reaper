#![allow(dead_code)]

use std::{collections, env, fmt, fs, io, ops, os, path, process};

use clap::{Parser, Subcommand, ValueHint};
use codespan_reporting::{
    diagnostic, files,
    term::{self, termcolor},
};
use colored::Colorize;

/// Acceptable plugin config toml names for renaming and symlinking REAPER extenion plugins built with Rust.
pub const CONFIG_FILE_NAMES: &[&str; 2] = &[".reaper.toml", "reaper.toml"];

/// The dynamically linked C library Windows extension
pub const WINDOWS_PLUGIN_EXT: &str = ".dll";
/// The dynamically linked C library Linux extension
pub const LINUX_PLUGIN_EXT: &str = ".so";
/// The dynamically linked C library MacOS (Darwin) extension
pub const DARWIN_PLUGIN_EXT: &str = ".dylib";

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

/// An identifier that corresponds to some [`codespan_reporting::file::SimpleFile`].
pub(crate) type FileId = usize;

/// The message to display for some diagnostic error.
pub(crate) type Message = String;

/// Collection of diagnostics for toml files that is context aware.
#[derive(Default)]
pub(crate) struct TomlErrorEmitter<FilePath, FileContents>
where
    FilePath: fmt::Display + Clone + Default + Sized,
    FileContents: AsRef<str> + Clone + Default,
{
    /// A collection of file paths and their contents.
    db: files::SimpleFiles<FilePath, FileContents>,

    /// A collection of diagnostic data containing identifiers corresponding to the db.
    errors: Vec<diagnostic::Diagnostic<FileId>>,
}
impl<FilePath, FileContents> TomlErrorEmitter<FilePath, FileContents>
where
    FilePath: fmt::Display + Clone + Default + Sized,
    FileContents: AsRef<str> + Clone + Default,
{
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Given some diagnostic error info, insert the file and string contents into the db, and
    /// add an error built from the messages and spans to the list of errors. The file id is
    /// automatically handled between the diagnostic error and the db.
    #[allow(clippy::too_many_arguments)]
    fn insert_err(
        &mut self,
        path: FilePath,
        contents: FileContents,
        message: impl Into<Message>,
        primary_span: ops::Range<usize>,
        primary_msg: Option<impl Into<Message>>,
        secondary_span: Option<ops::Range<usize>>,
        secondary_msg: Option<impl Into<Message>>,
    ) {
        let error = diagnostic::Diagnostic::error().with_message(message.into());
        let mut labels: Vec<diagnostic::Label<usize>> = Vec::with_capacity(2);
        let mut primary_label: diagnostic::Label<usize> = diagnostic::Label::primary(
            self.db.add(path.clone(), contents.clone()),
            primary_span.clone(),
        );
        if let Some(primary_msg) = primary_msg {
            primary_label = primary_label.with_message(primary_msg.into());
        }
        labels.push(primary_label);
        if let Some(secondary_msg) = secondary_msg {
            labels.push(
                diagnostic::Label::secondary(
                    self.db.add(path, contents),
                    secondary_span.unwrap_or(primary_span),
                )
                .with_message(secondary_msg.into()),
            );
        }
        self.errors.push(error.with_labels(labels))
    }

    /// Exit with errors, if any.
    fn emit(self) -> anyhow::Result<()> {
        if !self.errors.is_empty() {
            for error in self.errors.iter().rev() {
                term::emit(
                    &mut termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto),
                    &Default::default(),
                    &self.db,
                    error,
                )?;
            }
            process::exit(1);
        }
        Ok(())
    }
}

/// The parsed contents of a `reaper.toml` config file.
#[derive(Debug, serde::Deserialize)]
pub struct ReaperPluginConfig {
    /// The path to the `reaper.toml` config file.
    #[serde(skip)]
    file: path::PathBuf,

    /// The contents of a `reaper.toml` config file as a [`std::str::String`].
    #[serde(skip)]
    contents: String,

    /// The contents of a deserialized `reaper.toml` config file.
    extension_plugins: collections::HashMap<toml::Spanned<String>, toml::Spanned<path::PathBuf>>,
}
impl ReaperPluginConfig {
    /// The path to the `reaper.toml` config file.
    pub fn file(&self) -> &path::PathBuf {
        &self.file
    }

    /// The path to the `reaper.toml` config file.
    pub fn contents(&self) -> &str {
        &self.contents
    }

    /// Locate and deserialize a `reaper.toml` config file.
    pub fn load(project_root: &path::Path) -> anyhow::Result<Self> {
        let config_file = CONFIG_FILE_NAMES
            .iter()
            .map(|config_file_name| project_root.join(config_file_name))
            .find(|config_path| config_path.exists())
            .unwrap(); // We already ensured this path exists
        let config_contents = fs::read_to_string(&config_file)
            .map_err(|err| anyhow::anyhow!("failed to read reaper toml file:\n{err:#?}"))?;

        let mut config: Self = toml::from_str(&config_contents).map_err(|err| {
            anyhow::anyhow!("failed to load plugin config from reaper toml:\n{err:#?}")
        })?;
        config.file = config_file;
        config.contents = config_contents;

        Ok(config)
    }
}

/// Represents a REAPER plugin's manifest information.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PluginManifest {
    name: String,
    version: String,
    authors: Vec<String>,
    description: Option<String>,
}
impl PluginManifest {
    fn new(
        name: String,
        version: String,
        authors: Vec<String>,
        description: Option<String>,
    ) -> Self {
        Self {
            name,
            version,
            authors,
            description,
        }
    }
}
impl fmt::Display for PluginManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} v{}", self.name.blue(), self.version)?;
        if self.description.is_some() {
            write!(f, " -- {}", self.description.as_ref().unwrap())?;
        }
        if !self.authors.is_empty() {
            write!(f, "\n\nAuthored by: {}", self.authors.join(", "))?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = env::args().collect::<Vec<_>>();

    // If invoked by Cargo as `cargo reaper`, strip the inserted "reaper" argument
    if args.get(1).map(String::as_str) == Some("reaper") {
        args.remove(1);
    }

    let args = CargoReaperArgs::parse_from(args);

    match args.command {
        CargoReaperCommand::New { path } => {
            if path.exists() {
                anyhow::bail!("project path already exists");
            }

            let package_name = path
                .components()
                .next_back()
                .ok_or_else(|| anyhow::anyhow!("failed to produce package name from directory"))?
                .as_os_str()
                .to_string_lossy();
            println!(
                "    {} dynamically linked library (cdylib) `{}` REAPER extension plugin package",
                "Creating".green().bold(),
                package_name
            );
            new_from_template(&path, &package_name)
                .await
                .map_err(|err| {
                    anyhow::anyhow!("failed to create new REAPER extension plugin project: {err:?}")
                })
        }
        CargoReaperCommand::List => {
            let mut emitter = TomlErrorEmitter::<String, String>::new();
            let config = ReaperPluginConfig::load(&find_project_root()?)?;
            let mut plugins: Vec<String> = Vec::new();
            for (plugin_name, manifest_dir) in config.extension_plugins.iter() {
                let manifest_file = manifest_dir.get_ref().join("Cargo.toml");
                let manifest_file_content = fs::read_to_string(&manifest_file).map_err(|err| {
                    anyhow::anyhow!(
                        "Failed to read manifest '{}' for plugin '{}':\n{err:#?}",
                        manifest_file.display(),
                        plugin_name.as_ref()
                    )
                })?;
                let mut manifest = validate_plugin(
                    &mut emitter,
                    config.file(),
                    config.contents(),
                    plugin_name,
                    &manifest_file,
                    &manifest_file_content,
                )?;
                let _ = manifest
                    .as_mut()
                    .complete_from_path_and_workspace::<cargo_toml::Value>(&manifest_file, None);
                if let Some(package) = manifest.as_ref().package.as_ref() {
                    plugins.push(
                        PluginManifest::new(
                            plugin_name.as_ref().to_string(),
                            package.version().to_string(),
                            package.authors().to_owned(),
                            package.description().map(|desc| desc.to_string()),
                        )
                        .to_string(),
                    );
                } else {
                    emitter.insert_err(
                        manifest_file.to_string_lossy().to_string(),
                        manifest_file_content,
                        format!("`{}` is not a package", plugin_name.as_ref()),
                        manifest.span(),
                        Some("expected manifest path to a package containing a dynamic library target"),
                        None,
                        Some("help: is this a workspace? try adding the `[workspace.package]` attribute"),
                    );
                }
            }

            emitter.emit()?;
            plugins.sort();

            println!(
                "\n{}:\n\n{}",
                "Available Plugins".green().bold(),
                plugins.join("\n\n--\n\n")
            );

            Ok(())
        }
        CargoReaperCommand::Build { no_symlink, args } => build(no_symlink, args),
        CargoReaperCommand::Run { exec, args } => build(false, args).and_then(|_| run(exec)),
        CargoReaperCommand::Clean {
            plugins,
            dry_run,
            remove_artifacts,
        } => clean(&plugins, dry_run, remove_artifacts),
    }
}

/// Processes the reaper config toml and the plugin `Cargo.toml` files, collecting diagnostic errors and returning the plugin's manifest.
pub(crate) fn validate_plugin(
    emitter: &mut TomlErrorEmitter<String, String>,
    config_file: &path::Path,
    config_contents: &str,
    plugin_name: &toml::Spanned<String>,
    manifest_file: &path::Path,
    manifest_file_content: &str,
) -> anyhow::Result<toml::Spanned<cargo_toml::Manifest>> {
    let config_file = config_file.to_string_lossy();
    if !plugin_name.as_ref().starts_with("reaper_") {
        emitter.insert_err(
            config_file.to_string(),
            config_contents.to_string(),
            "Invalid extension plugin name",
            plugin_name.span(),
            Some("extension plugins must be prefixed by `reaper_` to be recognized"),
            None,
            Some(format!(
                "help: consider changing this to `reaper_{}`",
                plugin_name.as_ref()
            )),
        );
    }

    let manifest = toml::Spanned::new(
        0..manifest_file_content.len(),
        cargo_toml::Manifest::from_str(manifest_file_content).map_err(|err| {
            anyhow::anyhow!(
                "Failed to parse plugin manifest '{}':\n{err:#?}",
                manifest_file.display()
            )
        })?,
    );

    let lib = manifest.as_ref().lib.as_ref();

    if lib.is_none() {
        emitter.insert_err(
            manifest_file.to_string_lossy().to_string(),
            manifest_file_content.to_string(),
            format!(
                "`{}` does not contain a library target",
                plugin_name.as_ref()
            ),
            manifest.span(),
            None::<Message>,
            None,
            Some("help: add the `[lib]` target attribute"),
        );
    }
    if lib.is_some_and(|lib| lib.name.is_none()) {
        let lib_index = manifest_file_content.find("[lib]").unwrap();
        let lib = toml::Spanned::new(lib_index..lib_index + 5, manifest.as_ref().lib.as_ref());
        emitter.insert_err(
            manifest_file.to_string_lossy().to_string(),
            manifest_file_content.to_string(),
            format!("`{}` library is unnamed", plugin_name.as_ref()),
            lib.span(),
            Some("a name is required in order for plugin path resolution during renaming"),
            None,
            Some("help: add `name = \"<...>\"`"),
        );
    }
    if lib.is_some_and(|lib| {
        !lib.crate_type
            .iter()
            .any(|crate_type| crate_type == "cdylib")
    }) {
        let lib_index = manifest_file_content.find("[lib]").unwrap();
        let lib = toml::Spanned::new(lib_index..lib_index + 5, manifest.as_ref().lib.as_ref());
        emitter.insert_err(
            manifest_file.to_string_lossy().to_string(),
            manifest_file_content.to_string(),
            format!("`{}` is not a dynamic library", plugin_name.as_ref()),
            lib.span(),
            Some("extension plugins must be dynamic libraries to be recognized"),
            None,
            Some("help: add `crate-type = [\"cdylib\"]`"),
        );
    }
    Ok(manifest)
}

/// Build a REAPER extension plugin.
pub(crate) fn build(no_symlink: bool, args: Vec<String>) -> anyhow::Result<()> {
    let project_root = find_project_root()?;
    let mut emitter = TomlErrorEmitter::<String, String>::new();
    let config = ReaperPluginConfig::load(&project_root)?;

    match process::Command::new("cargo")
        .arg("build")
        .args(&args)
        .stdin(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .status()
        .map_err(|err| err.into())
    {
        Ok(status) if status.success() => {
            let profile = args
                .iter()
                .find(|arg| *arg == "--release")
                .map_or("debug", |_| "release");

            for (to_plugin_file_name, plugin_manifest_dir) in config.extension_plugins.iter() {
                let manifest_file = plugin_manifest_dir.get_ref().join("Cargo.toml");
                let manifest_file_content = fs::read_to_string(&manifest_file).map_err(|err| {
                    anyhow::anyhow!(
                        "Failed to read manifest '{}' for plugin '{}':\n{err:#?}",
                        manifest_file.display(),
                        to_plugin_file_name.as_ref()
                    )
                })?;
                let manifest = validate_plugin(
                    &mut emitter,
                    config.file(),
                    config.contents(),
                    to_plugin_file_name,
                    &manifest_file,
                    &manifest_file_content,
                )?;

                let from_lib_name_with_ext = add_plugin_ext(
                    &manifest
                        .into_inner()
                        .lib
                        .map(|lib| lib.name.unwrap())
                        .unwrap(),
                );
                let to_lib_name_with_ext = add_plugin_ext(to_plugin_file_name.as_ref());
                let plugin_path = project_root
                    .join("target")
                    .join(profile)
                    .join(from_plugin_file_name(&from_lib_name_with_ext));

                if plugin_path.exists() {
                    let plugin_path =
                        rename_plugin(&project_root, profile, &plugin_path, &to_lib_name_with_ext)?;
                    if !no_symlink {
                        symlink_plugin(&plugin_path)?;
                    } else {
                        println!(
                            "{}: plugin was not symlinked ({})",
                            "warning".yellow().bold(),
                            plugin_path.display()
                        );
                    }
                }
            }
            Ok(())
        }
        Ok(status) => {
            process::exit(status.code().unwrap_or(1));
        }
        Err(err) => Err(err),
    }
}

/// Remove extension plugins from the `UserPlugins` directory.
pub(crate) fn clean(
    plugins: &[String],
    dry_run: bool,
    remove_artifacts: bool,
) -> anyhow::Result<()> {
    let project_root = find_project_root()?;
    let config = ReaperPluginConfig::load(&project_root)?;
    let mut emitter = TomlErrorEmitter::<String, String>::new();

    let plugins: collections::HashMap<String, path::PathBuf> = if !plugins.is_empty() {
        let mut map = config.extension_plugins;
        map.retain(|k, _| plugins.contains(k.as_ref()));
        if map.is_empty() {
            anyhow::bail!(
                "The following plugin(s) were not found: {}\n\nTip: run `cargo reaper list` to view the available plugins.",
                plugins.join(", ")
            )
        }
        map.into_iter()
            .map(|(key, val)| (key.into_inner(), val.into_inner()))
            .collect()
    } else {
        config
            .extension_plugins
            .into_iter()
            .map(|(key, val)| (key.into_inner(), val.into_inner()))
            .collect()
    };
    let mut removal_failures = 0;
    for plugin_name in plugins.keys() {
        println!("    {} {}", "Removing".magenta().bold(), plugin_name);
        if let Err(err) = remove_plugin_symlink(plugin_name, &add_plugin_ext(plugin_name), dry_run)
        {
            removal_failures += 1;
            eprintln!("{}: {err}", "error (benign)".magenta());
        }
    }
    println!(
        "     {} {} symlink(s)",
        if dry_run {
            "Summary".green().bold()
        } else {
            "Removed".green().bold()
        },
        plugins.len() - removal_failures
    );

    if remove_artifacts {
        let mut package_args: Vec<String> = Vec::with_capacity(plugins.len());
        for (plugin_name, manifest_dir) in plugins.iter() {
            let manifest_file = manifest_dir.join("Cargo.toml");
            let manifest_file_content = fs::read_to_string(&manifest_file).map_err(|err| {
                anyhow::anyhow!(
                    "Failed to read manifest '{}' for plugin '{}':\n{err:#?}",
                    manifest_file.display(),
                    plugin_name
                )
            })?;
            let mut manifest = toml::Spanned::new(
                0..manifest_file_content.len(),
                cargo_toml::Manifest::from_str(&manifest_file_content).map_err(|err| {
                    anyhow::anyhow!(
                        "Failed to parse plugin manifest '{}':\n{err:#?}",
                        manifest_file.display()
                    )
                })?,
            );
            let _ = manifest
                .as_mut()
                .complete_from_path_and_workspace::<cargo_toml::Value>(&manifest_file, None);
            if let Some(package) = manifest.as_ref().package.as_ref() {
                package_args.extend(["-p".into(), package.name.clone()]);
            } else {
                emitter.insert_err(
                    manifest_file.to_string_lossy().to_string(),
                    manifest_file_content,
                    format!("`{}` is not a package", plugin_name),
                    manifest.span(),
                    Some("expected manifest path to a package containing a dynamic library target"),
                    None,
                    Some(
                        "help: is this a workspace? try adding the `[workspace.package]` attribute",
                    ),
                );
            }
        }
        emitter.emit()?;

        let mut cargo = process::Command::new("cargo");
        let mut cargo_clean = cargo
            .arg("clean")
            .args(&package_args)
            .stdin(process::Stdio::inherit())
            .stdout(process::Stdio::inherit())
            .stderr(process::Stdio::inherit());
        if dry_run {
            cargo_clean = cargo_clean.arg("--dry-run");
        }
        cargo_clean.status()?;
    } else if dry_run {
        println!(
            "{}: no files deleted due to --dry-run",
            "warning".yellow().bold()
        );
    }

    Ok(())
}

/// Rename the resulting extension plugin, returning the new plugin path if it succeeds.
///
/// > Note: This function is platform agnostic
///
/// # Usage
///
/// This is run automatically when running the `cargo reaper` command.
pub(crate) fn _rename_plugin(
    project_root: &path::Path,
    profile: &str,
    old_plugin_path: &path::PathBuf,
    plugin_name_to: &str,
) -> anyhow::Result<path::PathBuf> {
    let new_plugin_path = project_root
        .join("target")
        .join(profile)
        .join(plugin_name_to);

    fs::rename(old_plugin_path, &new_plugin_path)
        .map_err(|err| anyhow::anyhow!("failed to rename plugin: {err:?}"))?;

    println!(
        "    {} plugin renamed {} → {}",
        "Finished".green().bold(),
        old_plugin_path.display(),
        new_plugin_path.display()
    );

    Ok(new_plugin_path)
}

/// Symlink the REAPER extension plugin to the `UserPlugins` directory.
///
/// > Note: This function is platform agnostic
///
/// # Usage
///
/// This is run automatically when running the `cargo reaper build` command, unless passed `--no-symlink`.
pub(crate) fn _symlink_plugin<S>(
    plugin_path: &path::PathBuf,
    user_plugins_dir: &path::Path,
    symlink_plugin: S,
) -> anyhow::Result<()>
where
    S: FnOnce(&path::PathBuf, &path::PathBuf) -> io::Result<()>,
{
    if !user_plugins_dir.exists() {
        anyhow::bail!(
            "The 'UserPlugins' directory must exist before the plugin can be symlinked. Please launch REAPER to initialize the 'UserPlugins' directory and try again."
        );
    }

    let symlink_path = user_plugins_dir.join(plugin_path.file_name().ok_or_else(|| {
        anyhow::anyhow!(
            "Unable to get plugin file name from path '{}'",
            plugin_path.display()
        )
    })?);
    if symlink_path.exists() {
        let currently_symlinked_plugin_path = fs::read_link(&symlink_path)?;
        if &currently_symlinked_plugin_path != plugin_path {
            println!(
                "{}: removing stale symlink ({})",
                "warning".yellow().bold(),
                symlink_path.display()
            );
            fs::remove_file(&symlink_path)?;
        } else {
            println!(
                "    {} symbolic link already exists ({})",
                "Finished".green().bold(),
                symlink_path.display(),
            );
            return Ok(());
        }
    }

    // TODO: Sometimes this will still fail with 'AlreadyExists' errors. We should also go ahead and catch them here.
    symlink_plugin(plugin_path, &symlink_path)
        .map_err(|err| anyhow::anyhow!("failed to link extension plugin: {err:?}"))?;

    println!(
        "    {} symbolic link created {} → {}",
        "Finished".green().bold(),
        symlink_path.display(),
        plugin_path.display()
    );

    Ok(())
}

/// Remove a REAPER extension plugin symlink from the `UserPlugins` directory.
///
/// > Note: This function is platform agnostic
///
/// # Usage
///
/// This is run automatically when running the `cargo reaper clean` command.
pub(crate) fn _remove_plugin_symlink(
    plugin_name: &str,
    plugin_file_name: &str,
    user_plugins_dir: &path::Path,
    dry_run: bool,
) -> anyhow::Result<()> {
    let symlink_path = user_plugins_dir.join(plugin_file_name);
    if symlink_path.is_symlink() {
        if !dry_run {
            fs::remove_file(&symlink_path).map_err(|err| {
                anyhow::anyhow!("failed to remove symlink for `{plugin_name}`:\n{err:#?}")
            })?;
        }
        return Ok(());
    }

    anyhow::bail!(
        "`{}` does not contain a symlink for `{}` ({})",
        user_plugins_dir.display(),
        plugin_name,
        plugin_file_name
    )
}

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
fn from_plugin_file_name(lib_name: &str) -> String {
    lib_name.to_string()
}
#[cfg(target_os = "windows")]
fn add_plugin_ext(lib_name: &str) -> String {
    format!("{lib_name}{WINDOWS_PLUGIN_EXT}")
}
#[cfg(target_os = "windows")]
fn rename_plugin(
    project_root: &path::Path,
    profile: &str,
    old_plugin_path: &path::PathBuf,
    plugin_name_to: &str,
) -> anyhow::Result<path::PathBuf> {
    _rename_plugin(project_root, profile, old_plugin_path, plugin_name_to)
}
#[cfg(target_os = "windows")]
fn symlink_plugin(plugin_path: &path::PathBuf) -> anyhow::Result<()> {
    _symlink_plugin(
        plugin_path,
        &dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to find 'AppData' directory"))?
            .join("REAPER")
            .join("UserPlugins"),
        |plugin_path, symlink_path| {
            os::windows::fs::symlink_file(plugin_path, symlink_path).map_err(|err|
                if format!("{err:?}").contains("A required privilege is not held by the client.") {
                    io::Error::new(io::ErrorKind::PermissionDenied, "Windows treats symlink creation as a privileged action, therefore this function is likely to fail unless the user makes changes to their system to permit symlink creation. Users can try enabling Developer Mode, granting the SeCreateSymbolicLinkPrivilege privilege, or running the process as an administrator.")
                } else {
                    err
                }
            )
        },
    )
}
#[cfg(target_os = "windows")]
pub(crate) fn remove_plugin_symlink(
    plugin_name: &str,
    plugin_file_name: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    _remove_plugin_symlink(
        plugin_name,
        plugin_file_name,
        &dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to find 'AppData' directory"))?
            .join("REAPER")
            .join("UserPlugins"),
        dry_run,
    )
}
#[cfg(target_os = "windows")]
fn run(override_binary: Option<path::PathBuf>) -> anyhow::Result<()> {
    const BINARY_NAME: &str = "reaper";
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
fn from_plugin_file_name(lib_name: &str) -> String {
    format!("lib{lib_name}")
}
#[cfg(target_os = "linux")]
fn add_plugin_ext(lib_name: &str) -> String {
    format!("{lib_name}{LINUX_PLUGIN_EXT}")
}
#[cfg(target_os = "linux")]
fn rename_plugin(
    project_root: &path::Path,
    profile: &str,
    old_plugin_path: &path::PathBuf,
    plugin_name_to: &str,
) -> anyhow::Result<path::PathBuf> {
    _rename_plugin(project_root, profile, old_plugin_path, plugin_name_to)
}
#[cfg(target_os = "linux")]
fn symlink_plugin(plugin_path: &path::PathBuf) -> anyhow::Result<()> {
    _symlink_plugin(
        plugin_path,
        &dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to find '.config' directory"))?
            .join("REAPER")
            .join("UserPlugins"),
        |plugin_path, symlink_path| os::unix::fs::symlink(plugin_path, symlink_path),
    )
}
#[cfg(target_os = "linux")]
pub(crate) fn remove_plugin_symlink(
    plugin_name: &str,
    plugin_file_name: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    _remove_plugin_symlink(
        plugin_name,
        plugin_file_name,
        &dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to find '.config' directory"))?
            .join("REAPER")
            .join("UserPlugins"),
        dry_run,
    )
}
#[cfg(target_os = "linux")]
fn run(override_binary: Option<path::PathBuf>) -> anyhow::Result<()> {
    const BINARY_NAME: &str = "reaper";
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
fn from_plugin_file_name(lib_name: &str) -> String {
    format!("lib{lib_name}")
}
#[cfg(target_os = "macos")]
fn add_plugin_ext(lib_name: &str) -> String {
    format!("{lib_name}{DARWIN_PLUGIN_EXT}")
}
#[cfg(target_os = "macos")]
fn rename_plugin(
    project_root: &path::Path,
    profile: &str,
    old_plugin_path: &path::PathBuf,
    plugin_name_to: &str,
) -> anyhow::Result<path::PathBuf> {
    _rename_plugin(project_root, profile, old_plugin_path, plugin_name_to)
}
#[cfg(target_os = "macos")]
fn symlink_plugin(plugin_path: &path::PathBuf) -> anyhow::Result<()> {
    _symlink_plugin(
        plugin_path,
        &dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to find 'Users' directory"))?
            .join("Library")
            .join("Application Support")
            .join("REAPER")
            .join("UserPlugins"),
        |plugin_path, symlink_path| os::unix::fs::symlink(plugin_path, symlink_path),
    )
}
#[cfg(target_os = "macos")]
pub(crate) fn remove_plugin_symlink(
    plugin_name: &str,
    plugin_file_name: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    _remove_plugin_symlink(
        plugin_name,
        plugin_file_name,
        &dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to find 'Users' directory"))?
            .join("Library")
            .join("Application Support")
            .join("REAPER")
            .join("UserPlugins"),
        dry_run,
    )
}
#[cfg(target_os = "macos")]
fn run(override_binary: Option<path::PathBuf>) -> anyhow::Result<()> {
    const BINARY_NAME: &str = "reaper";
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

fn find_project_root() -> anyhow::Result<path::PathBuf> {
    let mut current_dir = env::current_dir()?;

    loop {
        if current_dir.join("Cargo.toml").is_file() && current_dir.join(".reaper.toml").is_file()
            || current_dir.join("reaper.toml").is_file()
        {
            return Ok(current_dir);
        }

        if !current_dir.pop() {
            break;
        }
    }

    anyhow::bail!(
        "Unable to find project root directory. Please ensure a reaper.toml or .reaper.toml file is present in the project root, and try again."
    )
}

/// Downloads and initializes the REAPER extension plugin template.
pub async fn new_from_template(
    destination: &path::PathBuf,
    package_name: &str,
) -> anyhow::Result<()> {
    const TEMPLATE_REPO_URL: &str =
        "https://github.com/helgoboss/reaper-rs-hello-world-extension/archive/refs/heads/main.zip";
    let reaper_toml = format!(
        "# Define the desired name and path to a directory containing a Cargo.toml for each extension plugin.
# Extension plugin names must start with `reaper_` or they will not be recognized by REAPER.

[extension_plugins]
{package_name} = \"./.\"
"
    );

    let response = reqwest::get(TEMPLATE_REPO_URL).await?;
    let mut archive_bytes = io::Cursor::new(response.bytes().await?);

    let temp_dir = tempfile::tempdir()?;
    let mut zip = zip::ZipArchive::new(&mut archive_bytes)?;
    zip.extract(&temp_dir)?;

    let inner_folder = fs::read_dir(&temp_dir)?
        .filter_map(Result::ok)
        .find(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|entry| entry.path())
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to find extracted folder containing extension plugin template")
        })?;

    fs::rename(&inner_folder, destination)?;

    let git_dir = destination.join(".git");
    if git_dir.exists() {
        fs::remove_dir_all(git_dir)?;
    }
    let gitignore = destination.join(".gitignore");
    if gitignore.exists() {
        fs::remove_file(&gitignore)?;
    }

    let cargo_toml_path = destination.join("Cargo.toml");
    let cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    let mut doc = cargo_toml.parse::<toml_edit::DocumentMut>()?;

    if let Some(package) = doc.get_mut("package") {
        if let Some(name) = package.get_mut("name") {
            *name = toml_edit::value(package_name);
        }
    }
    if let Some(lib) = doc.get_mut("lib") {
        if let Some(name) = lib.get_mut("name") {
            *name = toml_edit::value(package_name);
        }
    }

    fs::write(&cargo_toml_path, doc.to_string())?;
    fs::write(destination.join("reaper.toml"), reaper_toml)?;
    fs::write(&gitignore, "/target")?;

    gix::init(destination).map_err(|err| {
        anyhow::anyhow!(
            "failed to initialize REAPER extension plugin project as a git repository: {err:?}"
        )
    })?;

    Ok(())
}
