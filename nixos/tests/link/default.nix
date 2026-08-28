{ lib, testers, pkgsLinux }:
let
  plugin = pkgsLinux.callPackage ../../../nix/tests/plugin_manifests/package_manifest { };
  plugin_name = "reaper_package_ext";
in
testers.runNixOSTest {
  name = "test-cargo-reaper-link";
  node.pkgs = lib.mkForce pkgsLinux;
  nodes.corro = import ../profile.nix;
  testScript = ''
    corro.start()
    corro.wait_for_unit("multi-user.target")

    # Launch REAPER once to initialize `~/.config/REAPER/UserPlugins`
    corro.succeed("su - corro -c 'cargo-reaper run --no-build --headless --timeout 5s --stdout null --stderr null'", timeout=20)

    corro.succeed("su - corro -c 'cargo-reaper link ${plugin}/lib/${plugin_name}.*'", timeout=15)
    corro.succeed("su - corro -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'", timeout=15)
  '';
}
