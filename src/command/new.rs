use std::{fs, io, path};

use crate::util::Colorize;

pub(crate) async fn new(path: path::PathBuf) -> anyhow::Result<()> {
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
        "    {} dynamically linked library (cdylib) `{}` REAPER extension plugin package",
        "Creating".green().bold(),
        package_name
    );
    new_from_template(&path, &package_name)
        .await
        .map_err(|err| {
            anyhow::anyhow!("failed to create new REAPER extension plugin project: {err:?}")
        })
}

/// Downloads and initializes the REAPER extension plugin template.
pub(crate) async fn new_from_template(
    destination: &path::PathBuf,
    package_name: &str,
) -> anyhow::Result<()> {
    const TEMPLATE_REPO_URL: &str =
        "https://github.com/helgoboss/reaper-rs-hello-world-extension/archive/refs/heads/main.zip";
    let plugin_key = if package_name.starts_with("reaper_") {
        package_name.into()
    } else {
        format!("reaper_{package_name}")
    };
    let reaper_toml = format!(
        "# Define the desired name and path to a directory containing a Cargo.toml for each extension plugin.
# Extension plugin names must start with `reaper_` or they will not be recognized by REAPER.

[extension_plugins]
{plugin_key} = \"./.\"
"
    );

    let response = reqwest::get(TEMPLATE_REPO_URL).await?;
    let mut archive_bytes = io::Cursor::new(response.bytes().await?);

    let temp_dir = tempfile::tempdir()?;
    let mut zip = zip::ZipArchive::new(&mut archive_bytes)?;
    zip.extract(&temp_dir)?;

    let inner_folder = fs::read_dir(&temp_dir)?
        .filter_map(Result::ok)
        .find(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|entry| entry.path())
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to find extracted folder containing extension plugin template")
        })?;

    fs::rename(&inner_folder, destination)?;

    let git_dir = destination.join(".git");
    if git_dir.exists() {
        fs::remove_dir_all(git_dir)?;
    }
    let gitignore = destination.join(".gitignore");
    if gitignore.exists() {
        fs::remove_file(&gitignore)?;
    }

    let cargo_toml_path = destination.join("Cargo.toml");
    let cargo_toml = fs::read_to_string(&cargo_toml_path)?;
    let mut doc = cargo_toml.parse::<toml_edit::DocumentMut>()?;

    if let Some(package) = doc.get_mut("package") {
        if let Some(name) = package.get_mut("name") {
            *name = toml_edit::value(package_name);
        }
    }
    if let Some(lib) = doc.get_mut("lib") {
        if let Some(name) = lib.get_mut("name") {
            *name = toml_edit::value(package_name);
        }
    }

    fs::write(&cargo_toml_path, doc.to_string())?;
    fs::write(destination.join("reaper.toml"), reaper_toml)?;
    fs::write(&gitignore, "/target")?;

    gix::init(destination).map_err(|err| {
        anyhow::anyhow!(
            "failed to initialize REAPER extension plugin project as a git repository: {err:?}"
        )
    })?;

    Ok(())
}
