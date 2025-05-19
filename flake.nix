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
    {
      self,
      nixpkgs,
      crane,
      fenix,
      nix-core,
      flake-utils,
      advisory-db,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

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
        checks = {
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
            src = pkgs.lib.sources.sourceFilesBySuffices src [ ".toml" ];
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
          # Consider setting `doCheck = false` on `my-crate` if you do not want
          # the tests to run twice
          cargo-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
            cargoNextestPartitionsExtraArgs = "--no-tests=warn";
          });
        };

        packages = rec {
          cargo-reaper = cargo-reaper-drv;
          default = cargo-reaper;
        };

        apps = rec {
          cargo-reaper = flake-utils.lib.mkApp {
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
          packages = [
            self.packages.${system}.default
          ];
        };
      }
    );
}
