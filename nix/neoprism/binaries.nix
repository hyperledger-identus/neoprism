{ ... }:

{
  perSystem =
    { pkgs, system, ... }:
    let
      rust = pkgs.rustTools.rustMinimal;
      rust-gnu64 = pkgs.rustTools.mkRustCross {
        pkgsCross = pkgs.pkgsCross.gnu64;
        minimal = true;
      };
      rust-aarch64 = pkgs.rustTools.mkRustCross {
        pkgsCross = pkgs.pkgsCross.aarch64-multiplatform;
        minimal = true;
      };
    in
    {
      packages = {
        # native packages
        new-neoprism-ui-assets = pkgs.callPackage ./packages/neoprism-ui-assets.nix { };
        new-neoprism-bin = pkgs.callPackage ./packages/neoprism-bin.nix {
          inherit rust;
          inherit (pkgs.rustTools) cargoLock;
        };

        # cross built binaries
        new-neoprism-bin-x86_64-linux = pkgs.pkgsCross.gnu64.callPackage ./packages/neoprism-bin.nix {
          rust = rust-gnu64;
          inherit (pkgs.rustTools) cargoLock;
        };
        new-neoprism-bin-aarch64-linux =
          pkgs.pkgsCross.aarch64-multiplatform.callPackage ./packages/neoprism-bin.nix
            {
              rust = rust-aarch64;
              inherit (pkgs.rustTools) cargoLock;
            };
      };
    };
}
