use std::{env, fmt, fs, io, path};

pub(crate) use colored::Colorize;

use crate::{
    cli::PluginTemplate,
    error::{Message, TomlErrorEmitter},
};

/// The REAPER executable binary name.
pub(crate) const BINARY_NAME: &str = "reaper";

impl PluginTemplate {
    /// The extension plugin template directory
    const EXT: include_dir::Dir<'_> = include_dir::include_dir!("templates/extension");

    /// The vst plugin template directory
    const VST: include_dir::Dir<'_> = include_dir::include_dir!("templates/vst");

    /// Create directories and extract all files to real filesystem.
    /// Creates parent directories of `path` if they do not already exist.
    /// Fails if some files already exist. In case of error, partially extracted directory may remain on the filesystem.
    pub(crate) fn extract<S: AsRef<path::Path>>(&self, base_path: S) -> io::Result<()> {
        match self {
            Self::Ext => Self::EXT.extract(base_path),
            Self::Vst => Self::VST.extract(base_path),
        }
    }
}

/// Represents a REAPER plugin's manifest information.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PluginManifest {
    name: String,
    version: String,
    authors: Vec<String>,
    description: Option<String>,
}
impl PluginManifest {
    pub(crate) fn new(
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

pub(crate) fn find_project_root() -> anyhow::Result<path::PathBuf> {
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
        "Unable to find project root directory. Please ensure a `reaper.toml` or `.reaper.toml` file is present in the project root, and try again."
    )
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

/// Handles locating the REAPER default installation path.
/// Global location method varies for each operating system, i.e. Linux does not have a default global install location.
///
/// > Note: This function is platform agnostic
///
/// # Usage
///
/// This is the inner method of `locate_global_default` defined for each operating system in `crate::util::os`:
///
/// ```rust
/// use crate::util::os::locate_global_default;
///
/// assert!(locate_global_default().is_ok());
/// ```
pub(crate) fn _locate_global_default<G>(run_global_locator_method: G) -> io::Result<path::PathBuf>
where
    G: FnOnce() -> Option<path::PathBuf>,
{
    run_global_locator_method().ok_or(io::Error::new(
        io::ErrorKind::NotFound,
        "Unable to locate REAPER executable — Run `cargo reaper -h` for help, or override the default executable path with `--exec <PATH>`.",
    ))
}

/// Rename the resulting extension plugin, returning the new plugin path if it succeeds.
///
/// > Note: This function is platform agnostic
///
/// # Usage
///
/// This is run automatically when running the `cargo reaper build` command.
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
        "     {} {} → {}",
        "Renamed".green().bold(),
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
                "Skipping".yellow().bold(),
                symlink_path.display(),
            );
            return Ok(());
        }
    }

    // TODO: Sometimes this will still fail with 'AlreadyExists' errors. We should also go ahead and catch them here.
    symlink_plugin(plugin_path, &symlink_path)
        .map_err(|err| anyhow::anyhow!("failed to link extension plugin: {err:?}"))?;

    println!(
        "     {} symbolic link {} → {}",
        "Created".green().bold(),
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
pub(crate) mod os {
    //! Operating system specific functionality for handling operations which require knownledge of
    //! either dynamic library file extensions, or interacting with the `UserPlugins` directory.

    use std::{io, os, path};

    use super::{_locate_global_default, _remove_plugin_symlink, _rename_plugin, _symlink_plugin};

    /// The dynamically linked C library Windows extension
    pub(crate) const WINDOWS_PLUGIN_EXT: &str = ".dll";

    /// The global default REAPER executable file path for `x86_64-windows` (64bit)
    #[cfg(target_arch = "x86_64")]
    pub(crate) const GLOBAL_DEFAULT_PATH: &str = r"C:\Program Files\REAPER (x64)\reaper.exe";

    /// The global default REAPER executable file path for `x86-windows` (32bit)
    #[cfg(target_arch = "x86")]
    pub(crate) const GLOBAL_DEFAULT_PATH: &str = r"C:\Program Files (x86)\REAPER\reaper.exe";

    /// The global default REAPER executable file path for `aarch64-windows` (ARM64)
    #[cfg(target_arch = "aarch64")]
    pub(crate) const GLOBAL_DEFAULT_PATH: &str = r"C:\Program Files\REAPER (ARM64)\reaper.exe";

    pub(crate) fn locate_global_default() -> io::Result<path::PathBuf> {
        _locate_global_default(|| {
            let reaper = path::PathBuf::from(GLOBAL_DEFAULT_PATH);
            (reaper.exists()).then_some(reaper)
        })
    }

    pub(crate) fn from_plugin_file_name(lib_name: &str) -> String {
        lib_name.to_string()
    }

    pub(crate) fn add_plugin_ext(lib_name: &str) -> String {
        format!("{lib_name}{WINDOWS_PLUGIN_EXT}")
    }

    pub(crate) fn rename_plugin(
        project_root: &path::Path,
        profile: &str,
        old_plugin_path: &path::PathBuf,
        plugin_name_to: &str,
    ) -> anyhow::Result<path::PathBuf> {
        _rename_plugin(project_root, profile, old_plugin_path, plugin_name_to)
    }

    pub(crate) fn symlink_plugin(plugin_path: &path::PathBuf) -> anyhow::Result<()> {
        _symlink_plugin(
            plugin_path,
            &dirs::data_dir()
                .ok_or_else(|| anyhow::anyhow!("Unable to find 'AppData' directory"))?
                .join("REAPER")
                .join("UserPlugins"),
            |plugin_path, symlink_path| {
                os::windows::fs::symlink_file(plugin_path, symlink_path).map_err(|err|
                    if format!("{err:?}").contains("A required privilege is not held by the client.") {
                        io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            "Windows treats symlink creation as a privileged action, therefore this function is likely to fail unless the user makes changes to their system to permit symlink creation. Users can try enabling Developer Mode, granting the SeCreateSymbolicLinkPrivilege privilege, or running the process as an administrator.",
                        )
                    } else {
                        err
                    }
                )
            },
        )
    }

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
}

