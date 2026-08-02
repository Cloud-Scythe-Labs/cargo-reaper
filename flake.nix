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
            cargoReaper = self.mkLib {
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
              craneLib // (cargoReaper.crane { inherit craneLib; });
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
              # Building the crate and the tests reuses these, so expose them on
              # the package rather than polluting the top-level package set.
              passthru = {
                inherit cargoReaper craneLib commonArgs cargoArtifacts src rustToolchain;
              };
            });
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

      testProfile = { pkgs, name, ... }:
        {
          users.users = {
            "${name}" = {
              isNormalUser = true;
              description = name;
              home = "/home/${name}";
              createHome = true;
            };
            root = {
              hashedPassword = "";
              hashedPasswordFile = null;
            };
          };

          # Enable audio via pipewire.
          services.pulseaudio.enable = false;
          security.rtkit.enable = true;
          services.pipewire = {
            enable = true;
            alsa.enable = true;
            alsa.support32Bit = true;
            pulse.enable = true;
            jack.enable = true;
          };

          # Necessary for Xvfb
          services.xserver.enable = true;
          # This can be changed to another DM like xfce if a GUI is needed for debugging
          services.xserver.displayManager.startx.enable = true;

          environment.systemPackages = with pkgs; [
            reaper
            xdotool
            xvfb-run
            cargo-reaper
          ];
        };
    in
    {
      mkLib = import ./lib;

      checks = eachSystem (pkgs:
        let
          inherit (pkgs) lib stdenv;
          inherit (stdenv.hostPlatform) system;
          inherit (pkgs.cargo-reaper) cargoReaper craneLib commonArgs cargoArtifacts src rustToolchain;

          # `nixosTest`s always run their nodes on Linux. When these checks are
          # evaluated on Darwin the test *driver* runs locally (Darwin advertises
          # the `nixos-test` feature) while the guest VMs build on a linux-builder.
          # Any store path we interpolate into a guest — the pre-built plugin, the
          # in-VM Rust toolchain — must therefore be built for the guest's Linux
          # system, not the Darwin host. (Packages pulled in via a node module's
          # `pkgs` are already the guest's, so `systemPackages` need no adjustment.)
          guestSystem = builtins.replaceStrings [ "darwin" ] [ "linux" ] system;
          guestPkgs = pkgsFor guestSystem;

          commonTestArgs = src: {
            inherit src;
            strictDeps = true;
          } // lib.optionalAttrs stdenv.isLinux {
            # Rust 1.96+ uses lld with -nodefaultlibs, which means libstdc++ is no
            # longer implicitly findable at runtime in the Nix sandbox for test binaries
            # compiled by cargo during the check phase.
            LD_LIBRARY_PATH = lib.makeLibraryPath [ stdenv.cc.cc.lib ];
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
          cargo-reaper = pkgs.cargo-reaper;

          inherit
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

          test-cargo-reaper-new-ext = stdenv.mkDerivation {
            name = "test-cargo-reaper-new-ext";
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
              cargo-reaper new --template ext reaper_test
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
          test-cargo-reaper-new-vst = stdenv.mkDerivation {
            name = "test-cargo-reaper-new-vst";
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
              cargo-reaper new --template vst reaper_test
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
          test-cargo-reaper-list-package-manifest = stdenv.mkDerivation {
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
          test-cargo-reaper-list-workspace-manifest = stdenv.mkDerivation {
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
          test-cargo-reaper-list-workspace-package-manifest = stdenv.mkDerivation {
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

          # Link the pre-built plugin using `cargo-reaper link` and
          # assert the symbolic link exists in the `UserPlugins` directory.
          test-cargo-reaper-link =
            let
              plugin = self.checks.${guestSystem}.test-cargo-reaper-build-package-manifest;
              plugin_name = "reaper_package_ext";
            in
            pkgs.testers.nixosTest {
              name = "test-cargo-reaper-link";
              nodes.corro = testProfile;
              testScript = ''
                corro.start()
                corro.wait_for_unit("multi-user.target")

                # Launch REAPER once to initialize `~/.config/REAPER/UserPlugins`
                corro.succeed("su - corro -c 'cargo-reaper run --no-build --headless --timeout 5s --stdout null --stderr null'", timeout=20)

                corro.succeed("su - corro -c 'cargo-reaper link ${plugin}/lib/${plugin_name}.*'", timeout=15)
                corro.succeed("su - corro -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'", timeout=15)
              '';
            };

          # Copy plugin source code and its pre-vendored dependencies onto the
          # machine, build the plugin from source and open REAPER, asserting that
          # no error window (titled after the plugin) ever appears. `--keep-going`
          # keeps watching until the timeout instead of exiting on the first
          # match, so the run exits non-zero only if the window is never seen.
          test-cargo-reaper-run =
            let
              plugin_source = testFileset ./tests/plugin_manifests/package_manifest;
              plugin_vendor = craneLib.vendorCargoDeps { src = plugin_source; };
              plugin_name = "reaper_package_ext";
            in
            pkgs.testers.nixosTest {
              name = "test-cargo-reaper-run";
              nodes.corro = {
                imports = [ testProfile ];
                # A Rust toolchain is required to build the plugin from source in
                # the VM; it must target the guest's Linux system.
                environment.systemPackages = [
                  guestPkgs.cargo-reaper.rustToolchain
                  guestPkgs.gcc
                ];
              };
              testScript = ''
                corro.start()
                corro.wait_for_unit("multi-user.target")

                # Launch REAPER once to initialize `~/.config/REAPER/UserPlugins`
                corro.succeed("su - corro -c 'cargo-reaper run --no-build --headless --timeout 5s --stdout null --stderr null'", timeout=20)

                corro.succeed("su - root -c 'cp -r ${plugin_source}/* /home/corro/'", timeout=15)
                corro.succeed("su - root -c 'mkdir -p /home/corro/.cargo && cp -r ${plugin_vendor}/config.toml /home/corro/.cargo/'", timeout=15)

                status, output = corro.execute(
                    "su - corro -c 'cargo-reaper run --headless --keep-going "
                    "--locate-window \"${plugin_name} error\" "
                    "--timeout 5s --stdout null --stderr null -- --release --offline'",
                    timeout=180,
                )
                assert status != 0, f"'${plugin_name} error' window detected, REAPER output:\n{output}"
              '';
            };

          # Link the pre-built plugin using `cargo-reaper link`, then run
          # `cargo-reaper clean` and assert the plugin link no longer exists in
          # the `UserPlugins` directory.
          test-cargo-reaper-clean =
            let
              plugin = self.checks.${guestSystem}.test-cargo-reaper-build-package-manifest;
              plugin_source = testFileset ./tests/plugin_manifests/package_manifest;
              plugin_name = "reaper_package_ext";
            in
            pkgs.testers.nixosTest {
              name = "test-cargo-reaper-clean";
              nodes.corro = testProfile;
              testScript = ''
                corro.start()
                corro.wait_for_unit("multi-user.target")

                # Launch REAPER once to initialize `~/.config/REAPER/UserPlugins`
                corro.succeed("su - corro -c 'cargo-reaper run --no-build --headless --timeout 5s --stdout null --stderr null'", timeout=20)

                corro.succeed("su - corro -c 'cargo-reaper link ${plugin}/lib/${plugin_name}.*'", timeout=15)
                corro.succeed("su - corro -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'", timeout=15)

                corro.succeed("su - root -c 'cp -r ${plugin_source}/* /home/corro/'", timeout=15)
                corro.succeed("su - corro -c 'cargo-reaper clean -p ${plugin_name}'", timeout=15)
                corro.fail("su - corro -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'", timeout=15)
              '';
            };
        } // lib.optionalAttrs stdenv.isLinux {
          test-cargo-reaper-build-cross-windows =
            let
              rustcTarget = "x86_64-pc-windows-msvc";
              craneLibCross =
                let
                  rustWithWindowsTarget = fenix.packages.${system}.combine [
                    rustToolchain
                    fenix.packages.${system}.targets.${rustcTarget}.stable.rust-std
                  ];
                  craneLib = (crane.mkLib pkgs).overrideToolchain rustWithWindowsTarget;
                in
                craneLib // (cargoReaper.crane { inherit craneLib; });
              crossArgs =
                let
                  inherit (pkgs) llvmPackages windows;
                  envTarget = builtins.replaceStrings [ "-" ] [ "_" ] rustcTarget;
                  envTargetUpper = lib.toUpper envTarget;
                  CC = "${llvmPackages.clang-unwrapped}/bin/clang-cl";
                  # Flags forwarded to clang-cl by cc-rs so it can locate MSVC headers and libs.
                  CFLAGS = lib.concatStringsSep " " [
                    "/vctoolsdir ${windows.sdk}/crt"
                    "/winsdkdir ${windows.sdk}/sdk"
                  ];
                in
                {
                  src = testFileset ./tests/plugin_manifests/package_manifest;
                  strictDeps = true;
                  nativeBuildInputs = with llvmPackages; [
                    clang-unwrapped # clang-cl (C/C++ compiler)
                    bintools-unwrapped # lld-link (linker)
                    llvm # llvm-lib (MSVC lib.exe equivalent, used by cc-rs)
                  ];
                  "CC_${envTarget}" = CC;
                  "CXX_${envTarget}" = CC;
                  "CFLAGS_${envTarget}" = CFLAGS;
                  "CXXFLAGS_${envTarget}" = CFLAGS;
                  "AR_${envTarget}" = "${llvmPackages.llvm}/bin/llvm-lib";
                  "CARGO_TARGET_${envTargetUpper}_LINKER" = "${llvmPackages.bintools-unwrapped}/bin/lld-link";
                  "CARGO_TARGET_${envTargetUpper}_RUSTFLAGS" = lib.concatStringsSep " " [
                    "-C link-arg=/LIBPATH:${windows.sdk}/crt/lib/x64"
                    "-C link-arg=/LIBPATH:${windows.sdk}/sdk/lib/ucrt/x64"
                    "-C link-arg=/LIBPATH:${windows.sdk}/sdk/lib/um/x64"
                  ];
                };
              cargoArtifactsCross = craneLibCross.buildDepsOnly crossArgs;
            in
            craneLibCross.buildReaperExtension (crossArgs // {
              cargoArtifacts = cargoArtifactsCross;
              package = "package_manifest";
              plugin = "reaper_package_ext";
              target = rustcTarget;
              /* Checks could be ran using wine64, but in this case we only care
              that the package was built and the output is the expected format */
              doCheck = false;
              doInstallCheck = true;
              installCheckPhase = ''
                test -f $out/lib/reaper_package_ext.dll

                file_output=$(file $out/lib/reaper_package_ext.dll)
                echo "$file_output"
                echo "$file_output" |
                  grep -q "PE32+ executable for MS Windows.*(DLL), x86-64" || {
                    echo "ERROR: not a PE32+ DLL";
                    exit 1;
                  }

                imports=$(llvm-objdump -p $out/lib/reaper_package_ext.dll | grep "DLL Name")
                echo "$imports"
                echo "$imports" |
                  grep -q "VCRUNTIME140.dll" || {
                    echo "ERROR: VCRUNTIME140.dll not imported (not an MSVC ABI DLL)";
                    exit 1;
                  }
                echo "$imports" |
                  grep -qiE "libgcc|libstdc\+\+|msvcrt\.dll" && {
                    echo "ERROR: MinGW runtime imported (not an MSVC ABI DLL)";
                    exit 1;
                  }
                true
              '';
            });
        });

      packages = eachSystem (pkgs: {
        inherit (pkgs) cargo-reaper;
        default = pkgs.cargo-reaper;
      });

      apps = eachSystem (pkgs:
        let
          inherit (pkgs) lib;
          cargo-reaper = {
            type = "app";
            program = "${pkgs.cargo-reaper}/bin/cargo-reaper";
            meta = {
              homepage = "https://github.com/Cloud-Scythe-Labs/cargo-reaper/";
              description = "A Cargo plugin for developing REAPER extension plugins with Rust.";
              license = lib.licenses.mit;
              maintainers = with lib.maintainers; [ eureka-cpu ];
            };
          };
        in
        {
          inherit cargo-reaper;
          default = cargo-reaper;
        });

      devShells = eachSystem (pkgs:
        let
          inherit (pkgs.stdenv.hostPlatform) system;
        in
        {
          default = pkgs.cargo-reaper.craneLib.devShell {
            checks = self.checks.${system};
            packages = with pkgs; [
              nil
              nixpkgs-fmt
              mdbook
              self.packages.${system}.default
              reaper
            ] ++ lib.optionals stdenv.isLinux [
              xdotool
            ];
          };
        });

      formatter = eachSystem (pkgs: pkgs.nixpkgs-fmt);
    };
}
