#![allow(dead_code)]

use std::{env, fs, io, os, path};

use crate::{
    cli::{CargoReaperArgs, CargoReaperCommand, Parser},
    command::{build::build, clean::clean, run::run},
    error::TomlErrorEmitter,
    util::{Colorize, PluginManifest, ReaperPluginConfig, validate_plugin},
};

pub(crate) mod cli;
pub(crate) mod command;
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
            let config = ReaperPluginConfig::load(&find_project_root()?)?;
            let mut emitter = TomlErrorEmitter::<String, String>::new();
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
