{
  self,
  config,
  lib,
  ...
}:
{
  perSystem =
    { self', ... }:
    {
      _module.args.neoprismLib = {
        version = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile ../version);
      };
    };
}
