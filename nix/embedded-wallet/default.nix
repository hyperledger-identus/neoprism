{ ... }:

let
  # Fix for cross-compiled bun: remove postPhases that causes build failures
  # The nixpkgs bun package has a custom postPatchelf phase that fails when cross-compiling
  # because the cross-stdenv tries to run it as a command instead of a function
  fixCrossBun =
    bun:
    bun.overrideAttrs (_: {
      postPhases = [ ];
      postPatchelf = null;
    });
in
{
  perSystem =
    { pkgs, neoprismLib, ... }:
    let
      inherit (neoprismLib) version;
    in
    {
      packages = {
        # Native build (uses current platform's bun runtime)
        embedded-wallet = pkgs.callPackage ./package.nix { inherit version; };

        # Cross-compiled builds for Linux targets
        # Uses nixpkgs bun packages for the target platform
        embedded-wallet-x86_64-linux = pkgs.callPackage ./package.nix {
          inherit version;
          bunTarget = "bun-linux-x64";
          pnameSuffix = "x86_64-linux";
          targetBun = fixCrossBun pkgs.pkgsCross.gnu64.bun;
        };

        embedded-wallet-aarch64-linux = pkgs.callPackage ./package.nix {
          inherit version;
          bunTarget = "bun-linux-arm64";
          pnameSuffix = "aarch64-linux";
          targetBun = fixCrossBun pkgs.pkgsCross.aarch64-multiplatform.bun;
        };
      };
    };
}
