{
  description = "A Cargo plugin for developing REAPER extension and VST plugins with Rust.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    { self
    , nixpkgs
    , crane
    , fenix
    , advisory-db
    , ...
    }:
    let
      overlays = [
        fenix.overlays.default
        (final: prev:
          let
            inherit (prev) lib;
            cargoReaperLib = self.mkLib {
              inherit lib;
              inherit (final) cargo-reaper;
            };

            rustToolchain = prev.fenix.stable.withComponents [
              "cargo"
              "rustfmt"
              "clippy"
              "rust-src"
              "rust-analyzer"
            ];
            craneLib =
              let
                craneLib = (crane.mkLib prev).overrideToolchain rustToolchain;
              in
              craneLib // (cargoReaperLib.crane { inherit craneLib; });

            src = craneLib.cleanCargoSource ./.;

            commonArgs = {
              inherit src;
              strictDeps = true;

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
            };

            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          in
          {
            cargo-reaper = craneLib.buildPackage (commonArgs // {
              inherit cargoArtifacts;
              # NOTE: `installShellCompletion` only has support for Bash, Zsh and Fish
              postInstall = ''
                installShellCompletion --cmd cargo-reaper \
                  --bash <($out/bin/cargo-reaper completions bash) \
                  --fish <($out/bin/cargo-reaper completions fish) \
                  --zsh <($out/bin/cargo-reaper completions zsh)
              '';
              doCheck = false;
            });

            cargoReaper = lib.makeScope prev.newScope (self: (cargoReaperLib // {
              inherit craneLib commonArgs cargoArtifacts src rustToolchain;
            }));
          })
      ];

      pkgsFor = system: import nixpkgs {
        inherit system overlays;
        config = {
          allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) [
            "reaper"
            "win-sdk"
            "xwin-fetch-msvc"
          ];
          microsoftVisualStudioLicenseAccepted = true;
        };
      };

      eachSystem = f: nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ]
        (system: f (pkgsFor system));

      
    in
    {
      mkLib = import ./lib;

      
    };
}
