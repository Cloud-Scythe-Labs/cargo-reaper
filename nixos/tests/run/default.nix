{ lib, testers, rustPlatform, pkgsLinux }:
let
  plugin_source = lib.cleanSourceWith {
    src = lib.cleanSource ../../../nix/tests/plugin_manifests/package_manifest;
    filter = path: type: baseNameOf path != "target" && !lib.hasSuffix ".nix" (baseNameOf path);
  };
  plugin_vendor = rustPlatform.importCargoLock {
    lockFile = plugin_source + "/Cargo.lock";
    allowBuiltinFetchGit = true;
  };
  plugin_name = "reaper_package_ext";
in
testers.runNixOSTest {
  name = "test-cargo-reaper-run";
  node.pkgs = lib.mkForce pkgsLinux;
  containers.corro = {
    imports = [ ../profile.nix ];
    environment.systemPackages = with pkgsLinux; [
      cargo
      rustc
      gcc
    ];
  };
  testScript = ''
    corro.start()
    corro.wait_for_unit("multi-user.target")

    # Launch REAPER once to initialize `~/.config/REAPER/UserPlugins`
    corro.succeed("su - corro -c 'cargo-reaper run --no-build --headless --display :99 --timeout 5s --stdout null --stderr null'", timeout=20)

    corro.succeed("su - root -c 'cp -r ${plugin_source}/* /home/corro/'", timeout=15)
    # `${plugin_vendor}/.cargo/config.toml` points at a *relative*
    # `cargo-vendor-dir`, so the whole vendor tree (config included) has
    # to land at that exact relative path from $HOME, not just the config.
    corro.succeed("su - root -c 'cp -r ${plugin_vendor} /home/corro/cargo-vendor-dir'", timeout=15)
    corro.succeed("su - root -c 'mkdir -p /home/corro/.cargo && cp /home/corro/cargo-vendor-dir/.cargo/config.toml /home/corro/.cargo/'", timeout=15)

    status, output = corro.execute(
        "su - corro -c 'cargo-reaper run --headless --display :99 --keep-going "
        "--locate-window \"${plugin_name} error\" "
        "--timeout 5s --stdout null --stderr null -- --release --offline'",
        timeout=180,
    )
    assert status != 0, f"'${plugin_name} error' window detected, REAPER output:\n{output}"
  '';
}
