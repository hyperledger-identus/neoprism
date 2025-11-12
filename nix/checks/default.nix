{ pkgs, ... }:

{
  default = pkgs.callPackage ./neoprism-checks.nix { };
  tools = pkgs.callPackage ./tools-checks.nix { };
}
