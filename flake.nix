{
  description = "A rust implementation of PRISM node";

  nixConfig = {
    extra-substituters = [ "https://cache.iog.io" ];
    extra-trusted-public-keys = [ "hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ=" ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-parts.url = "github:hercules-ci/flake-parts";
    cardano-node.url = "github:IntersectMBO/cardano-node/10.5.1";
    cardano-db-sync.url = "github:IntersectMBO/cardano-db-sync/13.6.0.5";
    cardano-wallet.url = "github:cardano-foundation/cardano-wallet/v2025-03-31";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-parts,
      cardano-node,
      cardano-db-sync,
      cardano-wallet,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
      ];

      imports = [
        ./nix/devShells
        ./nix/checks
        ./nix/docs
        ./nix/neoprism
        ./nix/cardano
        ./nix/neoprismLib.nix
      ];

      perSystem =
        {
          system,
          ...
        }:
        {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            config.unfree = true;
            overlays = [
              (import rust-overlay)
              (_: prev: {
                rustTools = prev.callPackage ./nix/rustTools.nix { inherit rust-overlay; };
                pythonTools = prev.callPackage ./nix/pythonTools.nix { };
                inherit (cardano-node.packages.${system})
                  cardano-cli
                  cardano-node
                  cardano-testnet
                  cardano-submit-api
                  ;
                inherit (cardano-wallet.packages.${system}) cardano-wallet;
                cardano-db-sync = cardano-db-sync.packages.${system}."cardano-db-sync:exe:cardano-db-sync";
              })
            ];
          };
        };
    };
}
