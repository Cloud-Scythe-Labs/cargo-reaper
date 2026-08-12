# TODO: Fix this so that it is applied to rustPlatform instead of crane specifically
{ lib
, cargo-reaper
}:
let
  fileset.cargoReaperConfigFilter = from: lib.fileset.fileFilter (file: (builtins.match "\.?reaper\.toml" file.name) != null) from;
in
{
  inherit fileset;

  # Extend the functionality of the crane library for building REAPER extension plugins.
  crane =
    { craneLib ? /* nix */ ''
        # Requires `craneLib` from the `crane` library: https://crane.dev/API.html#cranelib

        # Example:
        # ```
        {
          inputs = {
            nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
            crane.url = "github:ipetkov/crane";
            cargo-reaper.url = "github:Cloud-Scythe-Labs/cargo-reaper";
          };
          outputs = { self, ... }@inputs:
            let
              system = "aarch64-linux";
              pkgs = inputs.nixpkgs.legacyPackages.''${system};
              craneLib =
                let
                  craneLib = inputs.crane.mkLib pkgs;
                  cargoReaper = inputs.cargo-reaper.mkLib {
                    inherit (pkgs) lib;
                    inherit (inputs.cargo-reaper.packages.''${system}) cargo-reaper;
                  };
                in
                craneLib // (cargoReaper.crane {
                  inherit craneLib;
                });
              crateArgs = { /* Args for your crate */ };
            in
            {
              reaper_extension_plugin = craneLib.buildReaperExtension (crateArgs // {
                package = "my-crate";
                plugin = "reaper-extension-plugin";
              });
            };
        }
        # ```
      ''
    }: {
      fileset = (craneLib.fileset or { }) // fileset;

      # TODO: The usage shouldn't really change, but under the hood we need a way to
      # determine which args to set. If we are using crane, the arg names are slightly
      # different than rustPlatform.buildRustPackage.
      buildReaperExtension = { package, plugin ? package, target ? null, ... }@crateArgs:
        craneLib.buildPackage (crateArgs // {
          pname = package;
          nativeBuildInputs = (crateArgs.nativeBuildInputs or [ ]) ++ [
            # Add `cargo-reaper` as a build time dependency of this derivation.
            cargo-reaper
          ];
          # Run `cargo-reaper`, passing trailing args to the cargo invocation.
          # We do not symlink the plugin since the `UserPlugins` directory is in
          # the `$HOME` directory which is inaccessible to the sandbox.
          buildPhaseCargoCommand = ''
            cargo reaper build --no-symlink \
              -p ${package} --lib \
              --release ${lib.optionalString (target != null) ''\
              --target ${target}
            ''}
          '';
          # Include extension plugin in the build result.
          installPhaseCommand = ''
            mkdir -p $out/lib
            mv target${lib.optionalString (target != null) "/${target}"}/release/${plugin}.* $out/lib
          '';
          # Bypass crane checks for target install paths.
          doNotPostBuildInstallCargoBinaries = true;
        });
    };
}
