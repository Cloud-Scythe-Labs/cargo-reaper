// Generates shell completions in `/target/completions`.
//
// See https://docs.rs/clap_complete/latest/clap_complete/index.html for more information.

include!("src/cli.rs");

fn main() {
    let outdir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("completions");
    println!("cargo::rerun-if-changed=src/cli.rs");
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=${}", outdir.display());

    if let Err(err) = std::fs::create_dir_all(&outdir) {
        println!("cargo:warning=failed to create completions dir: {err:?}")
    }

    let mut cmd = CargoReaperArgs::command();
    for &shell in clap_complete::Shell::value_variants() {
        if let Err(err) = clap_complete::generate_to(shell, &mut cmd, "cargo-reaper", &outdir) {
            println!("cargo:warning=failed to create completion script for {shell}: {err:?}")
        }
    }
}
