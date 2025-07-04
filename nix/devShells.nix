{ pkgs, rust, ... }:

{
  default =
    let
      rootDir = "$ROOT_DIR";
      localDb = {
        port = 5432;
        username = "postgres";
        password = "postgres";
        dbName = "postgres";
      };
      scripts = rec {
        format = pkgs.writeShellScriptBin "format" ''
          cd ${rootDir}
          find . | grep '\.nix$' | xargs -I _ bash -c "echo running nixfmt on _ && ${pkgs.nixfmt-rfc-style}/bin/nixfmt _"
          find . | grep '\.toml$' | xargs -I _ bash -c "echo running taplo on _ && ${pkgs.taplo}/bin/taplo format _"

          ${rust}/bin/cargo fmt

          cd ${rootDir}/lib/indexer-storage/migrations
          ${pkgs.sqlfluff}/bin/sqlfluff fix .
          ${pkgs.sqlfluff}/bin/sqlfluff lint .
        '';

        buildAssets = pkgs.writeShellScriptBin "buildAssets" ''
          cd ${rootDir}/service/indexer-node
          ${pkgs.tailwindcss_4}/bin/tailwindcss -i tailwind.css -o ./assets/styles.css
        '';

        build = pkgs.writeShellScriptBin "build" ''
          cd ${rootDir}
          ${buildAssets}/bin/buildAssets
          ${rust}/bin/cargo build --all-features
        '';

        clean = pkgs.writeShellScriptBin "clean" ''
          cd ${rootDir}
          ${rust}/bin/cargo clean
        '';

        dbUp = pkgs.writeShellScriptBin "dbUp" ''
          ${pkgs.docker}/bin/docker run \
            -d --rm \
            --name prism-db \
            -e POSTGRES_DB=${localDb.dbName} \
            -e POSTGRES_USER=${localDb.username} \
            -e POSTGRES_PASSWORD=${localDb.password} \
            -p ${toString localDb.port}:5432 postgres:16
        '';

        dbDown = pkgs.writeShellScriptBin "dbDown" ''
          ${pkgs.docker}/bin/docker stop prism-db
        '';

        pgDump = pkgs.writeShellScriptBin "pgDump" ''
          export PGPASSWORD=${localDb.password}
          ${pkgs.postgresql_16}/bin/pg_dump -h localhost -p ${toString localDb.port} -U ${localDb.username} -w -d ${localDb.dbName} -Fc > ${rootDir}/postgres.dump
        '';

        pgRestore = pkgs.writeShellScriptBin "pgRestore" ''
          export PGPASSWORD=${localDb.password}
          ${pkgs.postgresql_16}/bin/pg_restore -h localhost -p ${toString localDb.port} -U ${localDb.username} -w -d ${localDb.dbName} ${rootDir}/postgres.dump
        '';

        runNode = pkgs.writeShellScriptBin "runNode" ''
          cd ${rootDir}
          ${buildAssets}/bin/buildAssets
          ${rust}/bin/cargo run --bin indexer-node -- --db postgres://${localDb.username}:${localDb.password}@localhost:${toString localDb.port}/${localDb.dbName} "$@"
        '';
      };
    in
    pkgs.mkShell {
      packages =
        with pkgs;
        [
          # base
          docker
          git
          less
          ncurses
          protobuf
          watchexec
          which
          # db
          sqlfluff
          sqlx-cli
          # rust
          cargo-edit
          cargo-expand
          cargo-license
          cargo-udeps
          protobuf
          rust
          wasm-pack
          # node
          nodejs_20
          tailwindcss_4
        ]
        ++ (builtins.attrValues scripts);

      shellHook = ''
        export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
        ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
        cd ${rootDir}
      '';

      # envs
      RUST_LOG = "info,oura=warn,tower_http::trace=debug";
    };
}
