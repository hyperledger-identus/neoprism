{ pkgs }:

rec {
  scala-did = pkgs.callPackage ./scala-did { };
  compactc = pkgs.callPackage ./compactc.nix { };
  did-midnight-serde = pkgs.callPackage ./did-midnight-serde.nix { inherit compactc; };
}
