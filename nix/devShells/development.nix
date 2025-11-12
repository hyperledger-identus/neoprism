{ pkgs }:

let
  rootDir = "$ROOT_DIR";
  inherit (pkgs.rustTools) rust;
  pythonEnv = pkgs.python313.withPackages (
    ps: with ps; [
      pydantic
      pyyaml
    ]
  );
in
pkgs.mkShell {
  packages = with pkgs; [
    # base
    docker
    docker-compose
    git
    git-cliff
    hurl
    just
    less
    ncurses
    nix
    nixfmt-rfc-style
    protobuf
    taplo
    which
    # python
    pythonEnv
    pyright
    ruff
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
    # scala
    jdk
    metals
    sbt
  ];

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}"
  '';

  # envs
  RUST_LOG = "info,oura=warn,tower_http::trace=debug";

  JAVA_HOME = "${pkgs.jdk}/lib/openjdk";
  SBT_OPTS = "-Xmx4G";
  SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
  LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib}/lib/"; # required by scalapb
}
