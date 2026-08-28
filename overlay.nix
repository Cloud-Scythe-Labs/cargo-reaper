final: prev:
let
  inherit (prev) lib rustPlatform;

  manifest = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
  src = lib.cleanSourceWith {
    inherit (manifest) name;
    src = lib.cleanSource ./.;
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
    };
  });

  rustPlatform = prev.rustPlatform.overrideScope (rfinal: rprev: {
    buildReaperExtension =
      { package
      , plugin ? package
      , target ? null
      , ...
      }@crateArgs:
      rfinal.buildRustPackage (crateArgs // {
        pname = package;
        version = crateArgs.version or "0.0.0";
        cargoLock = crateArgs.cargoLock or {
          lockFile = crateArgs.src + "/Cargo.lock";
          allowBuiltinFetchGit = true;
        };
        doCheck = crateArgs.doCheck or false;
        # Run `cargo-reaper`, passing trailing args to the cargo invocation.
        # We do not symlink the plugin since the `UserPlugins` directory is in
        # the `$HOME` directory which is inaccessible to the sandbox.
        buildPhase = ''
          runHook preBuild
          cargo reaper build --no-symlink \
            -p ${package} --lib \
            --release ${lib.optionalString (target != null) ''\
            --target ${target}
          ''}
          runHook postBuild
        '';
        # Include extension plugin in the build result.
        installPhase = ''
          runHook preInstall
          mkdir -p $out/lib
          mv target${lib.optionalString (target != null) "/${target}"}/release/${plugin}.* $out/lib
          runHook postInstall
        '';
        nativeBuildInputs = (crateArgs.nativeBuildInputs or [ ]) ++ [
          # Add `cargo-reaper` as a build time dependency of this derivation.
          final.buildPackages.cargo-reaper
        ];
      });
  });
}