#[cfg(target_os = "linux")]
pub(crate) mod os {
    //! Operating system specific functionality for handling operations which require knownledge of
    //! either dynamic library file extensions, or interacting with the `UserPlugins` directory.

    use std::{io, os, path};

    use super::{
        _locate_global_default, _remove_plugin_symlink, _rename_plugin, _symlink_plugin,
        BINARY_NAME,
    };

    /// The dynamically linked C library Linux extension
    pub(crate) const LINUX_PLUGIN_EXT: &str = ".so";

    pub(crate) fn locate_global_default() -> io::Result<path::PathBuf> {
        _locate_global_default(|| which::which_global(BINARY_NAME).ok())
    }

    pub(crate) fn from_plugin_file_name(lib_name: &str) -> String {
        format!("lib{lib_name}")
    }

    pub(crate) fn add_plugin_ext(lib_name: &str) -> String {
        format!("{lib_name}{LINUX_PLUGIN_EXT}")
    }

    pub(crate) fn rename_plugin(
        project_root: &path::Path,
        profile: &str,
        old_plugin_path: &path::PathBuf,
        plugin_name_to: &str,
    ) -> anyhow::Result<path::PathBuf> {
        _rename_plugin(project_root, profile, old_plugin_path, plugin_name_to)
    }

    pub(crate) fn symlink_plugin(plugin_path: &path::PathBuf) -> anyhow::Result<()> {
        _symlink_plugin(
            plugin_path,
            &dirs::config_dir()
                .ok_or_else(|| anyhow::anyhow!("Unable to find '.config' directory"))?
                .join("REAPER")
                .join("UserPlugins"),
            |plugin_path, symlink_path| os::unix::fs::symlink(plugin_path, symlink_path),
        )
    }

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
}

#[cfg(target_os = "macos")]
pub(crate) mod os {
    //! Operating system specific functionality for handling operations which require knownledge of
    //! either dynamic library file extensions, or interacting with the `UserPlugins` directory.

    use std::{io, os, path};

    use super::{_locate_global_default, _remove_plugin_symlink, _rename_plugin, _symlink_plugin};

    /// The dynamically linked C library MacOS (Darwin) extension
    pub(crate) const DARWIN_PLUGIN_EXT: &str = ".dylib";

    /// The global default REAPER executable file path for `x86_64-darwin` (Intel) and `aarch64-darwin` (Apple Silicon)
    #[cfg(target_os = "macos")]
    pub(crate) const GLOBAL_DEFAULT_PATH: &str = "/Applications/REAPER.app/Contents/MacOS/REAPER";

    pub(crate) fn locate_global_default() -> io::Result<path::PathBuf> {
        _locate_global_default(|| {
            let reaper = path::PathBuf::from(GLOBAL_DEFAULT_PATH);
            (reaper.exists()).then_some(reaper)
        })
    }

    pub(crate) fn from_plugin_file_name(lib_name: &str) -> String {
        format!("lib{lib_name}")
    }

    pub(crate) fn add_plugin_ext(lib_name: &str) -> String {
        format!("{lib_name}{DARWIN_PLUGIN_EXT}")
    }

    pub(crate) fn rename_plugin(
        project_root: &path::Path,
        profile: &str,
        old_plugin_path: &path::PathBuf,
        plugin_name_to: &str,
    ) -> anyhow::Result<path::PathBuf> {
        _rename_plugin(project_root, profile, old_plugin_path, plugin_name_to)
    }

    pub(crate) fn symlink_plugin(plugin_path: &path::PathBuf) -> anyhow::Result<()> {
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
}
