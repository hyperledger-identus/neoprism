{ ... }:
{
  perSystem =
    { pkgs, self', ... }:
    {
      devShells = {
        default = import ./development.nix { inherit pkgs; };
        cardano = import ./cardano.nix { inherit pkgs; };
        docs = import ./docs.nix { inherit pkgs self'; };
      };
    };
}
