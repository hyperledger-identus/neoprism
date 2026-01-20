{ ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      inherit (pkgs.rustTools) rust;
      inherit (pkgs.pythonTools) pythonEnv;
    in
    {
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          # base
          cowsay
          docker
          docker-compose
          git
          git-cliff
          hurl
          jq
          just
          less
          ncurses
          nix
          nixfmt-rfc-style
          pkg-config
          protobuf
          taplo
          which
          # python
          pythonEnv
          pyright
          ruff
          # db
          postgresql_16
          sqlfluff
          sqlite
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
          ${pkgs.cowsay}/bin/cowsay "Working on project root directory: $ROOT_DIR"
          cd "$ROOT_DIR"
        '';

        # envs
        RUST_LOG = "info,oura=warn,tower_http::trace=debug";

        JAVA_HOME = "${pkgs.jdk}/lib/openjdk";
        SBT_OPTS = "-Xmx4G";
        SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
        LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib}/lib/"; # required by scalapb
      };
    };
}
