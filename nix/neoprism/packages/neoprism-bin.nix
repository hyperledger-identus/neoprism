{
  lib,
  makeRustPlatform,
  rust,
  cargoLock,
  stdenv,
  buildPackages,
  openssl,
  pkg-config,
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
    src = ./../../..;
  };
  nativeBuildInputs =
    with buildPackages;
    [
      protobuf
      pkg-config
    ]
    ++ lib.optionals stdenv.buildPlatform.isDarwin [
      buildPackages.libiconv
      buildPackages.apple-sdk
    ];
  buildInputs = [
    openssl
  ];
  doCheck = false;
  PROTOC = "${buildPackages.protobuf}/bin/protoc";
}
