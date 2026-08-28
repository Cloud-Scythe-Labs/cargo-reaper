{ lib, stdenv, cargo-reaper }:
let
  src = lib.cleanSourceWith {
    src = lib.cleanSource ../plugin_manifests/workspace_manifest;
    filter = path: type: baseNameOf path != "target" && !lib.hasSuffix ".nix" (baseNameOf path);
  };
in
stdenv.mkDerivation {
  name = "test-cargo-reaper-list-workspace-manifest";
  inherit src;
  buildInputs = [
    cargo-reaper
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
}
