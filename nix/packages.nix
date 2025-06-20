{ pkgs, rust, ... }:

let
  rustPlatform = pkgs.makeRustPlatform {
    cargo = rust;
    rustc = rust;
  };
in
rec {
  indexer-ui-assets =
    let
      npmDeps = pkgs.buildNpmPackage {
        name = "assets-nodemodules";
        src = ./..;
        npmDepsHash = "sha256-snC2EOnV3200x4fziwcj/1o9KoqSJkTFgJgAh9TWNpE=";
        dontNpmBuild = true;
        installPhase = ''
          cp -r ./node_modules $out
        '';
      };
    in
    pkgs.stdenv.mkDerivation {
      name = "assets";
      src = ./..;
      buildInputs = with pkgs; [ tailwindcss_4 ];
      installPhase = ''
        mkdir -p ./node_modules
        cp -r ${npmDeps}/* ./node_modules
        cd ./service/indexer-node
        mkdir -p $out/assets
        tailwindcss -i ./tailwind.css -o $out/assets/styles.css
      '';
    };

  indexer-bin = rustPlatform.buildRustPackage {
    name = "neoprism";
    src = pkgs.lib.cleanSource ./..;
    cargoLock = (import ./cargo.nix).cargoLock;
    nativeBuildInputs = [ pkgs.protobuf ];
    doCheck = false;
    PROTOC = "${pkgs.protobuf}/bin/protoc";
  };

  indexer-docker = pkgs.dockerTools.buildLayeredImage {
    name = "neoprism";
    tag = "0.1.0-SNAPSHOT";
    created = "now";
    contents = [
      indexer-bin
      indexer-ui-assets
    ];
    config = {
      Env = [ "RUST_LOG=info,oura=warn" ];
      Entrypoint = [ "/bin/indexer-node" ];
      Cmd = [
        "--assets"
        "/assets"
      ];
      WorkingDir = "";
    };
  };
}
