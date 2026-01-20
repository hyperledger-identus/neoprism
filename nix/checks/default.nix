{ ... }:
{
  perSystem =
    { pkgs, self', ... }:
    {
      checks = {
        default = pkgs.callPackage ./neoprism-checks.nix { };
        tools = pkgs.callPackage ./tools-checks.nix { };
      }
      // {
        inherit (self'.packages)
          docs-site
          neoprism-ui-assets
          neoprism-bin
          neoprism-bin-x86_64-linux
          neoprism-bin-aarch64-linux
          neoprism-docker
          neoprism-docker-latest
          neoprism-docker-linux-amd64
          neoprism-docker-linux-arm64
          ;
      };
    };
}
