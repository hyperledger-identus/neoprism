{ rust-bin, rust-overlay }:

let
  # NeoPRISM still needs nightly for lazybe and error_reporter. Keep the beta
  # beyond sdk-rust's Rust 1.98.1 floor so the git dependency can compile.
  nightlyVersion = "2026-09-02";
  rustOverrideArgs = {
    extensions = [
      "rust-src"
      "rust-analyzer"
      "llvm-tools"
    ];
    targets = [ ];
  };
in
rec {
  rust = mkRust { };

  rustMinimal = mkRust { minimal = true; };

  mkRust =
    {
      minimal ? false,
    }:
    if minimal then
      rust-bin.nightly.${nightlyVersion}.minimal
    else
      rust-bin.nightly.${nightlyVersion}.default.override rustOverrideArgs;

  mkRustCross =
    {
      pkgsCross,
      minimal ? false,
    }:
    let
      rust-bin = rust-overlay.lib.mkRustBin { } pkgsCross.buildPackages;
    in
    if minimal then
      rust-bin.nightly.${nightlyVersion}.minimal
    else
      rust-bin.nightly.${nightlyVersion}.default.override rustOverrideArgs;

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "identus-core-0.0.0" = "sha256-IX5FpDT9WirbaJ3xWhrkAfCGTjipDsdwfXICgmOjXOo=";
      "oura-1.9.4" = "sha256-SaSJOlxnM2+BDg9uE4GUxKync37DJQD+P4VVZA2NO3g=";
    };
  };
}
