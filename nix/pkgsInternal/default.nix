{ pkgs }:

rec {
  scala-did = pkgs.callPackage ./scala-did { };
  compactc = pkgs.callPackage ./compactc.nix { };

}
