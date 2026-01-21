{ ... }:
{
  perSystem =
    { ... }:
    {
      _module.args.neoprismLib = {
        version = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile ../version);
      };
    };
}
