{ pkgs }:

let
  version = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile ../../version);
  callPackageRustCross =
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
  mkNeoprismPackages =
    {
      buildFeatures ? [ ],
      extraPackages ? [ ],
    }:
    let
      callPackageCrossDefault =
        if pkgs.system == "x86_64-linux" then
          pkgs.pkgsCross.gnu64.callPackage
        else
          pkgs.pkgsCross.aarch64-multiplatform.callPackage;
    in
    rec {
      # assets
      neoprism-ui-assets = pkgs.callPackage ./neoprism-ui-assets.nix { };

      # neoprism
      neoprism-bin = pkgs.callPackage ./neoprism-bin.nix {
        inherit buildFeatures;
        rust = pkgs.rustTools.rustMinimal;
        inherit (pkgs.rustTools) cargoLock;
      };
      neoprism-bin-x86_64-linux = callPackageRustCross "gnu64" ./neoprism-bin.nix {
        inherit buildFeatures;
        inherit (pkgs.rustTools) cargoLock;
      };
      neoprism-bin-aarch64-linux = callPackageRustCross "aarch64-multiplatform" ./neoprism-bin.nix {
        inherit buildFeatures;
        inherit (pkgs.rustTools) cargoLock;
      };
      neoprism-docker = callPackageCrossDefault ./neoprism-docker.nix {
        inherit version neoprism-ui-assets extraPackages;
        neoprism-bin =
          if pkgs.system == "x86_64-linux" then neoprism-bin-x86_64-linux else neoprism-bin-aarch64-linux;
      };
      neoprism-docker-latest = callPackageCrossDefault ./neoprism-docker.nix {
        inherit neoprism-ui-assets extraPackages;
        neoprism-bin =
          if pkgs.system == "x86_64-linux" then neoprism-bin-x86_64-linux else neoprism-bin-aarch64-linux;
        version = "latest";
      };
      neoprism-docker-linux-amd64 = pkgs.pkgsCross.gnu64.callPackage ./neoprism-docker.nix {
        inherit version neoprism-ui-assets extraPackages;
        neoprism-bin = neoprism-bin-x86_64-linux;
        tagSuffix = "-amd64";
      };
      neoprism-docker-linux-arm64 =
        pkgs.pkgsCross.aarch64-multiplatform.callPackage ./neoprism-docker.nix
          {
            inherit version neoprism-ui-assets extraPackages;
            neoprism-bin = neoprism-bin-aarch64-linux;
            tagSuffix = "-arm64";
          };
    };
  neoprismPackages = mkNeoprismPackages { };

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
