{
  description = "A Cargo plugin for developing REAPER extension and VST plugins with Rust.";

  inputs = { };

  outputs = _:
    let
      overlays.default = import ./overlay.nix;

      eachSystem = f: builtins.listToAttrs (map
        (system: {
          name = system;
          value = f (import ./release.nix { inherit system; });
        })
        [
          "x86_64-linux"
          "aarch64-linux"
          "aarch64-darwin"
        ]);
    in
    {
      inherit overlays;

      apps = eachSystem (release:
        let
          inherit (release) pkgs;
          inherit (pkgs) lib;
          cargo-reaper = {
            type = "app";
            program = "${pkgs.cargo-reaper}/bin/cargo-reaper";
            meta = {
              homepage = "https://github.com/Cloud-Scythe-Labs/cargo-reaper/";
              description = "A Cargo plugin for developing REAPER extension plugins with Rust.";
              license = lib.licenses.mit;
              maintainers = with lib.maintainers; [ eureka-cpu ];
            };
          };
        in
        {
          inherit cargo-reaper;
          default = cargo-reaper;
        });

      formatter = eachSystem (release: release.fmt);
    };
}
