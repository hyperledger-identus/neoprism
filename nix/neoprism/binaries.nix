{ ... }:

{
  perSystem =
    { pkgs, neoprismLib, ... }:
    let
      rust = neoprismLib.rustTools.rustMinimal;
      rust-gnu64 = neoprismLib.rustTools.mkRustCross {
        pkgsCross = pkgs.pkgsCross.gnu64;
        minimal = true;
      };
      rust-aarch64 = neoprismLib.rustTools.mkRustCross {
        pkgsCross = pkgs.pkgsCross.aarch64-multiplatform;
        minimal = true;
      };
    in
    {
      packages = {
        # native packages
        neoprism-ui-assets = pkgs.callPackage ./packages/neoprism-ui-assets.nix { };
        neoprism-bin = pkgs.callPackage ./packages/neoprism-bin.nix {
          inherit rust;
          inherit (neoprismLib.rustTools) cargoLock;
        };

        # cross built binaries
        neoprism-bin-x86_64-linux = pkgs.pkgsCross.gnu64.callPackage ./packages/neoprism-bin.nix {
          rust = rust-gnu64;
          inherit (neoprismLib.rustTools) cargoLock;
        };
        neoprism-bin-aarch64-linux =
          pkgs.pkgsCross.aarch64-multiplatform.callPackage ./packages/neoprism-bin.nix
            {
              rust = rust-aarch64;
              inherit (neoprismLib.rustTools) cargoLock;
            };
      };
    };
}
