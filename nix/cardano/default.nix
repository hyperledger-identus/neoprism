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
    };
}
