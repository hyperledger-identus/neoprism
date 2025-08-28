{ rust-bin, rust-overlay }:

let
  nightlyVersion = "2025-07-08";
  rustOverrideArgs = {
    extensions = [
      "rust-src"
      "rust-analyzer"
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
      "oura-1.9.4" = "sha256-SaSJOlxnM2+BDg9uE4GUxKync37DJQD+P4VVZA2NO3g=";
    };
  };
}
