use std::path;

use crate::util::{Colorize, os::symlink_plugin};

pub(crate) fn link(paths: Vec<path::PathBuf>) -> anyhow::Result<()> {
    paths
        .into_iter()
        .filter_map(|p| match p.canonicalize() {
            Ok(path) => Some(path),
            Err(err) => {
                eprintln!(
                    "{}: failed to canonicalize path `{}`:\n\n{err:#?}",
                    "error".magenta(),
                    p.display()
                );
                None
            }
        })
        .for_each(|plugin_path| {
            if let Err(err) = symlink_plugin(&plugin_path) {
                eprintln!(
                    "{}: failed to symlink `{}` to the `UserPlugins` directory:\n\n{err:#?}",
                    "error".magenta(),
                    plugin_path.display()
                )
            }
        });
    Ok(())
}
