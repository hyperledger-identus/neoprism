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
    filter =
      path: _:
      let
        baseName = builtins.baseNameOf path;
      in
      !(
        baseName == "AGENTS.md"
        || baseName == "docker"
        || baseName == "docs"
        || baseName == ".github"
        || baseName == "nix"
        || baseName == "README.md"
        || baseName == "tests"
        || baseName == "tools"
      );
    src = ./../..;
  };
  nativeBuildInputs = with buildPackages; [ protobuf ];
  doCheck = false;
  PROTOC = "${buildPackages.protobuf}/bin/protoc";
}
