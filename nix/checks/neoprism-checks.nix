{
  lib,
  rustTools,
  makeRustPlatform,
  protobuf,
  sqlfluff,
  deadnix,
  openssl,
  pkg-config,
}:

let
  inherit (rustTools) rust;
  rustPlatform = makeRustPlatform {
    cargo = rust;
    rustc = rust;
  };
in
rustPlatform.buildRustPackage {
  name = "neoprism-checks";
  src = lib.cleanSource ./../..;
  inherit (rustTools) cargoLock;
  nativeBuildInputs = [
    protobuf
    sqlfluff
    deadnix
    pkg-config
  ];
  buildInputs = [
    openssl
  ];
  buildPhase = "cargo b --all-features --all-targets";
  checkPhase = ''
    deadnix -f
    sqlfluff lint --dialect postgres ./lib/node-storage/migrations/postgres
    cargo fmt --check
    cargo test
    cargo clippy --all-targets -- -D warnings

    cargo test --all-features
    cargo clippy --all-targets --all-features -- -D warnings

    # check individual feature if properly gated
    echo "checking feature gate for identus-apollo"
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
    cargo clippy -p identus-did-core --all-targets --features openapi -- -D warnings
    cargo clippy -p identus-did-core --all-targets --features ts-types -- -D warnings

    echo "checking feature gate for identus-did-resolver-http"
    cargo clippy -p identus-did-resolver-http --all-targets --features openapi -- -D warnings

    echo "checking feature gate for identus-did-prism"
    cargo clippy -p identus-did-prism --all-targets --features openapi -- -D warnings

    echo "checking feature gate for identus-did-prism-indexer"
    cargo clippy -p identus-did-prism-indexer --all-targets --features oura -- -D warnings
    cargo clippy -p identus-did-prism-indexer --all-targets --features dbsync -- -D warnings
    cargo clippy -p identus-did-prism-indexer --all-targets --features blockfrost -- -D warnings

    echo "checking feature gate for identus-did-prism-ledger"
    cargo clippy -p identus-did-prism-ledger --all-targets --features in-memory -- -D warnings

    echo "checking feature gate for identus-did-prism-submitter"
    cargo clippy -p identus-did-prism-submitter --all-targets --features cardano-wallet -- -D warnings

    echo "checking feature gate for node-storage"
    cargo clippy -p node-storage --all-targets --features sqlite-storage -- -D warnings
  '';
  installPhase = "touch $out";

  PROTOC = "${protobuf}/bin/protoc";
}
