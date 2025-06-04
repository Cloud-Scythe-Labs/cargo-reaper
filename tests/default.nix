{ pkgs, imports ? [ ], cargo-reaper, ... }:
let
  # REAPER doesn't like to execute in the background, and NixOS Test invokes commands
  # as root. To combat this, we change users based on machine before launching REAPER,
  # using xvfb and xdotool to search for an error window before exiting successfully.
  mkReaperDryRun =
    { user
    , reaper
    , xdotool
    , xvfb-run
    }:
    pkgs.writeShellScriptBin "reaper_dry_run" ''
      function run_reaper() {
          su - ${user} -c ${reaper}/bin/reaper &
          export reaper_pid=$!
          sleep 5
          error_window=$(${xdotool}/bin/xdotool search --name "$1")
          if [[ -n "$error_window" ]]; then
              echo "found error window with ID: $error_window"
              exit 1
          fi
          kill $reaper_pid
      }

      echo "searching for error window title '$1'"
      ${xvfb-run}/bin/xvfb-run -a sh -c "$(declare -f run_reaper); run_reaper \"\$1\"" _ "$1"
    '';
  mkCargoReaperDryRun =
    { user
    , cargo-reaper
    , xdotool
    , xvfb-run
    }:
    pkgs.writeShellScriptBin "cargo_reaper_dry_run" ''
      function run_cargo_reaper() {
          su - ${user} -c '${cargo-reaper}/bin/cargo-reaper run &'
          export reaper_pid=$!
          sleep 5
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
in
{
  nodes = {
    ferris = { config, pkgs, ... }:
      let
        user = "ferris";
      in
      {
        inherit imports;

        users.users = {
          "${user}" = {
            isNormalUser = true;
            description = user;
            home = "/home/${user}";
            createHome = true;
          };
          root = {
            hashedPassword = "";
            hashedPasswordFile = null;
          };
        };

        services.pulseaudio.enable = false;
        security.rtkit.enable = true;
        services.pipewire = {
          enable = true;
          alsa.enable = true;
          alsa.support32Bit = true;
          pulse.enable = true;
          jack.enable = true;
        };

        environment.systemPackages = with pkgs; [
          reaper
          xdotool
          xvfb-run
        ] ++ [
          cargo-reaper
          (mkReaperDryRun {
            inherit user;
            inherit (pkgs) reaper xdotool xvfb-run;
          })
          (mkCargoReaperDryRun {
            inherit user cargo-reaper;
            inherit (pkgs) xdotool xvfb-run;
          })
        ];
      };
  };

  test-cargo-reaper-link = { plugin, plugin_name }: ''
    ferris.start()
    ferris.wait_for_unit("multi-user.target")
    ferris.succeed("reaper_dry_run \"${plugin_name} error\"")
    ferris.succeed("su - ferris -c '${cargo-reaper}/bin/cargo-reaper link ${plugin}/lib/${plugin_name}.*'")
    ferris.succeed("su - ferris -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'")
  '';

  test-cargo-reaper-run = { plugin_source, plugin_name }: ''
    ferris.start()
    ferris.wait_for_unit("multi-user.target")
    ferris.succeed("reaper_dry_run \"${plugin_name} error\"")
    ferris.succeed("su - root -c 'cp -r ${plugin_source}/* /home/ferris/ && chmod -R 755 /home/ferris/'")
    ferris.succeed("cargo_reaper_dry_run \"${plugin_name} error\"")
  '';
}
