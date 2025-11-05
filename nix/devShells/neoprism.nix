{ pkgs }:

let
  rootDir = "$ROOT_DIR";
  inherit (pkgs.rustTools) rust;
in
pkgs.mkShell {
  packages = with pkgs; [
    # base
    docker
    git
    git-cliff
    just
    less
    ncurses
    nixfmt-rfc-style
    protobuf
    taplo
    which
    # config
    dhall
    dhall-json
    # db
    sqlfluff
    sqlx-cli
    # rust
    cargo-edit
    cargo-expand
    cargo-license
    cargo-udeps
    rust
    # js
    nodejs_20
    tailwindcss_4
    typescript-language-server
    esbuild
  ];

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}"
  '';

  # envs
  RUST_LOG = "info,oura=warn,tower_http::trace=debug";
}
