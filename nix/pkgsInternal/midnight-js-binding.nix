{
  stdenv,
  deno,
  writeText,
  runCommand,
  compactc
}:

let
  denoCache = stdenv.mkDerivation {
    name = "deno-vendor";
    src = ./../../lib/did-midnight/binding;
    buildInputs = [ deno ];
    outputHash = "sha256-GMOLT/K4E+ILrAHmnRGcWBPFaH4LLiqETCZ9efUUakA=";
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
    buildPhase = ''
      export DENO_DIR=$out
      mkdir -p $DENO_DIR
      deno bundle --vendor index.ts > /dev/null
    '';
    installPhase = "true";
  };
  binding = stdenv.mkDerivation {
    name = "midnight-js-binding";
    src = ./../../lib/did-midnight/binding;
    buildInputs = [ deno compactc ];
    buildPhase = ''
      export DENO_DIR=${denoCache}

      mkdir -p managed
      compactc --skip-zk did.compact managed/

      mkdir -p dist
      cp -r $DENO_DIR/npm/registry.npmjs.org/@midnight-ntwrk/ledger/4.0.0/midnight_ledger_wasm_bg.wasm ./dist/midnight_ledger_wasm_bg.wasm
      deno bundle --vendor index.ts > ./dist/bundle.js
    '';
    installPhase = ''
      mkdir -p $out
      cp -r dist/* $out/
    '';
  };
  patchedBundleJs =
    let
      content = builtins.readFile "${binding}/bundle.js";
      p1 =
        builtins.replaceStrings
          [
            ''
              import { readFileSync } from "node:fs";
              import { join, dirname } from "node:path";
              import { fileURLToPath } from "node:url";
            ''
          ]
          [ "" ]
          content;
      p2 =
        builtins.replaceStrings
          [
            ''
              var __filename = fileURLToPath(import.meta.url);
              var __dirname = dirname(__filename);
              var wasmPath = join(__dirname, "midnight_ledger_wasm_bg.wasm");
              var bytes = readFileSync(wasmPath);
            ''
          ]
          [
            ''
              const scriptDir = new URL(".", import.meta.url).pathname;
              var bytes = await Deno.readFile(`''${scriptDir}/midnight_ledger_wasm_bg.wasm`);
            ''
          ]
          p1;
    in
    writeText "patched-bundle-js" p2;
in
runCommand "patched-midnight-js-binding" { } ''
  mkdir $out
  cp -r ${binding}/* $out/
  cp ${patchedBundleJs} $out/bundle-pure-deno.js
''
