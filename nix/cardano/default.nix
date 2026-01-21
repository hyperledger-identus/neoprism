{ ... }:

{
  perSystem =
    { pkgs, system, ... }:
    let
      dockerBuildConfig = {
        x86_64-linux = {
          callPackage = pkgs.pkgsCross.gnu64.callPackage;
        };
        aarch64-darwin = {
          callPackage = pkgs.pkgsCross.aarch64-multiplatform.callPackage;
        };
      };
    in
    {
      packages = {
        cardano-testnet-docker = dockerBuildConfig.${system}.callPackage ./cardano-testnet-docker.nix { };

        cardano-testnet-docker-linux-amd64 = pkgs.pkgsCross.gnu64.callPackage ./cardano-testnet-docker.nix {
          tagSuffix = "-amd64";
        };

        cardano-testnet-docker-linux-arm64 =
          pkgs.pkgsCross.aarch64-multiplatform.callPackage ./cardano-testnet-docker.nix
            {
              tagSuffix = "-arm64";
            };
      };

      devShells.cardano = pkgs.mkShell {
        packages = with pkgs; [
          nix
          jq
          hurl
          cardano-node
          cardano-cli
          cardano-wallet
          cardano-testnet
          cardano-db-sync
          cardano-submit-api
        ];

        shellHook = ''
          export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
          ${pkgs.cowsay}/bin/cowsay "Working on project root directory: $ROOT_DIR"
          cd "$ROOT_DIR"
        '';

        CARDANO_CLI = "${pkgs.cardano-cli}/bin/cardano-cli";
        CARDANO_NODE = "${pkgs.cardano-node}/bin/cardano-node";
        PRISM_HOME = ".";
      };
    };
}
