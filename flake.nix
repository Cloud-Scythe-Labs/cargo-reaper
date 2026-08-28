{
  description = "A Cargo plugin for developing REAPER extension and VST plugins with Rust.";

  inputs = { };

  outputs = _:
    let
      overlays.default = import ./nix/overlays/cargo-reaper;

      eachSystem = f: builtins.listToAttrs (map (system:
        let
          inputs = import ./nix/tamal { inherit system; };
        in
        {
          name = system;
          value = f (import inputs.nixpkgs {
            inherit system;
            overlays = [ overlays.default ];
          });
        })
        [
          "x86_64-linux"
          "aarch64-linux"
          "aarch64-darwin"
        ]);
    in
    {
      inherit overlays;

      apps = eachSystem (pkgs:
        let
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
    };
}
