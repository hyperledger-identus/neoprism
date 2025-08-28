{ pkgs }:

pkgs.mkShell {
  packages = with pkgs; [
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
  '';
}
