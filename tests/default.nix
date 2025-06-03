{ pkgs, imports, ... }:
let
  # REAPER doesn't like to execute in the background, and NixOS Test invokes commands
  # as root. To combat this, we change users based on machine before launching REAPER,
  # using xvfb and xdotool to search for an error window before exiting successfully.
  mkReaperDryRun =
    { user
    , reaper ? pkgs.reaper
    , xdotool ? pkgs.xdotool
    , xvfb-run ? pkgs.xvfb-run
    }:
    pkgs.writeShellScriptBin "reaper_dry_run" ''
      function run_reaper() {
        su - ${user} -c ${reaper}/bin/reaper &
        export reaper_pid=$!
        sleep 5
        error_window=$(${xdotool}/bin/xdotool search --name 'error')
        if [[ -n "$error_window" ]]; then
            echo "found error window with ID: $error_window"
            exit 1
        fi
        kill $reaper_pid
      }

      ${xvfb-run}/bin/xvfb-run -a sh -c "$(declare -f run_reaper); run_reaper"
    '';
in
{
  machine = { config, pkgs, ... }:
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
        # Our scripts
        root = {
          hashedPassword = "";
          hashedPasswordFile = null;
        };
      };

      environment.systemPackages = with pkgs; [
        reaper
      ] ++ [
        (mkReaperDryRun {
          inherit user;
          inherit (pkgs) reaper xdotool xvfb-run;
        })
      ];
    };

  testCargoReaperNew = ''
    machine.start()
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("network.target")
    machine.succeed("cargo reaper new reaper_test")
  '';
}
