use std::fs;

use crate::{
    config::ReaperPluginConfig,
    error::TomlErrorEmitter,
    util::{Colorize, PluginManifest, find_project_root, validate_plugin},
};

/// Print available extension plugins to stdout.
pub(crate) fn list() -> anyhow::Result<()> {
    let config = ReaperPluginConfig::load(&find_project_root()?)?;
    let mut emitter = TomlErrorEmitter::<String, String>::new();
    let mut plugins: Vec<String> = Vec::new();
    for (plugin_name, manifest_dir) in config.extension_plugins().iter() {
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
