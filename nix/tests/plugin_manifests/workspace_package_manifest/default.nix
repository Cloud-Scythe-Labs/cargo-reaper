{ lib, stdenv, rustPlatform }:
let
  src = lib.cleanSourceWith {
    src = lib.cleanSource ./.;
    filter = path: type: baseNameOf path != "target" && !lib.hasSuffix ".nix" (baseNameOf path);
  };
in
rustPlatform.buildReaperExtension ({
  inherit src;
  strictDeps = true;
  cargoLock = {
    lockFile = src + "/Cargo.lock";
    allowBuiltinFetchGit = true;
  };
  package = "workspace_package_manifest";
  plugin = "reaper_workspace_package_ext";
} // lib.optionalAttrs stdenv.isLinux {
  # Rust 1.96+ uses lld with -nodefaultlibs, which means libstdc++ is no
  # longer implicitly findable at runtime in the Nix sandbox for test binaries
  # compiled by cargo during the check phase.
  LD_LIBRARY_PATH = lib.makeLibraryPath [ stdenv.cc.cc.lib ];
})
