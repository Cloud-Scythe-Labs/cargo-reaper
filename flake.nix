{
  description = "A Cargo plugin for developing REAPER extension plugins with Rust.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    nix-core = {
      url = "github:Cloud-Scythe-Labs/nix-core";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.fenix.follows = "fenix";
    };

    flake-utils.url = "github:numtide/flake-utils";

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
    , nix-core
    , flake-utils
    , advisory-db
    , ...
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        config.allowUnfree = true;
      };

      inherit (pkgs) lib;

      rustToolchain = nix-core.toolchains.${system}.mkRustToolchainFromTOML
        ./.rust-toolchain.toml
        "sha256-KUm16pHj+cRedf8vxs/Hd2YWxpOrWZ7UOrwhILdSJBU=";
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain.fenix-pkgs;
      src = craneLib.cleanCargoSource ./.;

      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit src;
        strictDeps = true;

        buildInputs = [
          rustToolchain.darwin-pkgs
        ];
      };

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # Build the actual crate itself, reusing the dependency
      # artifacts from above.
      cargo-reaper-drv = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
        doCheck = false;
      });
    in
    {
      checks =
        let
          buildReaperExtension = { package, plugin ? package, ... }@crateArgs: 
            craneLib.buildPackage (crateArgs // {
              pname = package;
              nativeBuildInputs = (crateArgs.nativeBuildInputs or []) ++ [
                # Add `cargo-reaper` as a build time dependency of this derivation.
                self.packages.${system}.default
              ];
              # Run `cargo-reaper`, passing trailing args to the cargo invocation.
              # We do not symlink the plugin since the `UserPlugins` directory is in
              # the `$HOME` directory which is inaccessible to the sandbox.
              buildPhaseCargoCommand = ''
                cargo reaper build --no-symlink \
                  -p ${package} --lib \
                  --release
              '';
              # Include extension plugin in the build result.
              installPhaseCommand = ''
                mkdir -p $out/lib
                mv target/release/${plugin}.* $out/lib
              '';
              # Bypass crane checks for target install paths.
              doNotPostBuildInstallCargoBinaries = true;
            });
          testArgs = src: {
            inherit src;
            version = "0.1.0";
            strictDeps = true;
          };
        in
        {
          # Build the crate as part of `nix flake check` for convenience
          inherit cargo-reaper-drv;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          cargo-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          cargo-doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          cargo-fmt = craneLib.cargoFmt {
            inherit src;
          };

          taplo-fmt = craneLib.taploFmt {
            src = lib.sources.sourceFilesBySuffices src [ ".toml" ];
          };

          # Audit dependencies
          cargo-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Audit licenses
          cargo-deny = craneLib.cargoDeny {
            inherit src;
          };

          # Run tests with cargo-nextest
          cargo-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
            cargoNextestPartitionsExtraArgs = "--no-tests=warn";
          });

          test-cargo-reaper-build =
            let
              root = ./tests/test_data/package_manifest;
              src = lib.fileset.toSource {
                inherit root;
                fileset = lib.fileset.unions [
                  (root + "/Cargo.toml")
                  (root + "/Cargo.lock")
                  (root + "/reaper.toml")
                  (root + "/src")
                ];
              };
              cargoReaperBuildArgs = (testArgs (craneLib.cleanCargoSources root)) // { inherit src; };
            in
            buildReaperExtension (cargoReaperBuildArgs // {
              package = "package_extension";
              plugin = "reaper_package_ext";
            });
        };

      # These checks require `--option sandbox false`.
      checks-no-sandbox = {
        # TODO: add a `--offline` feature to `cargo reaper new`, then
        # pre-populate the cargo temp directory using `fetchFromGithub`.
        # Once we can invoke it in offline mode, we can move this back to checks.
        test-cargo-reaper-new = pkgs.stdenv.mkDerivation {
          name = "test-cargo-reaper-new";
          buildInputs = [
            rustToolchain.fenix-pkgs
            self.packages.${system}.default
          ];
          doCheck = true;
          phases = [
            "buildPhase"
            "checkPhase"
            "installPhase"
          ];
          buildPhase = ''
            cargo reaper new reaper_test
          '';
          checkPhase = ''
            if [ ! -d "reaper_test" ]; then
              exit 1
            fi
          '';
          installPhase = ''
            mkdir -p $out
            mv reaper_test $out/
          '';
        };
      };

      packages = rec {
        cargo-reaper = cargo-reaper-drv;
        default = cargo-reaper;
      };

      apps = rec {
        cargo-reaper = flake-utils.lib.mkApp
          {
            drv = cargo-reaper-drv;
          } // {
          meta = {
            homepage = "https://github.com/Cloud-Scythe-Labs/cargo-reaper/";
            description = "A Cargo plugin for developing REAPER extension plugins with Rust.";
            license = lib.licenses.mit;
            maintainers = with lib.maintainers; [ eureka-cpu ];
          };
        };
        default = cargo-reaper;
      };

      devShells.default = craneLib.devShell {
        checks = self.checks.${system};
        packages = with pkgs; [
          nil
          nixpkgs-fmt
          mdbook
          self.packages.${system}.default
        ];
      };

      formatter = pkgs.nixpkgs-fmt;
    });
}
