{ lib, testers, pkgsLinux }:
let
  plugin = pkgsLinux.callPackage ../../../nix/tests/plugin_manifests/package_manifest { };
  plugin_source = lib.cleanSourceWith {
    src = lib.cleanSource ../../../nix/tests/plugin_manifests/package_manifest;
    filter = path: type: baseNameOf path != "target" && !lib.hasSuffix ".nix" (baseNameOf path);
  };
  plugin_name = "reaper_package_ext";
in
testers.runNixOSTest {
  name = "test-cargo-reaper-clean";
  node.pkgs = lib.mkForce pkgsLinux;
  containers.corro = import ../profile.nix;
  testScript = ''
    corro.start()
    corro.wait_for_unit("multi-user.target")

    # Launch REAPER once to initialize `~/.config/REAPER/UserPlugins`
    corro.succeed("su - corro -c 'cargo-reaper run --no-build --headless --timeout 5s --stdout null --stderr null'", timeout=20)

    corro.succeed("su - corro -c 'cargo-reaper link ${plugin}/lib/${plugin_name}.*'", timeout=15)
    corro.succeed("su - corro -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'", timeout=15)

    corro.succeed("su - root -c 'cp -r ${plugin_source}/* /home/corro/'", timeout=15)
    corro.succeed("su - corro -c 'cargo-reaper clean -p ${plugin_name}'", timeout=15)
    corro.fail("su - corro -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'", timeout=15)
  '';
}
