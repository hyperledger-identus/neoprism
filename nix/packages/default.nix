{ pkgs }:

let
  version = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile ../../version);
  callPackageRustCross =
    targetSystem: path: overrides:
    pkgs.pkgsCross."${targetSystem}".callPackage path (
      {
        rust = pkgs.rustUtils.mkRustCross {
          pkgsCross = pkgs.pkgsCross."${targetSystem}";
          minimal = true;
        };
      }
      // overrides
    );
in
rec {
  # native
  ui-assets = pkgs.callPackage ./ui-assets.nix { };
  neoprism-bin = pkgs.callPackage ./neoprism-bin.nix {
    rust = pkgs.rustUtils.rustMinimal;
    cargoLock = pkgs.rustUtils.cargoLock;
  };
  neoprism-docker = pkgs.callPackage ./neoprism-docker.nix {
    inherit version neoprism-bin ui-assets;
  };

  # cross x86
  neoprism-bin-x86_64-linux = callPackageRustCross "gnu64" ./neoprism-bin.nix {
    cargoLock = pkgs.rustUtils.cargoLock;
  };
  neoprism-docker-linux-amd64 = pkgs.pkgsCross.gnu64.callPackage ./neoprism-docker.nix {
    inherit version ui-assets;
    neoprism-bin = neoprism-bin-x86_64-linux;
    tagSuffix = "-amd64";
  };

  # cross aarch64
  neoprism-bin-aarch64-linux = callPackageRustCross "aarch64-multiplatform" ./neoprism-bin.nix {
    cargoLock = pkgs.rustUtils.cargoLock;
  };
  neoprism-docker-linux-arm64 =
    pkgs.pkgsCross.aarch64-multiplatform.callPackage ./neoprism-docker.nix
      {
        inherit version ui-assets;
        neoprism-bin = neoprism-bin-aarch64-linux;
        tagSuffix = "-arm64";
      };
}
