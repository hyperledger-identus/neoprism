{
  lib,
  rustUtils,
  makeRustPlatform,
  protobuf,
  sqlfluff,
  deadnix
}:

let
  rust = rustUtils.rust;
  rustPlatform = makeRustPlatform {
    cargo = rust;
    rustc = rust;
  };
in
rustPlatform.buildRustPackage {
  name = "neoprism-checks";
  src = lib.cleanSource ./../..;
  cargoLock = rustUtils.cargoLock;
  nativeBuildInputs = [
    protobuf
    sqlfluff
    deadnix
  ];
  buildPhase = "cargo b --all-features --all-targets";
  checkPhase = ''
    deadnix -f
    sqlfluff lint --dialect postgres ./lib/indexer-storage/migrations
    cargo fmt --check
    cargo clippy --all-features --all-targets -- -D warnings
    cargo test --all-features

    # check individual feature if properly gated
    echo "checking feature gate for identus-apollo"
    cargo clippy -p identus-apollo --all-targets -- -D warnings
    cargo clippy -p identus-apollo --all-targets --features base64 -- -D warnings
    cargo clippy -p identus-apollo --all-targets --features ed25519 -- -D warnings
    cargo clippy -p identus-apollo --all-targets --features hash -- -D warnings
    cargo clippy -p identus-apollo --all-targets --features hex -- -D warnings
    cargo clippy -p identus-apollo --all-targets --features jwk -- -D warnings
    cargo clippy -p identus-apollo --all-targets --features openapi -- -D warnings
    cargo clippy -p identus-apollo --all-targets --features secp256k1 -- -D warnings
    cargo clippy -p identus-apollo --all-targets --features serde -- -D warnings
    cargo clippy -p identus-apollo --all-targets --features x25519 -- -D warnings

    echo "checking feature gate for identus-did-core"
    cargo clippy -p identus-did-core --all-targets -- -D warnings
    cargo clippy -p identus-did-core --all-targets --features openapi -- -D warnings

    echo "checking feature gate for identus-did-prism-indexer"
    cargo clippy -p identus-did-prism-indexer --all-targets -- -D warnings
    cargo clippy -p identus-did-prism-indexer --all-targets --features oura -- -D warnings
    cargo clippy -p identus-did-prism-indexer --all-targets --features dbsync -- -D warnings
  '';
  installPhase = "touch $out";

  PROTOC = "${protobuf}/bin/protoc";
}
