final: prev:
let
  inherit (prev) lib rustPlatform;

  manifest = (builtins.fromTOML (builtins.readFile ../../../Cargo.toml)).package;
  src = lib.cleanSourceWith {
    inherit (manifest) name;
    src = lib.cleanSource ../../../.;
    filter = path: type:
      let
        path' = (toString path);
        base = baseNameOf path';
        parentDir = baseNameOf (dirOf path');
        matchesSuffix = lib.any (suffix: lib.hasSuffix suffix base) [
          ".rs"
          ".toml"
        ];
        isCargoFile = base == "Cargo.lock";
        isCargoConfig = parentDir == ".cargo" && base == "config";
      in
      type == "directory" || matchesSuffix || isCargoFile || isCargoConfig;
  };

  commonArgs = {
    inherit src;
    inherit (manifest) version;
    pname = manifest.name;
    strictDeps = true;
    cargoLock.lockFile = src + "/Cargo.lock";

    nativeBuildInputs = with prev; [
      installShellFiles
    ] ++ lib.optionals stdenv.isLinux [
      autoPatchelfHook
    ];

    buildInputs = with prev; [
      libgcc
    ] ++ lib.optionals stdenv.isDarwin [
      libiconv
    ];
  };
in
{
  cargo-reaper = rustPlatform.buildRustPackage (commonArgs // {
    # NOTE: `installShellCompletion` only has support for Bash, Zsh and Fish
    postInstall = ''
      installShellCompletion --cmd cargo-reaper \
        --bash <($out/bin/cargo-reaper completions bash) \
        --fish <($out/bin/cargo-reaper completions fish) \
        --zsh <($out/bin/cargo-reaper completions zsh)
    '';
    doCheck = false;
    passthru.tests = {
      cargo-clippy = rustPlatform.buildRustPackage (commonArgs // {
        pname = "${manifest.name}-clippy";
        nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ prev.clippy ];
        buildPhase = ''
          runHook preBuild
          cargo clippy --all-targets -- --deny warnings
          runHook postBuild
        '';
        installPhase = ''
          runHook preInstall
          touch $out
          runHook postInstall
        '';
        doCheck = false;
      });
      cargo-doc = rustPlatform.buildRustPackage (commonArgs // {
        pname = "${manifest.name}-doc";
        buildPhase = ''
          runHook preBuild
          cargo doc --no-deps
          runHook postBuild
        '';
        installPhase = ''
          runHook preInstall
          cp -r target/doc $out
          runHook postInstall
        '';
        doCheck = false;
      });
      # `cargo fmt` and `cargo deny` both shell out to `cargo metadata`
      # internally, which wants a resolved dependency graph — so, unlike
      # `taplo fmt` below, they need the same vendored-deps setup as the
      # clippy/doc/nextest checks to stay network-free in the sandbox.
      cargo-fmt = rustPlatform.buildRustPackage (commonArgs // {
        pname = "${manifest.name}-fmt";
        nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ prev.rustfmt ];
        buildPhase = ''
          runHook preBuild
          cargo fmt --check
          runHook postBuild
        '';
        installPhase = ''
          runHook preInstall
          touch $out
          runHook postInstall
        '';
        doCheck = false;
      });
      taplo-fmt = prev.runCommand "${manifest.name}-taplo-fmt"
        {
          nativeBuildInputs = [ prev.taplo ];
        } ''
        export HOME=$TMPDIR
        cd ${lib.sources.sourceFilesBySuffices src [ ".toml" ]}
        taplo fmt --check
        touch $out
      '';
      cargo-deny = rustPlatform.buildRustPackage (commonArgs // {
        pname = "${manifest.name}-deny";
        nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ prev.cargo-deny ];
        buildPhase = ''
          runHook preBuild
          cargo deny check --config .deny.toml bans licenses sources
          runHook postBuild
        '';
        installPhase = ''
          runHook preInstall
          touch $out
          runHook postInstall
        '';
        doCheck = false;
      });
      cargo-nextest = rustPlatform.buildRustPackage (commonArgs // {
        pname = "${manifest.name}-nextest";
        nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ prev.cargo-nextest ];
        buildPhase = ''
          runHook preBuild
          cargo nextest run --no-tests=warn
          runHook postBuild
        '';
        installPhase = ''
          runHook preInstall
          touch $out
          runHook postInstall
        '';
        doCheck = false;
      });
    };
  });

  lib = prev.lib // {
    fileset = prev.lib.fileset // {
      cargoReaperConfigFilter = from: prev.lib.fileset.fileFilter (file: (builtins.match "\.?reaper\.toml" file.name) != null) from;
    };
  };

  rustPlatform = prev.rustPlatform.overrideScope (rfinal: rprev:
    let
      # `rustToolchain` is only ever set by a caller that needs a
      # fenix-provided cross-target sysroot (see `crossChecks` in
      # release.nix) — everywhere else this resolves to plain
      # nixpkgs `rustPlatform.buildRustPackage`.
      useFenix = rfinal ? rustToolchain;
      buildRustPackage =
        if useFenix then
          (prev.makeRustPlatform {
            rustc = rfinal.rustToolchain;
            cargo = rfinal.rustToolchain;
          }).buildRustPackage
        else
          rfinal.buildRustPackage
        ;
    in
    {
      buildReaperExtension =
        { package
        , plugin ? package
        , target ? null
        , ...
        }@crateArgs:
        let
          # Run `cargo-reaper`, passing trailing args to the cargo invocation.
          # We do not symlink the plugin since the `UserPlugins` directory is in
          # the `$HOME` directory which is inaccessible to the sandbox.
          buildPhaseCargoCommand = ''
            cargo reaper build --no-symlink \
              -p ${package} --lib \
              --release ${lib.optionalString (target != null) ''\
              --target ${target}
            ''}
          '';
          # Include extension plugin in the build result.
          installPhaseCommand = ''
            mkdir -p $out/lib
            mv target${lib.optionalString (target != null) "/${target}"}/release/${plugin}.* $out/lib
          '';
        in
        buildRustPackage (crateArgs // {
          pname = package;
          # `buildRustPackage` requires a version to name the derivation;
          # these are test-fixture builds with no meaningful version of
          # their own, so a placeholder is fine unless the caller sets one.
          version = crateArgs.version or "0.0.0";
          cargoLock = crateArgs.cargoLock or {
            lockFile = crateArgs.src + "/Cargo.lock";
            allowBuiltinFetchGit = true;
          };
          # `buildPhase` above is already the meaningful check (the build
          # either produces a working plugin or it doesn't) — running the
          # default `checkPhase` on top would just recompile the whole
          # dependency graph a second time for `cargo test` to find nothing.
          # Off by default; callers that actually have tests can opt in.
          doCheck = crateArgs.doCheck or false;
          buildPhase = ''
            runHook preBuild
            ${buildPhaseCargoCommand}
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            ${installPhaseCommand}
            runHook postInstall
          '';
          nativeBuildInputs = (crateArgs.nativeBuildInputs or [ ]) ++ [
            # Add `cargo-reaper` as a build time dependency of this derivation.
            final.cargo-reaper
          ];
        });
    });
}
