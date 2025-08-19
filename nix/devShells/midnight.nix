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
      python313
      # midnight
      pkgsInternal.compactc
      nodejs_22
      typescript
      nodePackages.typescript-language-server
    ];

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    export COMPACT_HOME="${pkgs.pkgsInternal.compactc}/bin"
    ${pkgs.cowsay}/bin/cowsay "Working on Midnight blockchain: ${rootDir}"
    cd "${rootDir}/../example-counter"
  '';
}
