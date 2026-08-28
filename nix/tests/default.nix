let
  release = (import ../../release.nix { });
in
release.checks // release.crossChecks
