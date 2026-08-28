let
  inputs = import ../nix/tamal { };
  # Only used to build `pkgsFor` itself (chicken-and-egg) — plain, not
  # overlaid. Everything else below uses `lib` from an already-overlaid
  # `pkgs` (see below), since `lib.fileset.cargoReaperConfigFilter` only
  # exists once the `cargo-reaper` overlay has been applied.
  bootstrapLib = (import inputs.nixpkgs { }).lib;

  pkgsFor = system: import inputs.nixpkgs {
    inherit system;
    overlays = [
      (import ../nix/overlays/cargo-reaper)
    ];
    config = {
      allowUnfreePredicate = pkg: builtins.elem (bootstrapLib.getName pkg) [
        "reaper"
      ];
    };
  };

  system = builtins.currentSystem;
  hostPkgs = pkgsFor system;

  guestSystem = builtins.replaceStrings [ "darwin" ] [ "linux" ] system;
  guestPkgs = pkgsFor guestSystem;

  inherit (hostPkgs) lib;

  # The fileset for a plugin manifest fixture, duplicated from release.nix's
  # own `testFileset` for now rather than sharing a lib — see release.nix.
  testFileset = root: lib.fileset.toSource {
    inherit root;
    fileset = lib.fileset.unions [
      (root + "/Cargo.toml")
      (root + "/Cargo.lock")
      (root + "/src")
      (lib.fileset.cargoReaperConfigFilter (root + "/reaper.toml"))
    ];
  };

  # Build a plugin manifest fixture against the guest's (Linux) pkgs — the
  # built plugin gets loaded by REAPER running inside the VM, so it must be
  # a Linux build, not whatever system this file happens to be evaluated on.
  buildPluginFixture = { root, package, plugin ? package }:
    guestPkgs.rustPlatform.buildReaperExtension {
      inherit package plugin;
      src = testFileset root;
      strictDeps = true;
      cargoLock = {
        lockFile = testFileset root + "/Cargo.lock";
        allowBuiltinFetchGit = true;
      };
      LD_LIBRARY_PATH = lib.makeLibraryPath [ guestPkgs.stdenv.cc.cc.lib ];
    };

  testProfile = { pkgs, name, ... }:
    {
      nixpkgs.pkgs = guestPkgs;

      users.users = {
        "${name}" = {
          isNormalUser = true;
          description = name;
          home = "/home/${name}";
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

      # Necessary for Xvfb
      services.xserver = {
        enable = true;
        # This can be changed to another DM like xfce if a GUI is needed for debugging
        displayManager.startx.enable = true;
      };

      environment.systemPackages = with pkgs; [
        reaper
        xdotool
        xvfb-run
        cargo-reaper
      ];
    };
in
{
  # Link the pre-built plugin using `cargo-reaper link` and
  # assert the symbolic link exists in the `UserPlugins` directory.
  test-cargo-reaper-link =
    let
      plugin = buildPluginFixture {
        root = ./plugin_manifests/package_manifest;
        package = "package_manifest";
        plugin = "reaper_package_ext";
      };
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
      plugin_vendor = hostPkgs.rustPlatform.importCargoLock {
        lockFile = plugin_source + "/Cargo.lock";
        # The fixture pins git dependencies (`nutype`, the `reaper-rs`
        # workspace) that pull in submodules (e.g. WDL) — the builtin
        # `fetchGit` path fetches those; a pinned-hash `fetchgit` doesn't.
        allowBuiltinFetchGit = true;
      };
      plugin_name = "reaper_package_ext";
    in
    hostPkgs.testers.nixosTest {
      name = "test-cargo-reaper-run";
      nodes.corro = {
        imports = [ testProfile ];
        # A Rust toolchain is required to build the plugin from source in
        # the VM; it must target the guest's Linux system.
        environment.systemPackages = with guestPkgs; [
          cargo
          rustc
          gcc
        ];
      };
      testScript = ''
        corro.start()
        corro.wait_for_unit("multi-user.target")

        # Launch REAPER once to initialize `~/.config/REAPER/UserPlugins`
        corro.succeed("su - corro -c 'cargo-reaper run --no-build --headless --timeout 5s --stdout null --stderr null'", timeout=20)

        corro.succeed("su - root -c 'cp -r ${plugin_source}/* /home/corro/'", timeout=15)
        # `${plugin_vendor}/.cargo/config.toml` points at a *relative*
        # `cargo-vendor-dir`, so the whole vendor tree (config included) has
        # to land at that exact relative path from $HOME, not just the config.
        corro.succeed("su - root -c 'cp -r ${plugin_vendor} /home/corro/cargo-vendor-dir'", timeout=15)
        corro.succeed("su - root -c 'mkdir -p /home/corro/.cargo && cp /home/corro/cargo-vendor-dir/.cargo/config.toml /home/corro/.cargo/'", timeout=15)

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
      plugin = buildPluginFixture {
        root = ./plugin_manifests/package_manifest;
        package = "package_manifest";
        plugin = "reaper_package_ext";
      };
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
  
