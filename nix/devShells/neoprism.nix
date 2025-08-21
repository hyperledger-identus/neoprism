{ pkgs, buildConfig }:

let
  rootDir = "$ROOT_DIR";
  rust = pkgs.rustUtils.rust;
  localDb = {
    port = 5432;
    username = "postgres";
    password = "postgres";
    dbName = "postgres";
  };
  scripts = rec {
    inherit buildConfig;

    format = pkgs.writeShellApplication {
      name = "format";
      runtimeInputs = with pkgs; [
        nixfmt-rfc-style
        taplo
      ];
      text = ''
        cd "${rootDir}"
        find . | grep '\.nix$' | xargs -I _ bash -c "echo running nixfmt on _ && nixfmt _"
        find . | grep '\.toml$' | xargs -I _ bash -c "echo running taplo on _ && taplo format _"
        find . | grep '\.dhall$' | xargs -I _ bash -c "echo running dhall format on _ && dhall format _"
        cargo fmt

        cd "${rootDir}/lib/node-storage/migrations"
        sqlfluff fix .
        sqlfluff lint .
      '';
    };

    buildAssets = pkgs.writeShellApplication {
      name = "buildAssets";
      text = ''
        cd "${rootDir}/bin/neoprism-node"
        tailwindcss -i tailwind.css -o ./assets/styles.css
      '';
    };

    build = pkgs.writeShellApplication {
      name = "build";
      text = ''
        cd "${rootDir}"
        ${buildAssets}/bin/buildAssets
        cargo build --all-features
      '';
    };

    clean = pkgs.writeShellApplication {
      name = "clean";
      text = ''
        cd "${rootDir}"
        cargo clean
      '';
    };

    dbUp = pkgs.writeShellApplication {
      name = "dbUp";
      text = ''
        docker run \
          -d --rm \
          --name prism-db \
          -e POSTGRES_DB=${localDb.dbName} \
          -e POSTGRES_USER=${localDb.username} \
          -e POSTGRES_PASSWORD=${localDb.password} \
          -p ${toString localDb.port}:5432 postgres:16
      '';
    };

    dbDown = pkgs.writeShellApplication {
      name = "dbDown";
      text = ''
        docker stop prism-db
      '';
    };

    pgDump = pkgs.writeShellApplication {
      name = "pgDump";
      runtimeInputs = with pkgs; [ postgresql_16 ];
      text = ''
        cd "${rootDir}"
        export PGPASSWORD=${localDb.password}
        pg_dump -h localhost -p ${toString localDb.port} -U ${localDb.username} -w -d ${localDb.dbName} -Fc > postgres.dump
      '';
    };

    pgRestore = pkgs.writeShellApplication {
      name = "pgRestore";
      runtimeInputs = with pkgs; [ postgresql_16 ];
      text = ''
        cd "${rootDir}"
        export PGPASSWORD=${localDb.password}
        pg_restore -h localhost -p ${toString localDb.port} -U ${localDb.username} -w -d ${localDb.dbName} postgres.dump
      '';
    };

    runNode = pkgs.writeShellApplication {
      name = "runNode";
      text = ''
        cd "${rootDir}"
        ${buildAssets}/bin/buildAssets
        export NPRISM_DB_URL="postgres://${localDb.username}:${localDb.password}@localhost:${toString localDb.port}/${localDb.dbName}"
        cargo run --bin neoprism-node -- "$@"
      '';
    };
  };
in
pkgs.mkShell {
  packages =
    with pkgs;
    [
      # base
      docker
      git
      git-cliff
      less
      ncurses
      protobuf
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
      # node
      nodejs_20
      tailwindcss_4
      deno
      # midnight
      pkgsInternal.compactc
    ]
    ++ (builtins.attrValues scripts);

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}"
  '';

  # envs
  RUST_LOG = "info,oura=warn,tower_http::trace=debug";
}
