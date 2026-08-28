{ system ? builtins.currentSystem }:

let
  inputs = import ./nix/tamal { inherit system; };

  inherit (pkgs) lib stdenv;
  pkgs = import inputs.nixpkgs {
    inherit system;
    overlays = [ (import ./overlay.nix) ];
    config = {
      allowUnfreePredicate = pkg: builtins.elem (lib.getName pkg) [
        "reaper"
        "win-sdk"
        "xwin-fetch-msvc"
      ];
      microsoftVisualStudioLicenseAccepted = true;
    };
  };

  treefmtEval = (import inputs.treefmt).evalModule pkgs {
    projectRootFile = "release.nix";
    programs = {
      nixpkgs-fmt.enable = true;
      rustfmt.enable = true;
      taplo.enable = true;
      mdformat.enable = true;
      yamlfmt.enable = true;
    };
  };
in
{
  inherit pkgs;

  default = pkgs.cargo-reaper;

  fmt = treefmtEval.config.build.wrapper;

  checks = {
    inherit (pkgs) cargo-reaper;
    inherit (pkgs.cargo-reaper.passthru.tests)
      cargo-clippy
      cargo-doc
      cargo-deny
      ;

    fmt = treefmtEval.config.build.check (lib.cleanSourceWith {
      src = lib.cleanSource ./.;
      filter = path: type: baseNameOf path != "target";
    });

    test-cargo-reaper-build-package-manifest = pkgs.callPackage ./nix/tests/plugin_manifests/package_manifest { };
    test-cargo-reaper-build-workspace-manifest = pkgs.callPackage ./nix/tests/plugin_manifests/workspace_manifest { };
    test-cargo-reaper-build-workspace-package-manifest = pkgs.callPackage ./nix/tests/plugin_manifests/workspace_package_manifest { };

    test-cargo-reaper-new-ext = pkgs.callPackage ./nix/tests/new-ext { };
    test-cargo-reaper-new-vst = pkgs.callPackage ./nix/tests/new-vst { };
    test-cargo-reaper-list-package-manifest = pkgs.callPackage ./nix/tests/list-package-manifest { };
    test-cargo-reaper-list-workspace-manifest = pkgs.callPackage ./nix/tests/list-workspace-manifest { };
    test-cargo-reaper-list-workspace-package-manifest = pkgs.callPackage ./nix/tests/list-workspace-package-manifest { };
  };

  crossChecks = lib.optionalAttrs stdenv.isLinux {
    test-cargo-reaper-build-cross-windows =
      let
        winPkgs = pkgs.pkgsCross.x86_64-windows;
        inherit (winPkgs.stdenv.hostPlatform.rust) rustcTarget;

        fenix = pkgs.callPackage inputs.fenix { };
        rustToolchain = fenix.combine [
          (fenix.stable.withComponents [ "cargo" "rustc" ])
          fenix.targets.${rustcTarget}.stable.rust-std
        ];
        winRustPlatform = pkgs.rustPlatform.overrideScope (rfinal: rprev: {
          # Override `buildRustPackage` with our windows toolchain
          inherit (pkgs.makeRustPlatform {
            rustc = rustToolchain;
            cargo = rustToolchain;
          })
            buildRustPackage;
        });
      in
      winRustPlatform.buildReaperExtension (
        let
          inherit (pkgs) llvmPackages;
          inherit (winPkgs) windows;
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
          package = "package_manifest";
          plugin = "reaper_package_ext";
          target = rustcTarget;
          src = lib.cleanSourceWith {
            src = lib.cleanSource ./nix/tests/plugin_manifests/package_manifest;
            filter = path: type: baseNameOf path != "target" && !lib.hasSuffix ".nix" (baseNameOf path);
          };

          strictDeps = true; # Enforce only buildPlatform deps in sandbox path
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
        }
      );
  };

  nixosTests = {
    test-cargo-reaper-link = pkgs.callPackage ./nixos/tests/link { };
    test-cargo-reaper-run = pkgs.callPackage ./nixos/tests/run { };
    test-cargo-reaper-clean = pkgs.callPackage ./nixos/tests/clean { };
  };

  devShell = pkgs.mkShell {
    inputsFrom = builtins.attrValues pkgs.cargo-reaper.passthru.tests;
    packages = with pkgs; [
      nil
      nixtamal
      mdbook
      cargo-reaper
      rust-analyzer
      reaper
    ] ++ lib.optionals stdenv.isLinux [
      xdotool
    ];
  };
}
