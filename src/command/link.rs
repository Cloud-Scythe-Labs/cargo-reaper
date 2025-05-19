use std::path;

use crate::{
    config::ReaperPluginConfig,
    util::{find_project_root, os::symlink_plugin},
};

pub(crate) fn link(plugins: Vec<String>, paths: Vec<path::PathBuf>) -> anyhow::Result<()> {
    let project_root = find_project_root()?;
    let config = ReaperPluginConfig::load(&project_root)?;

    let mut map = config.extension_plugins().to_owned();
    map.retain(|k, _| plugins.contains(k.as_ref()));
    if map.is_empty() {
        anyhow::bail!(
            "The following plugin(s) were not found: {}\n\nTip: run `cargo reaper list` to view the available plugins.",
            plugins.join(", ")
        )
    }
    let mut plugin_paths: Vec<path::PathBuf> = map
        .into_keys()
        .filter_map(|_| {
            // ensure plugin name is in the config, and try to find the plugin in the target directory
            // convert the name into a path
            Some(path::PathBuf::new())
        })
        .collect();

    plugin_paths.extend(paths.into_iter().filter_map(|p| match p.canonicalize() {
        Ok(path) => Some(path),
        Err(err) => {
            eprintln!(
                "error: failed to canonicalize path `{}`:\n\n{err:#?}",
                p.display()
            );
            None
        }
    }));
    for plugin_path in plugin_paths.iter() {
        if let Err(err) = symlink_plugin(plugin_path) {
            eprintln!(
                "error: failed to symlink `{}` to the `UserPlugins` directory:\n\n{err:#?}",
                plugin_path.display()
            )
        }
    }
    Ok(())
}
