{ pkgs, self', ... }:

{
  default = pkgs.callPackage ./neoprism-checks.nix { };
  tools = pkgs.callPackage ./tools-checks.nix { };
}
// {
  inherit (self'.packages)
    docs-site
    neoprism-bin
    neoprism-docker
    neoprism-docker-latest
    ;
}
