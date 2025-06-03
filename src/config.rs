use std::{collections, fs, path};

/// Acceptable plugin config toml names for renaming and symlinking REAPER extenion plugins built with Rust.
pub(crate) const CONFIG_FILE_NAMES: &[&str; 2] = &[".reaper.toml", "reaper.toml"];

/// The parsed contents of a `reaper.toml` config file.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct ReaperPluginConfig {
    /// The path to the `reaper.toml` config file.
    #[serde(skip)]
    file: path::PathBuf,

    /// The contents of a `reaper.toml` config file as a [`std::string::String`].
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

    /// The available extension plugins listed in the config file.
    pub(crate) fn extension_plugins(
        &self,
    ) -> &collections::HashMap<toml::Spanned<String>, toml::Spanned<path::PathBuf>> {
        &self.extension_plugins
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
