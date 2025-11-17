{ pkgs, self, ... }:

{
  default = pkgs.callPackage ./neoprism-checks.nix { };
  tools = pkgs.callPackage ./tools-checks.nix { };
}
// {
  inherit (self.packages.${pkgs.stdenv.hostPlatform.system})
    neoprism-bin
    neoprism-docker
    ;
}
