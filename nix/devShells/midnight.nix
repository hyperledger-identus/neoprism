{ pkgs }:

let
  rootDir = "$ROOT_DIR";
in
pkgs.mkShell {
  packages =
    with pkgs;
    [
      docker
      git
      hurl
      jq
      # midnight
      pkgsInternal.compactc
    ]
    ++ (builtins.attrValues scripts);

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    export COMPACT_HOME="${pkgs.pkgsInternal.compactc}/bin"
    ${pkgs.cowsay}/bin/cowsay "Working on Midnight blockchain: ${rootDir}"
    cd "${rootDir}"
  '';
}
