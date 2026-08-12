# TODO: Add a way to easily override the overlays so we don't have to commit to crane
# It would be great if we could have a way to opt out of crane for testing. @claude,
# don't work on this todo, just leave this comment as is.
let
  inputs = import ./nix/tamal { };
  lib = (import inputs.nixpkgs { }).lib;
  system = builtins.currentSystem;
  overlays = [
    (import "${inputs.fenix}/overlay.nix")
    (final: prev:
      let
        cargoReaperLib' = import ./lib.nix {
          inherit (prev) lib;
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
            craneLib = (import inputs.crane { pkgs = prev; }).overrideToolchain rustToolchain;
          in
          # TODO: Since the lib.nix function is no longer dependent on crane specifically we
          # should instead treat it as an extension of rustPlatform, just like crane. This
          # means we no longer need crane specifically as an input in order to use it. It is
          # effectively another overlay which calls overrideScope on rustPlatform, and lib.fileset
          # in order to extend their capabilities.
          craneLib // (cargoReaperLib'.crane { inherit craneLib; });
        src = craneLib.cleanCargoSource ./.;
        # TODO: We should be including this in passthru of the package, not in this overlay
        # just because we require it somewhere downstream. If it's in the package, we can
        # refer to it easily, as where here it is just clutter.
        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with prev; [
            installShellFiles
          ] ++ prev.lib.optionals prev.stdenv.isLinux [
            autoPatchelfHook
          ];

          buildInputs = with prev; [
            libgcc
          ] ++ prev.lib.optionals prev.stdenv.isDarwin [
            libiconv
          ];
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      in
      {
        rustPlatform = prev.rustPlatform.overrideScope (rfinal: rprev: {
          inherit (craneLib) buildDepsOnly buildReaperExtension;
          buildCranePackage = craneLib.buildPackage;
        });
        cargoReaperLib = prev.lib.makeScope prev.newScope (self: (cargoReaperLib' // {
          inherit craneLib commonArgs cargoArtifacts src rustToolchain;
        }));
      })
    (import ./nix/overlays/cargo-reaper)
  ];

  pkgs = import inputs.nixpkgs {
    inherit system overlays;
    config = {
      allowUnfreePredicate = pkg: builtins.elem (lib.getName pkg) [
        "reaper"
        "win-sdk"
        "xwin-fetch-msvc"
      ];
      microsoftVisualStudioLicenseAccepted = true;
    };
  };

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
      services.xserver = {
        enable = true;
        # This can be changed to another DM like xfce if a GUI is needed for debugging
        displayManager.startx.enable = true;
      };

      environment.systemPackages = with pkgs; [
        reaper
        xdotool
        xvfb-run
        cargo-reaper
      ];
    };
  checks =
    let
      inherit (pkgs) lib stdenv cargoReaperLib;
      inherit (cargoReaperLib) craneLib commonArgs cargoArtifacts src rustToolchain;

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
      inherit (pkgs) cargo-reaper;

      inherit
        test-cargo-reaper-build-package-manifest
        test-cargo-reaper-build-workspace-manifest
        test-cargo-reaper-build-workspace-package-manifest
        ;

      cargo-clippy = craneLib.cargoClippy (commonArgs // {
        inherit cargoArtifacts;
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      });

      cargo-doc = craneLib.cargoDoc (commonArgs // {
        inherit cargoArtifacts;
      });

      cargo-fmt = craneLib.cargoFmt {
        inherit src;
      };

      taplo-fmt = craneLib.taploFmt {
        src = lib.sources.sourceFilesBySuffices src [ ".toml" ];
      };

      # TODO: `advisory-db` (rustsec/advisory-db) isn't tracked as a nixtamal input yet,
      # so `cargo-audit` is left out of `checks` until it is.

      cargo-deny = craneLib.cargoDeny {
        inherit src;
      };

      cargo-nextest = craneLib.cargoNextest (commonArgs // {
        inherit cargoArtifacts;
        partitions = 1;
        partitionType = "count";
        cargoNextestPartitionsExtraArgs = "--no-tests=warn";
      });

      test-cargo-reaper-new-ext = stdenv.mkDerivation {
        name = "test-cargo-reaper-new-ext";
        buildInputs = [
          pkgs.cargo-reaper
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
          pkgs.cargo-reaper
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
          pkgs.cargo-reaper
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
          pkgs.cargo-reaper
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
          pkgs.cargo-reaper
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
    } // lib.optionalAttrs stdenv.isLinux {
      test-cargo-reaper-build-cross-windows =
        let
          rustcTarget = "x86_64-pc-windows-msvc";
          craneLibCross =
            let
              rustWithWindowsTarget = pkgs.fenix.combine [
                rustToolchain
                pkgs.fenix.targets.${rustcTarget}.stable.rust-std
              ];
              craneLib = (import inputs.crane { inherit pkgs; }).overrideToolchain rustWithWindowsTarget;
            in
            craneLib // (cargoReaperLib.crane { inherit craneLib; });
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
          # Checks could be ran using wine64, but in this case we only care
          # that the package was built and the output is the expected format
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
    };

  packages = {
    inherit (pkgs) cargo-reaper;
    default = pkgs.cargo-reaper;
  };

  apps =
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
    };

  devShells =
    let
      inherit (pkgs) lib stdenv;
    in
    {
      default = pkgs.cargoReaperLib.craneLib.devShell {
        inherit checks;
        packages = with pkgs; [
          nil
          nixpkgs-fmt
          mdbook
          cargo-reaper
          reaper
        ] ++ lib.optionals stdenv.isLinux [
          xdotool
        ];
      };
    };

  formatter = pkgs.nixpkgs-fmt;
in
{
  inherit checks packages apps devShells formatter;
}

