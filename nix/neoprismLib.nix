{ ... }:
{
  perSystem =
    { pkgs, rust-overlay, ... }:
    {
      _module.args.neoprismLib = {
        version = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile ../version);
        rustTools = pkgs.callPackage ./rustTools.nix { inherit rust-overlay; };
        pythonTools = pkgs.callPackage ./pythonTools.nix { };
      };
    };
}
