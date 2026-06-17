use std::{env, fs, process};

use crate::{
    config::ReaperPluginConfig,
    error::TomlErrorEmitter,
    util::{
        Colorize, TargetOs, find_project_root, os::symlink_plugin, rename_plugin, validate_plugin,
    },
};

/// Build a REAPER extension plugin.
pub(crate) fn build(no_symlink: bool, args: Vec<String>) -> anyhow::Result<()> {
    let project_root = find_project_root()?;
    let config = ReaperPluginConfig::load(&project_root)?;
    let mut emitter = TomlErrorEmitter::<String, String>::new();

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

            let target_triple = args
                .iter()
                .position(|arg| arg == "--target")
                .and_then(|pos| args.get(pos + 1))
                .cloned()
                .or_else(|| env::var("CARGO_BUILD_TARGET").ok());
            let target_os = target_triple
                .as_deref()
                .and_then(TargetOs::from_triple)
                .unwrap_or_else(TargetOs::host);

            for (to_plugin_file_name, plugin_manifest_dir) in config.extension_plugins().iter() {
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

                let lib_name = manifest
                    .into_inner()
                    .lib
                    .map(|lib| lib.name.unwrap())
                    .unwrap();

                // Cargo's output filename: lib<name>.so / lib<name>.dylib / <name>.dll
                let from_lib_name_with_ext = target_os.add_plugin_ext(&lib_name);
                let from_lib_file_name = target_os.plugin_file_name(&from_lib_name_with_ext);

                // Desired output filename: reaper_<name>.so / .dylib / .dll
                let to_lib_name_with_ext = target_os.add_plugin_ext(to_plugin_file_name.as_ref());

                // Cross builds land in target/{triple}/{profile}/; native in target/{profile}/
                let profile_path = target_triple
                    .iter()
                    .fold(project_root.join("target"), |plugin_path, target_triple| {
                        plugin_path.join(target_triple)
                    })
                    .join(profile);
                let plugin_path = profile_path.join(&*from_lib_file_name);

                if plugin_path.exists() {
                    let plugin_path =
                        rename_plugin(&plugin_path, profile_path.join(to_lib_name_with_ext))?;
                    if target_triple.is_some() {
                        println!(
                            "{}: skipping symlink — cross compilation target specified ({})",
                            "warning".yellow().bold(),
                            plugin_path.display()
                        );
                    } else if !no_symlink {
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
