{
  description = "A rust implementation of PRISM node";

  nixConfig = {
    extra-substituters = [ "https://cache.iog.io" ];
    extra-trusted-public-keys = [ "hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ=" ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    sbt = {
      url = "github:zaninime/sbt-derivation";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    cardano-node.url = "github:IntersectMBO/cardano-node?ref=10.4.1";
    cardano-db-sync.url = "github:IntersectMBO/cardano-db-sync?ref=13.6.0.5";
    cardano-wallet.url = "github:cardano-foundation/cardano-wallet?ref=v2025-03-31";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      sbt,
      flake-utils,
      cardano-node,
      cardano-db-sync,
      cardano-wallet,
      ...
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-darwin" ] (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.unfree = true;
          overlays = [
            (import rust-overlay)
            (_: prev: {
              mkSbtDerivation = sbt.mkSbtDerivation.${pkgs.system};
              rustTools = prev.callPackage ./nix/rustTools.nix { inherit rust-overlay; };
              cardano-cli = cardano-node.packages.${system}.cardano-cli;
              cardano-node = cardano-node.packages.${system}.cardano-node;
              cardano-testnet = cardano-node.packages.${system}.cardano-testnet;
              cardano-wallet = cardano-wallet.packages.${system}.cardano-wallet;
              cardano-db-sync = cardano-db-sync.packages.${system}.default;
            })
            (_: prev: {
              pkgsInternal = import ./nix/pkgsInternal { pkgs = prev; };
            })
          ];
        };
      in
      {
        checks = import ./nix/checks/default.nix { inherit pkgs; };
        devShells = import ./nix/devShells/default.nix { inherit pkgs self; };
        packages = import ./nix/packages/default.nix { inherit pkgs; };
      }
    );
}
