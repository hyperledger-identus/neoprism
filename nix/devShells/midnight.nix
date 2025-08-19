{ pkgs }:

let
  rootDir = "$ROOT_DIR";
  scripts = {
    midnightUp = pkgs.writeShellApplication {
      name = "midnightUp";
      text = ''
        cd "${rootDir}"
        docker run -d --rm \
          --name midnight-proof-server \
          -p 6300:6300 \
          midnightnetwork/proof-server:4.0.0 \
          midnight-proof-server --network testnet
      '';
    };
    midnightDown = pkgs.writeShellApplication {
      name = "midnightDown";
      text = ''
        cd "${rootDir}"
        docker stop midnight-proof-server
      '';
    };
  };
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
