{ imports ? [ ]
, cargo-reaper
, mkReaperDryRun
, mkCargoReaperDryRun
, ...
}:
{
  nodes = {
    corro = { config, pkgs, ... }:
      let
        user = "corro";
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

        # Enable audio via pipewire.
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

  # Link the pre-built plugin using `cargo-reaper link` and
  # assert the symbolic link exists in the `UserPlugins` directory.
  test-cargo-reaper-link = { plugin, plugin_name }: ''
    corro.start()
    corro.wait_for_unit("multi-user.target")
    corro.succeed("reaper_dry_run \"${plugin_name} error\"")
    corro.succeed("su - corro -c '${cargo-reaper}/bin/cargo-reaper link ${plugin}/lib/${plugin_name}.*'")
    corro.succeed("su - corro -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'")
  '';

  # Copy plugin source code and its pre-vendored dependencies into
  # a directory and run `cargo-reaper run` in offline mode.
  test-cargo-reaper-run = { plugin_source, plugin_vendor, plugin_name }: ''
    corro.start()
    corro.wait_for_unit("multi-user.target")
    corro.succeed("reaper_dry_run \"${plugin_name} error\"")
    corro.succeed("su - root -c 'cp -r ${plugin_source}/* /home/corro/'")
    corro.succeed("su - root -c 'mkdir -p /home/corro/.cargo && cp -r ${plugin_vendor}/config.toml /home/corro/.cargo/'")
    corro.succeed("cargo_reaper_dry_run \"${plugin_name} error\"")
  '';

  # Link the pre-built plugin using `cargo-reaper link` and
  # assert the symbolic link exists in the `UserPlugins` directory.
  # Copy plugin source code into a directory and run `cargo-reaper clean`,
  # then assert the plugin link no longer exists in the `UserPlugins` directory.
  test-cargo-reaper-clean = { plugin, plugin_source, plugin_name }: ''
    corro.start()
    corro.wait_for_unit("multi-user.target")
    corro.succeed("reaper_dry_run \"${plugin_name} error\"")
    corro.succeed("su - corro -c '${cargo-reaper}/bin/cargo-reaper link ${plugin}/lib/${plugin_name}.*'")
    corro.succeed("su - corro -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'")
    corro.succeed("su - root -c 'cp -r ${plugin_source}/* /home/corro/'")
    corro.succeed("su - corro -c '${cargo-reaper}/bin/cargo-reaper clean -p ${plugin_name}'")
    corro.fail("su - corro -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'")
  '';
}
