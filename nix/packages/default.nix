{ ... }:

{
  perSystem =
    { pkgs, self', ... }:
    let
      hostSystem = pkgs.stdenv.hostPlatform.system;
      version = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile ../../version);
      dockerCrossPlatformConfig = {
        x86_64-linux = pkgs.pkgsCross.gnu64;
        aarch64-darwin = pkgs.pkgsCross.aarch64-multiplatform;
      };
    in
    {
      packages = {
        # docs-site
        docs-site = pkgs.callPackage ./docs-site.nix {
          inherit version;
          neoprism-bin = self'.packages.neoprism-bin;
        };

        # cardano-testnet docker
        cardano-testnet-docker =
          dockerCrossPlatformConfig.${hostSystem}.callPackage ./cardano-testnet-docker.nix
            { };
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
