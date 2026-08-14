final: prev: {
  rustPlatform = prev.rustPlatform.overrideScope (rfinal: rprev: {
    rustToolchain = prev.fenix.stable.withComponents [
      "cargo"
      "rustfmt"
      "clippy"
      "rust-src"
      "rust-analyzer"
    ];
  });
}
