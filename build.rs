include!("src/cli.rs");

fn main() -> std::io::Result<()> {
    let outdir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("completions");

    let mut cmd = CargoReaperArgs::command();
    for &shell in clap_complete::Shell::value_variants() {
        clap_complete::generate_to(shell, &mut cmd, "cargo-reaper", &outdir)?;
    }

    Ok(())
}
