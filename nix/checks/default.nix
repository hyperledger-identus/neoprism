{ ... }:
{
  perSystem =
    {
      pkgs,
      self',
      neoprismLib,
      ...
    }:
    {
      checks = {
        default = pkgs.callPackage ./neoprism-checks.nix {
          rustTools = neoprismLib.rustTools;
        };
        tools = pkgs.callPackage ./tools-checks.nix {
          pythonTools = neoprismLib.pythonTools;
        };
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
