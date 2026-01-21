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
          neoprism-docker
          neoprism-docker-latest
          ;
      };
    };
}
