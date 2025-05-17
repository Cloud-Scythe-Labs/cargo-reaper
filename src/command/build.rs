use std::process;

use crate::{
    error::TomlErrorEmitter,
    util::{Colorize, ReaperPluginConfig, validate_plugin},
};

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
