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
    , flake-utils
    , advisory-db
    , ...
    }:

    { mkLib = import ./lib; } //

    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        config.allowUnfree = true;
      };

      inherit (pkgs) lib;
      cargoReaper = self.mkLib {
        inherit lib;
        inherit (self.packages.${system}) cargo-reaper;
      };

      rustToolchain = fenix.packages.${system}.fromToolchainFile {
        file = ./.rust-toolchain.toml;
        sha256 = "sha256-KUm16pHj+cRedf8vxs/Hd2YWxpOrWZ7UOrwhILdSJBU=";
      };
      craneLib =
        let
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        in
        craneLib // (cargoReaper.crane {
          inherit craneLib;
        });
      src = craneLib.cleanCargoSource ./.;

      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit src;
        strictDeps = true;

        buildInputs = lib.optionals pkgs.stdenv.isDarwin [
          pkgs.libiconv
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
          mkTestScripts = cargoReaper.scripts { inherit (pkgs) writeShellScriptBin; };
          commonTestArgs = src: {
            inherit src;
            strictDeps = true;
          };

          testFileset = root: lib.fileset.toSource {
            inherit root;
            fileset = lib.fileset.unions [
              (root + "/Cargo.toml")
              (root + "/Cargo.lock")
              (root + "/src")
              (craneLib.fileset.cargoReaperConfigFilter (root + "/reaper.toml"))
            ];
          };

          packageManifestTestArgs =
            let
              root = ./tests/plugin_manifests/package_manifest;
              src = testFileset root;
              individualCrateArgs = commonTestArgs src;
              cargoArtifacts = craneLib.buildDepsOnly individualCrateArgs;
            in
            (individualCrateArgs // {
              inherit cargoArtifacts;
            });
          test-cargo-reaper-build-package-manifest = craneLib.buildReaperExtension (packageManifestTestArgs // {
            package = "package_manifest";
            plugin = "reaper_package_ext";
          });

          workspaceManifestTestArgs =
            let
              root = ./tests/plugin_manifests/workspace_manifest;
              src = testFileset root;
              individualCrateArgs = commonTestArgs src;
              cargoArtifacts = craneLib.buildDepsOnly individualCrateArgs;
            in
            (individualCrateArgs // {
              inherit cargoArtifacts;
            });
          test-cargo-reaper-build-workspace-manifest = craneLib.buildReaperExtension (workspaceManifestTestArgs // {
            package = "extension_0";
            plugin = "reaper_ext_0";
          });

          workspacePackageManifestTestArgs =
            let
              root = ./tests/plugin_manifests/workspace_package_manifest;
              src = testFileset root;
              individualCrateArgs = commonTestArgs src;
              cargoArtifacts = craneLib.buildDepsOnly individualCrateArgs;
            in
            (individualCrateArgs // {
              inherit cargoArtifacts;
            });
          test-cargo-reaper-build-workspace-package-manifest = craneLib.buildReaperExtension (workspacePackageManifestTestArgs // {
            package = "workspace_package_manifest";
            plugin = "reaper_workspace_package_ext";
          });
        in
        {
          # Build the crate as part of `nix flake check` for convenience
          inherit
            cargo-reaper-drv
            test-cargo-reaper-build-package-manifest
            test-cargo-reaper-build-workspace-manifest
            test-cargo-reaper-build-workspace-package-manifest
            ;

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

          test-cargo-reaper-list-package-manifest = pkgs.stdenv.mkDerivation {
            name = "test-cargo-reaper-list-package-manifest";
            src = testFileset ./tests/plugin_manifests/package_manifest;
            buildInputs = [
              self.packages.${system}.default
            ];
            phases = [
              "unpackPhase"
              "buildPhase"
              "installPhase"
            ];
            buildPhase = ''
              cargo-reaper list
            '';
            installPhase = ''
              mkdir -p $out
            '';
          };
          test-cargo-reaper-list-workspace-manifest = pkgs.stdenv.mkDerivation {
            name = "test-cargo-reaper-list-workspace-manifest";
            src = testFileset ./tests/plugin_manifests/workspace_manifest;
            buildInputs = [
              self.packages.${system}.default
            ];
            phases = [
              "unpackPhase"
              "buildPhase"
              "installPhase"
            ];
            buildPhase = ''
              cargo-reaper list
            '';
            installPhase = ''
              mkdir -p $out
            '';
          };
          test-cargo-reaper-list-workspace-package-manifest = pkgs.stdenv.mkDerivation {
            name = "test-cargo-reaper-list-workspace-package-manifest";
            src = testFileset ./tests/plugin_manifests/workspace_package_manifest;
            buildInputs = [
              self.packages.${system}.default
            ];
            phases = [
              "unpackPhase"
              "buildPhase"
              "installPhase"
            ];
            buildPhase = ''
              cargo-reaper list
            '';
            installPhase = ''
              mkdir -p $out
            '';
          };
        } // lib.optionalAttrs pkgs.stdenv.isLinux {
          test-cargo-reaper-link =
            let
              tests = import ./tests {
                inherit pkgs;
                inherit (self.packages.${system}) cargo-reaper;
                inherit (mkTestScripts) mkReaperDryRun mkCargoReaperDryRun;
              };
            in
            pkgs.nixosTest {
              name = "test-cargo-reaper-link";
              inherit (tests) nodes;
              testScript = tests.test-cargo-reaper-link {
                plugin = test-cargo-reaper-build-package-manifest;
                plugin_name = "reaper_package_ext";
              };
            };
          test-cargo-reaper-run =
            let
              tests = import ./tests {
                inherit pkgs;
                inherit (self.packages.${system}) cargo-reaper;
                inherit (mkTestScripts) mkReaperDryRun mkCargoReaperDryRun;
                imports = [
                  {
                    environment.systemPackages = [
                      rustToolchain
                      pkgs.gcc
                    ];
                  }
                ];
              };
            in
            pkgs.nixosTest {
              name = "test-cargo-reaper-run";
              inherit (tests) nodes;
              testScript = tests.test-cargo-reaper-run rec {
                plugin_source = testFileset ./tests/plugin_manifests/package_manifest;
                plugin_vendor = craneLib.vendorCargoDeps { src = plugin_source; };
                plugin_name = "reaper_package_ext";
              };
            };
          test-cargo-reaper-clean =
            let
              tests = import ./tests {
                inherit pkgs;
                inherit (self.packages.${system}) cargo-reaper;
                inherit (mkTestScripts) mkReaperDryRun mkCargoReaperDryRun;
              };
            in
            pkgs.nixosTest {
              name = "test-cargo-reaper-clean";
              inherit (tests) nodes;
              testScript = tests.test-cargo-reaper-clean {
                plugin = test-cargo-reaper-build-package-manifest;
                plugin_source = testFileset ./tests/plugin_manifests/package_manifest;
                plugin_name = "reaper_package_ext";
              };
            };
        };

      # These checks require `--option sandbox false`.
      checks-no-sandbox = {
        # TODO: add a `--offline` feature to `cargo reaper new`, then
        # pre-populate the cargo temp directory using `fetchFromGithub`.
        # Once we can invoke it in offline mode, we can move this back to checks.
        test-cargo-reaper-new = pkgs.stdenv.mkDerivation {
          name = "test-cargo-reaper-new";
          buildInputs = [
            self.packages.${system}.default
          ];
          doCheck = true;
          phases = [
            "buildPhase"
            "checkPhase"
            "installPhase"
          ];
          buildPhase = ''
            cargo-reaper new reaper_test
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
          reaper
        ];
      };

      formatter = pkgs.nixpkgs-fmt;
    });
}
