{ pkgs }:

let
  rootDir = "$ROOT_DIR";
in
pkgs.mkShell {
  packages = with pkgs; [
    nix
    jq
    hurl
    cardano-node
    cardano-cli
    cardano-wallet
    cardano-testnet
    cardano-db-sync
    cardano-submit-api
  ];

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}"
  '';

  CARDANO_CLI = "${pkgs.cardano-cli}/bin/cardano-cli";
  CARDANO_NODE = "${pkgs.cardano-node}/bin/cardano-node";
  PRISM_HOME = ".";
}
