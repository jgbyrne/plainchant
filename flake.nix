{
  description = "plainchant";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.zst";
    systems.url = "github:nix-systems/x86_64-linux";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs@{
    nixpkgs,
    flake-parts,
    rust-overlay,
    ...
  }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = [ "x86_64-linux" ];
    perSystem = { system, ... }: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rust-dist = pkgs.rust-bin.stable.latest.default;
      rust-platform = pkgs.makeRustPlatform {
        cargo = rust-dist;
        rustc = rust-dist;
      };

      plainchant = buildType: rust-platform.buildRustPackage {
        name = with builtins; (fromTOML (readFile ./Cargo.toml)).package.name;
        src = ./.;
        buildInputs = [ pkgs.sqlite ];
        buildType = buildType;
        meta.description = "A lightweight and libre imageboard";
        meta.mainProgram = "plainchant";
        cargoLock.lockFile = ./Cargo.lock;

        postInstall = ''
          mkdir -p $out/etc/plainchant
          cp -r static/ $out/etc/plainchant/
          cp -r templates/ $out/etc/plainchant/
        '';
      };
    in
    {
      packages.default = plainchant "debug";
      packages.release = plainchant "release";

      devShells.default = pkgs.mkShell {
        packages = [
          rust-dist
          pkgs.rust-analyzer
        ];
      };
    };
  };
}
