final: prev:
let
  inherit (import ../../../nix/tamal { }) crane;
in
{
  rustPlatform = prev.rustPlatform.overrideScope (rfinal: rprev: {
    inherit ((import crane { pkgs = prev; }).overrideToolchain (pkgs: pkgs.rustPlatform.rustToolchain))
      buildDepsOnly
      vendorCargoDeps
      buildPackage
      cargoClippy
      cargoDoc
      cargoFmt
      taploFmt
      cargoDeny
      cargoNextest
      devShell
      ;
  });
}
