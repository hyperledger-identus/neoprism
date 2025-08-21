{ stdenv, deno, ... }:

let
  denoCache = stdenv.mkDerivation {
    name = "deno-vendor";
    src = ./../../lib/did-midnight/binding;
    buildInputs = [ deno ];
    outputHash = "sha256-p3Cmn7XMraXr5xh/d662phlgJQH+1fyLO8m4P73MBSY=";
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
    buildPhase = ''
      export DENO_DIR=$out
      mkdir -p $DENO_DIR
      deno bundle --vendor index.ts > /dev/null
    '';
    installPhase = "true";
  };
in
# /home/pat/Desktop/workspace/personal/neoprism/result/npm/registry.npmjs.org/@midnight-ntwrk/ledger/4.0.0/midnight_ledger_wasm_bg.wasm
stdenv.mkDerivation {
  name = "midnight-js-binding";
  src = ./../../lib/did-midnight/binding;
  buildInputs = [ deno ];
  buildPhase = ''
    export DENO_DIR=${denoCache}
    mkdir -p dist
    cp -r $DENO_DIR/npm/registry.npmjs.org/@midnight-ntwrk/ledger/4.0.0/midnight_ledger_wasm_bg.wasm ./dist/midnight_ledger_wasm_bg.wasm
    deno bundle --vendor index.ts > ./dist/bundle.js
  '';
  installPhase = ''
    mkdir -p $out
    cp -r dist/* $out/
  '';
}
