{
  description = "plainchant";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.zst";
    systems.url = "github:nix-systems/x86_64-linux";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{
    nixpkgs,
    crane,
    rust-overlay,
    flake-parts,
    ...
  }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = [ "x86_64-linux" ];
    perSystem = { system, ... }: let

      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };

      _craneLib = crane.mkLib pkgs;

      craneLibStable = _craneLib.overrideToolchain
        (p: p.rust-bin.stable.latest.default);

      plainchant = profile:
        let
          craneArgs = {
            src = craneLibStable.cleanCargoSource ./.;
            strictDeps = true;
            CARGO_PROFILE = profile;
          };
        in
        craneLibStable.buildPackage (craneArgs // {
          cargoArtifacts = craneLibStable.buildDepsOnly craneArgs;

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

      devShells.default = craneLibStable.devShell {
        packages = [
          pkgs.rust-analyzer
          pkgs.rust-bin.nightly.latest.rustfmt
        ];
      };
    };
  };
}
