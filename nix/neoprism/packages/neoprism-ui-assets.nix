{
  stdenv,
  tailwindcss_4,
  bun,
}:

let
  bunDepsHash = "sha256-RuHGcTAg1QqZQ2nSJ4SjQXGow/qz3k8eFedqaQkYn0s=";

  bunDeps = stdenv.mkDerivation {
    name = "neoprism-ui-deps";
    nativeBuildInputs = [ bun ];

    src = ./../../..;

    outputHash = bunDepsHash;
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";

    installPhase = ''
      export HOME=$TMPDIR
      bun install --frozen-lockfile
      cp -r ./node_modules $out
    '';
  };
in
stdenv.mkDerivation {
  name = "neoprism-ui-assets";
  src = ./../../..;
  buildInputs = [ tailwindcss_4 ];

  installPhase = ''
    cp -r ${bunDeps} ./node_modules
    cd ./bin/neoprism-node
    mkdir -p $out/assets
    tailwindcss -i ./tailwind.css -o $out/assets/styles.css
  '';
}
