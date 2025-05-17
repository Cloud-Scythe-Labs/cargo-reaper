use std::{collections, fs, path, process};

use crate::{
    error::TomlErrorEmitter,
    util::{Colorize, ReaperPluginConfig},
};

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
