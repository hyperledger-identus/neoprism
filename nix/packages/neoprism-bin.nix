{
  lib,
  makeRustPlatform,
  rust,
  cargoLock,
  buildPackages,
  buildFeatures ? [ ],
}:

let
  rustPlatform = makeRustPlatform {
    cargo = rust;
    rustc = rust;
  };
in
rustPlatform.buildRustPackage {
  inherit cargoLock buildFeatures;
  name = "neoprism";
  src = lib.cleanSourceWith {
    filter = (
      path: _:
      let
        baseName = builtins.baseNameOf path;
      in
      !(
        baseName == "docs"
        || baseName == "docker"
        || baseName == ".github"
        || baseName == "tests"
        || baseName == "README.md"
        || baseName == "AGENTS.md"
      )
    );
    src = ./../..;
  };
  nativeBuildInputs = with buildPackages; [ protobuf ];
  doCheck = false;
  PROTOC = "${buildPackages.protobuf}/bin/protoc";
}
