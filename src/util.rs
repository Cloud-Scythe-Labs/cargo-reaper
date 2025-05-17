use std::{collections, fmt, fs, path};

pub(crate) use colored::Colorize;

use crate::error::{Message, TomlErrorEmitter};

/// Acceptable plugin config toml names for renaming and symlinking REAPER extenion plugins built with Rust.
pub(crate) const CONFIG_FILE_NAMES: &[&str; 2] = &[".reaper.toml", "reaper.toml"];

/// The dynamically linked C library Windows extension
pub(crate) const WINDOWS_PLUGIN_EXT: &str = ".dll";
/// The dynamically linked C library Linux extension
pub(crate) const LINUX_PLUGIN_EXT: &str = ".so";
/// The dynamically linked C library MacOS (Darwin) extension
pub(crate) const DARWIN_PLUGIN_EXT: &str = ".dylib";

/// The parsed contents of a `reaper.toml` config file.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct ReaperPluginConfig {
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
    pub(crate) fn file(&self) -> &path::PathBuf {
        &self.file
    }

    /// The path to the `reaper.toml` config file.
    pub(crate) fn contents(&self) -> &str {
        &self.contents
    }

    /// Locate and deserialize a `reaper.toml` config file.
    pub(crate) fn load(project_root: &path::Path) -> anyhow::Result<Self> {
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
