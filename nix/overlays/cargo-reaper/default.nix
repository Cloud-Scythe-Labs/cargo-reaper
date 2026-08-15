final: prev:
let
  inherit (prev) lib rustPlatform;

  useFenix = rustPlatform ? rustToolchain;
  useCrane = rustPlatform ? buildPackage;
  buildRustPackage =
    if useCrane then
      rustPlatform.buildPackage
    else if useFenix then
      (prev.makeRustPlatform {
        rustc = rustPlatform.rustToolchain;
        cargo = rustPlatform.rustToolchain;
      }).buildRustPackage
    else
      rustPlatform.buildRustPackage
  ;

  manifest = (builtins.fromTOML (builtins.readFile ../../../Cargo.toml)).package;
  src = lib.cleanSourceWith {
    inherit (manifest) name;
    src = lib.cleanSource ../../../.;
    filter = orig_path: type:
      let
        path = (toString orig_path);
        base = baseNameOf path;
        parentDir = baseNameOf (dirOf path);
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
    strictDeps = true;
    __structuredAttrs = true;

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
  } // lib.optionalAttrs (!useCrane) {
    inherit (manifest) version;
    pname = manifest.name;
    cargoLock.lockFile = src + "/Cargo.lock";
  };

  cargoArtifactsAttrs = lib.optionalAttrs (rustPlatform ? buildDepsOnly) {
    cargoArtifacts = rustPlatform.buildDepsOnly commonArgs;
  };
in
{
  cargo-reaper = buildRustPackage (commonArgs // cargoArtifactsAttrs // {
    # NOTE: `installShellCompletion` only has support for Bash, Zsh and Fish
    postInstall = ''
      installShellCompletion --cmd cargo-reaper \
        --bash <($out/bin/cargo-reaper completions bash) \
        --fish <($out/bin/cargo-reaper completions fish) \
        --zsh <($out/bin/cargo-reaper completions zsh)
    '';
    doCheck = false;
    passthru = lib.optionalAttrs useCrane {
      tests = {
        cargo-clippy = rustPlatform.cargoClippy (commonArgs // {
          inherit (cargoArtifactsAttrs) cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });
        cargo-doc = rustPlatform.cargoDoc (commonArgs // {
          inherit (cargoArtifactsAttrs) cargoArtifacts;
        });
        cargo-fmt = rustPlatform.cargoFmt { inherit src; };
        taplo-fmt = rustPlatform.taploFmt {
          src = lib.sources.sourceFilesBySuffices src [ ".toml" ];
        };
        cargo-deny = rustPlatform.cargoDeny { inherit src; };
        cargo-nextest = rustPlatform.cargoNextest (commonArgs // {
          inherit (cargoArtifactsAttrs) cargoArtifacts;
          partitions = 1;
          partitionType = "count";
          cargoNextestPartitionsExtraArgs = "--no-tests=warn";
        });
      };
    };
  });

  lib = prev.lib // {
    fileset = prev.lib.fileset // {
      cargoReaperConfigFilter = from: prev.lib.fileset.fileFilter (file: (builtins.match "\.?reaper\.toml" file.name) != null) from;
    };
  };

  rustPlatform = prev.rustPlatform.overrideScope (rfinal: rprev:
    let
      useFenix = rfinal ? rustToolchain;
      useCrane = rfinal ? buildPackage;
      buildRustPackage =
        if useCrane then
          rfinal.buildPackage
        else if useFenix then
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

          phaseArgs =
            if useCrane then {
              inherit buildPhaseCargoCommand installPhaseCommand;
              # Bypass crane checks for target install paths.
              doNotPostBuildInstallCargoBinaries = true;
            } else {
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
            };
        in
        buildRustPackage (crateArgs // phaseArgs // {
          pname = package;
          nativeBuildInputs = (crateArgs.nativeBuildInputs or [ ]) ++ [
            # Add `cargo-reaper` as a build time dependency of this derivation.
            final.cargo-reaper
          ];
        });
    });
}
