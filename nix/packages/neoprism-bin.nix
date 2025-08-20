{
  lib,
  makeRustPlatform,
  rust,
  cargoLock,
  buildPackages,
}:

let
  rustPlatform = makeRustPlatform {
    cargo = rust;
    rustc = rust;
  };
in
rustPlatform.buildRustPackage {
  inherit cargoLock;
  name = "neoprism";
  src = lib.cleanSourceWith {
    filter = (
      path: type:
      let
        baseName = builtins.baseNameOf path;
      in
      !(
        baseName == "docs"
        || baseName == "docker"
        || baseName == ".github"
        || baseName == "tests"
        || baseName == "README.md"
      )
    );
    src = ./../..;
  };
  nativeBuildInputs = with buildPackages; [ protobuf ];
  doCheck = false;
  PROTOC = "${buildPackages.protobuf}/bin/protoc";
}
