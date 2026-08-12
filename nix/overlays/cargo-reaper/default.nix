final: prev:
let
  inherit (prev) lib rustPlatform;

  buildRustPackage =
    if (rustPlatform ? buildCranePackage) then
      rustPlatform.buildCranePackage
    else
      rustPlatform.buildRustPackage
    ;
in
{
  cargo-reaper = buildRustPackage ({
    src = lib.cleanSourceWith {
      inherit ((builtins.fromTOML (builtins.readFile (src + "/Cargo.toml"))).package) name;
      src = lib.cleanSource ../../../.;
      filter = orig_path: type:
        let
          path = (toString orig_path);
          base = baseNameOf path;
          parentDir = baseNameOf (dirOf path);
          matchesSuffix = lib.any (suffix: lib.hasSuffix suffix base) [
            ".rs"
            ".toml"
          ];
          isCargoFile = base == "Cargo.lock";
          isCargoConfig = parentDir == ".cargo" && base == "config";
        in
        type == "directory" || matchesSuffix || isCargoFile || isCargoConfig;
    };

    strictDeps = true;
    __structuredAttrs = true;

    nativeBuildInputs = with prev; [
      installShellFiles
    ] ++ lib.optionals stdenv.isLinux [
      autoPatchelfHook
    ];

    buildInputs = with prev; [
      libgcc
    ] ++ lib.optionals stdenv.isDarwin [
      libiconv
    ];

    # NOTE: `installShellCompletion` only has support for Bash, Zsh and Fish
    postInstall = ''
      installShellCompletion --cmd cargo-reaper \
        --bash <($out/bin/cargo-reaper completions bash) \
        --fish <($out/bin/cargo-reaper completions fish) \
        --zsh <($out/bin/cargo-reaper completions zsh)
    '';
    doCheck = false;
  }

  // lib.optionalAttrs (rustPlatform ? buildDepsOnly) {
    cargoArtifacts = rustPlatform.buildDepsOnly commonArgs;
  });
}
