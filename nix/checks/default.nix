{ pkgs, ... }:

{
  default = pkgs.callPackage ./neoprism-checks.nix { };
  python-tools = pkgs.callPackage ./python-tools-checks.nix { };
}
