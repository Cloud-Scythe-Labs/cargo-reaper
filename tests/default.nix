let
  /** Get a node from the flake lock file.

  getFlake' :: String -> Attrset */
  getFlake' = node:
    let source = (builtins.fromJSON (builtins.readFile ../flake.lock)).nodes.${node}.locked; in
    {
      inherit (source) rev;
      outPath = fetchTarball
        (
          let inherit (source) owner repo rev narHash; in {
            url = "https://github.com/${owner}/${repo}/archive/${rev}.tar.gz";
            sha256 = narHash;
          }
        );
    };
in
{ nixpkgs ? getFlake' "nixpkgs" }:

let
  inherit (builtins.getFlake ../.) overlays checks;

  pkgsFor = system: import nixpkgs {
    inherit system;
    overlays = [ overlays.default ];
  };

  system = builtins.currentSystem;
  hostPkgs = pkgsFor system;

  guestSystem = builtins.replaceStrings [ "darwin" ] [ "linux" ] system;
  guestPkgs = pkgsFor guestSystem;
in
{
  # Link the pre-built plugin using `cargo-reaper link` and
  # assert the symbolic link exists in the `UserPlugins` directory.
  test-cargo-reaper-link =
    let
      plugin = checks.${guestSystem}.test-cargo-reaper-build-package-manifest;
      plugin_name = "reaper_package_ext";
    in
    hostPkgs.testers.nixosTest {
      name = "test-cargo-reaper-link";
      nodes.corro = testProfile;
      testScript = ''
        corro.start()
        corro.wait_for_unit("multi-user.target")

        # Launch REAPER once to initialize `~/.config/REAPER/UserPlugins`
        corro.succeed("su - corro -c 'cargo-reaper run --no-build --headless --timeout 5s --stdout null --stderr null'", timeout=20)

        corro.succeed("su - corro -c 'cargo-reaper link ${plugin}/lib/${plugin_name}.*'", timeout=15)
        corro.succeed("su - corro -c 'test -e ~/.config/REAPER/UserPlugins/${plugin_name}.*'", timeout=15)
      '';
    };

  # Copy plugin source code and its pre-vendored dependencies onto the
  # machine, build the plugin from source and open REAPER, asserting that
  # no error window (titled after the plugin) ever appears. `--keep-going`
  # keeps watching until the timeout instead of exiting on the first
  # match, so the run exits non-zero only if the window is never seen.
  test-cargo-reaper-run =
    let
      plugin_source = testFileset ./plugin_manifests/package_manifest;
      plugin_vendor = craneLib.vendorCargoDeps { src = plugin_source; };
      plugin_name = "reaper_package_ext";
    in
    hostPkgs.testers.nixosTest {
      name = "test-cargo-reaper-run";
      nodes.corro = {
        imports = [ testProfile ];
        # A Rust toolchain is required to build the plugin from source in
        # the VM; it must target the guest's Linux system.
        environment.systemPackages = with guestPkgs; [
          cargoReaper.rustToolchain
          gcc
        ];
      };
      testScript = ''
        corro.start()
        corro.wait_for_unit("multi-user.target")

        # Launch REAPER once to initialize `~/.config/REAPER/UserPlugins`
        corro.succeed("su - corro -c 'cargo-reaper run --no-build --headless --timeout 5s --stdout null --stderr null'", timeout=20)

        corro.succeed("su - root -c 'cp -r ${plugin_source}/* /home/corro/'", timeout=15)
        corro.succeed("su - root -c 'mkdir -p /home/corro/.cargo && cp -r ${plugin_vendor}/config.toml /home/corro/.cargo/'", timeout=15)

        status, output = corro.execute(
            "su - corro -c 'cargo-reaper run --headless --keep-going "
            "--locate-window \"${plugin_name} error\" "
            "--timeout 5s --stdout null --stderr null -- --release --offline'",
            timeout=180,
        )
        assert status != 0, f"'${plugin_name} error' window detected, REAPER output:\n{output}"
      '';
    };

  # Link the pre-built plugin using `cargo-reaper link`, then run
  # `cargo-reaper clean` and assert the plugin link no longer exists in
  # the `UserPlugins` directory.
  test-cargo-reaper-clean =
    let
      plugin = checks.${guestSystem}.test-cargo-reaper-build-package-manifest;
      plugin_source = testFileset ./plugin_manifests/package_manifest;
      plugin_name = "reaper_package_ext";
    in
    hostPkgs.testers.nixosTest {
      name = "test-cargo-reaper-clean";
      nodes.corro = testProfile;
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
    };
}
  
