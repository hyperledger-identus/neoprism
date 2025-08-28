{
  compactc,
  esbuild,
  buildNpmPackage,
  importNpmLock,
  nodejs_22,
  writeShellApplication,
  symlinkJoin,
}:

let
  bundle = buildNpmPackage {
    name = "did-midnight-serde";
    src = ../..;
    npmRoot = "./bin/did-midnight-serde";

    npmDeps = importNpmLock { npmRoot = ../../bin/did-midnight-serde; };
    npmConfigHook = importNpmLock.npmConfigHook;

    nativeBuildInputs = [
      compactc
      esbuild
    ];

    buildPhase = ''
      cd ./bin/did-midnight-serde

      # run typecheck
      npm run build
      rm -rf dist

      # actual build
      compactc --skip-zk src/did.compact src/managed/did
      esbuild --bundle \
        --packages=external \
        --platform=node \
        --outdir=dist \
        --format=cjs \
        src/cli.ts
    '';

    installPhase = ''
      mkdir -p $out/dist
      mkdir -p $out/node_modules
      cp -r dist/* $out/dist
      cp -r node_modules/* $out/node_modules
    '';
  };
  wrapper = writeShellApplication {
    name = "did-midnight-serde";
    runtimeInputs = [ nodejs_22 ];
    text = ''
      export NODE_PATH=${bundle}/node_modules
      node ${bundle}/dist/cli.js "$@"
    '';
  };
in
symlinkJoin {
  name = "did-midnight-serde";
  paths = [
    bundle
    wrapper
  ];
}
