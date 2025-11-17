{ pkgs }:

let
  hostSystem = pkgs.stdenv.hostPlatform.system;
  version = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile ../../version);
  callPackageCrossWithRust =
    targetSystem: path: overrides:
    pkgs.pkgsCross."${targetSystem}".callPackage path (
      {
        rust = pkgs.rustTools.mkRustCross {
          pkgsCross = pkgs.pkgsCross."${targetSystem}";
          minimal = true;
        };
      }
      // overrides
    );
  nativePackages = {
    neoprism-ui-assets = pkgs.callPackage ./neoprism-ui-assets.nix { };
    neoprism-bin = pkgs.callPackage ./neoprism-bin.nix {
      rust = pkgs.rustTools.rustMinimal;
      inherit (pkgs.rustTools) cargoLock;
    };
    neoprism-bin-x86_64-linux = callPackageCrossWithRust "gnu64" ./neoprism-bin.nix {
      inherit (pkgs.rustTools) cargoLock;
    };
    neoprism-bin-aarch64-linux = callPackageCrossWithRust "aarch64-multiplatform" ./neoprism-bin.nix {
      inherit (pkgs.rustTools) cargoLock;
    };
  };
  dockerPackages =
    let
      # Docker images target Linux, regardless of host platform
      dockerCrossPlatformConfig = {
        x86_64-linux = {
          callPackage = pkgs.pkgsCross.gnu64.callPackage;
          neoprism-bin = nativePackages.neoprism-bin-x86_64-linux;
        };
        aarch64-darwin = {
          # macOS builds Linux ARM64 containers
          callPackage = pkgs.pkgsCross.aarch64-multiplatform.callPackage;
          neoprism-bin = nativePackages.neoprism-bin-aarch64-linux;
        };
      };
    in
    {
      neoprism-docker = dockerCrossPlatformConfig.${hostSystem}.callPackage ./neoprism-docker.nix {
        inherit version;
        inherit (nativePackages) neoprism-ui-assets;
        inherit (dockerCrossPlatformConfig.${hostSystem}) neoprism-bin;
      };
      neoprism-docker-latest = dockerCrossPlatformConfig.${hostSystem}.callPackage ./neoprism-docker.nix {
        inherit (nativePackages) neoprism-ui-assets;
        inherit (dockerCrossPlatformConfig.${hostSystem}) neoprism-bin;
        version = "latest";
      };
      neoprism-docker-linux-amd64 = pkgs.pkgsCross.gnu64.callPackage ./neoprism-docker.nix {
        inherit version;
        inherit (nativePackages) neoprism-ui-assets;
        neoprism-bin = nativePackages.neoprism-bin-x86_64-linux;
        tagSuffix = "-amd64";
      };
      neoprism-docker-linux-arm64 =
        pkgs.pkgsCross.aarch64-multiplatform.callPackage ./neoprism-docker.nix
          {
            inherit version;
            inherit (nativePackages) neoprism-ui-assets;
            neoprism-bin = nativePackages.neoprism-bin-aarch64-linux;
            tagSuffix = "-arm64";
          };
    };
  neoprismPackages = nativePackages // dockerPackages;
in
{
  # docs-site
  docs-site = pkgs.callPackage ./docs-site.nix {
    inherit version;
    inherit (neoprismPackages) neoprism-bin;
  };

  # cardano-testnet
  cardano-testnet-docker = pkgs.callPackage ./cardano-testnet-docker.nix { };
  cardano-testnet-docker-linux-amd64 = pkgs.pkgsCross.gnu64.callPackage ./cardano-testnet-docker.nix {
    tagSuffix = "-amd64";
  };
  cardano-testnet-docker-linux-arm64 =
    pkgs.pkgsCross.aarch64-multiplatform.callPackage ./cardano-testnet-docker.nix
      {
        tagSuffix = "-arm64";
      };
}
// neoprismPackages
