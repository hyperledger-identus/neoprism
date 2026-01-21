{ ... }:

{
  perSystem =
    {
      pkgs,
      system,
      self',
      neoprismLib,
      ...
    }:
    let
      inherit (neoprismLib) version;
      dockerBuildConfig = {
        x86_64-linux = {
          callPackage = pkgs.pkgsCross.gnu64.callPackage;
          neoprism-bin = self'.packages.neoprism-bin-x86_64-linux;
        };
        aarch64-darwin = {
          # macOS builds Linux ARM64 containers
          callPackage = pkgs.pkgsCross.aarch64-multiplatform.callPackage;
          neoprism-bin = self'.packages.neoprism-bin-aarch64-linux;
        };
      };
    in
    {
      packages = rec {
        # native built images
        neoprism-docker = dockerBuildConfig.${system}.callPackage ./packages/neoprism-docker.nix {
          inherit version;
          neoprism-ui-assets = self'.packages.neoprism-ui-assets;
          inherit (dockerBuildConfig.${system}) neoprism-bin;
        };
        neoprism-docker-latest = neoprism-docker.override { version = "latest"; };

        # cross built images
        neoprism-docker-linux-amd64 = pkgs.pkgsCross.gnu64.callPackage ./packages/neoprism-docker.nix {
          inherit version;
          neoprism-ui-assets = self'.packages.neoprism-ui-assets;
          neoprism-bin = self'.packages.neoprism-bin-x86_64-linux;
          tagSuffix = "-amd64";
        };
        neoprism-docker-linux-arm64 =
          pkgs.pkgsCross.aarch64-multiplatform.callPackage ./packages/neoprism-docker.nix
            {
              inherit version;
              neoprism-ui-assets = self'.packages.neoprism-ui-assets;
              neoprism-bin = self'.packages.neoprism-bin-aarch64-linux;
              tagSuffix = "-arm64";
            };
      };
    };
}
