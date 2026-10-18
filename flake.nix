{
  description = "plainchant";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.zst";
    systems.url = "github:nix-systems/x86_64-linux";
    crane.url = "github:ipetkov/crane";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs@{
    nixpkgs,
    crane,
    flake-parts,
    ...
  }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = [ "x86_64-linux" ];
    perSystem = { pkgs, ... }: let

      craneLib = crane.mkLib pkgs;

      plainchant = profile:
        let
          craneArgs = {
            src = craneLib.cleanCargoSource ./.;
            strictDeps = true;
            CARGO_PROFILE = profile;
          };
        in
        craneLib.buildPackage (craneArgs // {
          cargoArtifacts = craneLib.buildDepsOnly craneArgs;

          buildInputs = [ pkgs.sqlite ];

          postInstall = ''
            mkdir -p $out/etc/plainchant
            cp -r static/ $out/etc/plainchant/
            cp -r templates/ $out/etc/plainchant/
          '';
        });
    in
    {
      packages.default = plainchant "release";
      packages.debug = plainchant "dev";
      packages.release = plainchant "release";

      devShells.default = craneLib.devShell {
        packages = [
          pkgs.rust-analyzer
        ];
      };
    };
  };
}
