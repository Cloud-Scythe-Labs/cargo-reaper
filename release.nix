{ ifd ? false }:
let
  inputs = import ./nix/tamal { };
  inherit (import inputs.nixpkgs { }) lib;

  pkgs = import inputs.nixpkgs {
    system = builtins.currentSystem;
    overlays = (lib.optionals ifd [
      (import "${inputs.fenix}/overlay.nix")
      (import ./nix/overlays/fenix-toolchain)
      (import ./nix/overlays/crane)
    ]) ++ [
      (import ./nix/overlays/cargo-reaper)
    ];
    config = {
      allowUnfreePredicate = pkg: builtins.elem (lib.getName pkg) [
        "reaper"
        "win-sdk"
        "xwin-fetch-msvc"
      ];
      microsoftVisualStudioLicenseAccepted = true;
    };
  };

  # The fileset for each test derivation.
  testFileset = root: pkgs.lib.fileset.toSource {
    inherit root;
    fileset = pkgs.lib.fileset.unions [
      (root + "/Cargo.toml")
      (root + "/Cargo.lock")
      (root + "/src")
      (pkgs.lib.fileset.cargoReaperConfigFilter (root + "/reaper.toml"))
    ];
  };

  checks =
    let
      inherit (pkgs) lib stdenv rustPlatform;

      commonTestArgs = src: {
        inherit src;
        strictDeps = true;
      } // lib.optionalAttrs stdenv.isLinux {
        # Rust 1.96+ uses lld with -nodefaultlibs, which means libstdc++ is no
        # longer implicitly findable at runtime in the Nix sandbox for test binaries
        # compiled by cargo during the check phase.
        LD_LIBRARY_PATH = lib.makeLibraryPath [ stdenv.cc.cc.lib ];
      };

      packageManifestTestArgs =
        let
          root = ./tests/plugin_manifests/package_manifest;
          src = testFileset root;
          individualCrateArgs = commonTestArgs src;
          cargoArtifacts = rustPlatform.buildDepsOnly individualCrateArgs;
        in
        individualCrateArgs // {
          inherit cargoArtifacts;
        };
      test-cargo-reaper-build-package-manifest = rustPlatform.buildReaperExtension (packageManifestTestArgs // {
        package = "package_manifest";
        plugin = "reaper_package_ext";
      });

      workspaceManifestTestArgs =
        let
          root = ./tests/plugin_manifests/workspace_manifest;
          src = testFileset root;
          individualCrateArgs = commonTestArgs src;
          cargoArtifacts = rustPlatform.buildDepsOnly individualCrateArgs;
        in
        individualCrateArgs // {
          inherit cargoArtifacts;
        };
      test-cargo-reaper-build-workspace-manifest = rustPlatform.buildReaperExtension (workspaceManifestTestArgs // {
        package = "extension_0";
        plugin = "reaper_ext_0";
      });

      workspacePackageManifestTestArgs =
        let
          root = ./tests/plugin_manifests/workspace_package_manifest;
          src = testFileset root;
          individualCrateArgs = commonTestArgs src;
          cargoArtifacts = rustPlatform.buildDepsOnly individualCrateArgs;
        in
        individualCrateArgs // {
          inherit cargoArtifacts;
        };
      test-cargo-reaper-build-workspace-package-manifest = rustPlatform.buildReaperExtension (workspacePackageManifestTestArgs // {
        package = "workspace_package_manifest";
        plugin = "reaper_workspace_package_ext";
      });
    in
    {
      # Build the crate as part of `checks` for convenience
      inherit (pkgs) cargo-reaper;

      inherit
        test-cargo-reaper-build-package-manifest
        test-cargo-reaper-build-workspace-manifest
        test-cargo-reaper-build-workspace-package-manifest
        ;

      inherit (pkgs.cargo-reaper.passthru.tests)
        cargo-clippy
        cargo-doc
        cargo-fmt
        taplo-fmt
        cargo-deny
        cargo-nextest
        ;

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
    };
in
{
  inherit checks;

  crossChecks =
    let
      inherit (pkgs) lib stdenv rustPlatform;
    in
    lib.optionalAttrs stdenv.isLinux {
      test-cargo-reaper-build-cross-windows =
        let
          rustcTarget = "x86_64-pc-windows-msvc";
          rustPlatformCross = rustPlatform.overrideScope (rfinal: rprev:
            let
              craneLibCross = (import (import ./nix/tamal { }).crane { inherit pkgs; }).overrideToolchain (pkgs: pkgs.fenix.combine [
                pkgs.rustPlatform.rustToolchain
                pkgs.fenix.targets.${rustcTarget}.stable.rust-std
              ]);
            in
            {
              inherit (craneLibCross) buildPackage buildDepsOnly;
            });
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
          cargoArtifactsCross = rustPlatformCross.buildDepsOnly crossArgs;
        in
        rustPlatformCross.buildReaperExtension (crossArgs // {
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

  formatter = pkgs.nixpkgs-fmt;

  devShells =
    let
      inherit (pkgs) lib stdenv;
    in
    {
      default = pkgs.rustPlatform.devShell {
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
}

