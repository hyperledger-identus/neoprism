{ pkgs }:

{
  scala-did = pkgs.callPackage ./scala-did { };
  compactc = pkgs.callPackage ./compactc.nix { };
  midnight-js-binding = pkgs.callPackage ./midnight-js-binding.nix { };
}
