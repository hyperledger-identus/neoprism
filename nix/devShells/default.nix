{ self, pkgs }:

{
  default = import ./neoprism.nix { inherit pkgs; };
  cardano = import ./cardano.nix { inherit pkgs; };
  prism-test = import ./prism-test.nix { inherit pkgs; };

  docs = import ./docs.nix { inherit pkgs self; };
}
