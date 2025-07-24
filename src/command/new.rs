use std::{fs, path};

use crate::{cli::PluginTemplate, util::Colorize};

pub(crate) async fn new(template: PluginTemplate, path: path::PathBuf) -> anyhow::Result<()> {
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
        "    {} dynamically linked library (cdylib) `{}` REAPER {:?} plugin package",
        "Creating".green().bold(),
        package_name,
        &template
    );
    new_from_template(template, &path, &package_name)
        .await
        .map_err(|err| anyhow::anyhow!("failed to create new REAPER plugin project: {err:?}"))
}

/// Downloads and initializes the REAPER extension plugin template.
pub(crate) async fn new_from_template(
    template: PluginTemplate,
    destination: &path::PathBuf,
    package_name: &str,
) -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    template.extract(&temp_dir)?;

    fs::rename(&temp_dir.path(), destination)?;

    let cargo_toml_path = destination.join("Cargo.toml");
    let mut cargo_toml = fs::read_to_string(&cargo_toml_path)?.parse::<toml_edit::DocumentMut>()?;
    if let Some(package) = cargo_toml.get_mut("package") {
        if let Some(name) = package.get_mut("name") {
            *name = toml_edit::value(package_name);
        }
    }
    if let Some(lib) = cargo_toml.get_mut("lib") {
        if let Some(name) = lib.get_mut("name") {
            *name = toml_edit::value(package_name);
        }
    }

    let reaper_toml_path = destination.join("reaper.toml");
    let mut reaper_toml =
        fs::read_to_string(&reaper_toml_path)?.parse::<toml_edit::DocumentMut>()?;
    if let Some(extension_plugins) = reaper_toml
        .get_mut("extension_plugins")
        .map(toml_edit::Item::as_table_mut)
        .flatten()
    {
        extension_plugins.insert(
            &(package_name.starts_with("reaper_"))
                .then(|| package_name.into())
                .unwrap_or_else(|| format!("reaper_{package_name}")),
            toml_edit::value("./."),
        );
    }

    fs::write(&cargo_toml_path, cargo_toml.to_string())?;
    fs::write(&reaper_toml_path, reaper_toml.to_string())?;
    fs::write(destination.join(".gitignore"), "/target")?;

    gix::init(destination).map_err(|err| {
        anyhow::anyhow!("failed to initialize REAPER plugin project as a git repository: {err:?}")
    })?;

    Ok(())
}
