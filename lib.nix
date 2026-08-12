{ lib
, cargo-reaper
}:
let
  fileset.cargoReaperConfigFilter = from: lib.fileset.fileFilter (file: (builtins.match "\.?reaper\.toml" file.name) != null) from;

  # Build the extension plugin using `buildPackage`, a REAPER-plugin-aware wrapper
  # for either crane's `craneLib.buildPackage` or nixpkgs' `rustPlatform.buildRustPackage`.
  # The two builders configure the cargo build/install phases under different attribute
  # names, so `isCrane` picks the right ones without changing the caller-facing usage.
  mkBuildReaperExtension = buildPackage: isCrane:
    { package, plugin ? package, target ? null, ... }@crateArgs:
    let
      cargoReaperBuildCommand = ''
        cargo reaper build --no-symlink \
          -p ${package} --lib \
          --release ${lib.optionalString (target != null) ''\
          --target ${target}
        ''}
      '';
      installPluginCommand = ''
        mkdir -p $out/lib
        mv target${lib.optionalString (target != null) "/${target}"}/release/${plugin}.* $out/lib
      '';
      # Bypass crane's check for target install paths / rustPlatform's default install phase,
      # since the plugin is installed manually above.
      phaseArgs =
        if isCrane then {
          buildPhaseCargoCommand = cargoReaperBuildCommand;
          installPhaseCommand = installPluginCommand;
          doNotPostBuildInstallCargoBinaries = true;
        } else {
          buildPhase = ''
            runHook preBuild
            ${cargoReaperBuildCommand}
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            ${installPluginCommand}
            runHook postInstall
          '';
        };
    in
    buildPackage (crateArgs // {
      pname = package;
      # Add `cargo-reaper` as a build time dependency of this derivation.
      nativeBuildInputs = (crateArgs.nativeBuildInputs or [ ]) ++ [
        cargo-reaper
      ];
    } // phaseArgs);
in
{
  inherit fileset;

  # Extend the functionality of nixpkgs' `rustPlatform` for building REAPER extension plugins.
  rustPlatform =
    { rustPlatform ? /* nix */ ''
        # Requires `rustPlatform` from nixpkgs.

        # Example:
        # ```
        {
          inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
          outputs = { self, nixpkgs, ... }@inputs:
            let
              system = "aarch64-linux";
              pkgs = nixpkgs.legacyPackages.''${system};
              cargoReaper = inputs.cargo-reaper.mkLib {
                inherit (pkgs) lib;
                inherit (inputs.cargo-reaper.packages.''${system}) cargo-reaper;
              };
              rustPlatform = pkgs.rustPlatform // (cargoReaper.rustPlatform {
                inherit (pkgs) rustPlatform;
              });
              crateArgs = { /* Args for your crate */ };
            in
            {
              reaper_extension_plugin = rustPlatform.buildReaperExtension (crateArgs // {
                package = "my-crate";
                plugin = "reaper-extension-plugin";
              });
            };
        }
        # ```
      ''
    }: {
      buildReaperExtension = mkBuildReaperExtension rustPlatform.buildRustPackage false;
    };

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

      buildReaperExtension = mkBuildReaperExtension craneLib.buildPackage true;
    };
}
