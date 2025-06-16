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
    { craneLib ? ''
        Requires `craneLib` from the `crane` library: https://crane.dev/API.html#cranelib

        Example:
        ```
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
        ```
      ''
    }: {
      fileset = (craneLib.fileset or { }) // fileset;

      buildReaperExtension = { package, plugin ? package, ... }@crateArgs:
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
            ls .
            cargo reaper build --no-symlink \
              -p ${package} --lib \
              --release
          '';
          # Include extension plugin in the build result.
          installPhaseCommand = ''
            mkdir -p $out/lib
            mv target/release/${plugin}.* $out/lib
          '';
          # Bypass crane checks for target install paths.
          doNotPostBuildInstallCargoBinaries = true;
        });
    };

  # Scripts useful for testing REAPER extension plugins with `nixosTest`.
  scripts = { writeShellScriptBin }:
    {
      # REAPER doesn't like to execute in the background, and NixOS Test invokes commands
      # as root. To combat this, we change users based on machine before launching REAPER,
      # using xvfb and xdotool to search for an error window before exiting successfully.
      mkCargoReaperDryRun =
        { user
        , cargo-reaper
        , xdotool
        , xvfb-run
        }:
        writeShellScriptBin "cargo_reaper_dry_run" ''
          function run_cargo_reaper() {
              su - ${user} -c '${cargo-reaper}/bin/cargo-reaper run --release --offline &'
              sleep 5

              # In this case reaper is running as a subprocess of `cargo-reaper run`
              # so we must find the process id manually in order to terminate it.
              reaper_pid=$(pgrep -u ${user} -f 'reaper')
              if [[ -z "$reaper_pid" ]]; then
                  echo "REAPER process not found!"
                  exit 1
              fi
              echo "REAPER is running with PID $reaper_pid"

              error_window=$(${xdotool}/bin/xdotool search --name "$1")
              if [[ -n "$error_window" ]]; then
                  echo "found error window with ID: $error_window"
                  exit 1
              fi
              kill $reaper_pid
          }

          echo "searching for error window title '$1'"
          ${xvfb-run}/bin/xvfb-run -a sh -c "$(declare -f run_cargo_reaper); run_cargo_reaper \"\$1\"" _ "$1"
        '';
    };
}
