{ ... }:

{
  perSystem =
    { pkgs, self', ... }:
    let
      version = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile ../../version);
    in
    {
      packages = {
        docs-site = pkgs.callPackage ./docs-site.nix {
          inherit version;
          neoprism-bin = self'.packages.neoprism-bin;
        };
      };
    };
}
