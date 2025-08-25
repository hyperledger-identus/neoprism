{
  compactc,
  esbuild,
  buildNpmPackage,
  importNpmLock,
  nodejs_20,
  writeShellApplication,
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
      compactc --skip-zk src/did.compact src/managed/did
      esbuild --bundle \
        --packages=external \
        --platform=node \
        --outdir=dist \
        --format=cjs \
        src/index.ts
    '';

    installPhase = ''
      ls -aoh dist
      mkdir -p $out/dist
      mkdir -p $out/node_modules
      cp -r dist/* $out/dist
      cp -r node_modules/* $out/node_modules
    '';
  };
in
writeShellApplication {
  name = "did-midnight-serde";
  runtimeInputs = [ nodejs_20 ];
  text = ''
    export NODE_PATH=${bundle}/node_modules
    node ${bundle}/dist/index.js "$@"
  '';
}
