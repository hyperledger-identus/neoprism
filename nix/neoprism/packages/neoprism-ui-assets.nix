{
  buildNpmPackage,
  stdenv,
  tailwindcss_4,
}:

let
  npmDeps = buildNpmPackage {
    name = "assets-nodemodules";
    src = ./../../..;
    npmDepsHash = "sha256-snC2EOnV3200x4fziwcj/1o9KoqSJkTFgJgAh9TWNpE=";
    dontNpmBuild = true;
    installPhase = ''
      cp -r ./node_modules $out
    '';
  };
in
stdenv.mkDerivation {
  name = "neoprism-ui-assets";
  src = ./../../..;
  buildInputs = [ tailwindcss_4 ];
  installPhase = ''
    mkdir -p ./node_modules
    cp -r ${npmDeps}/* ./node_modules
    cd ./bin/neoprism-node
    mkdir -p $out/assets
    tailwindcss -i ./tailwind.css -o $out/assets/styles.css
  '';
}
