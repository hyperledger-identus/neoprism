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
  mkNeoprismPackages =
    {
      buildFeatures ? [ ],
      extraPackages ? [ ],
    }:
    rec {
      # assets
      neoprism-ui-assets = pkgs.callPackage ./neoprism-ui-assets.nix { };

      # neoprism
      neoprism-bin = pkgs.callPackage ./neoprism-bin.nix {
        inherit buildFeatures;
        rust = pkgs.rustUtils.rustMinimal;
        cargoLock = pkgs.rustUtils.cargoLock;
      };
      neoprism-bin-x86_64-linux = callPackageRustCross "gnu64" ./neoprism-bin.nix {
        inherit buildFeatures;
        cargoLock = pkgs.rustUtils.cargoLock;
      };
      neoprism-bin-aarch64-linux = callPackageRustCross "aarch64-multiplatform" ./neoprism-bin.nix {
        inherit buildFeatures;
        cargoLock = pkgs.rustUtils.cargoLock;
      };
      neoprism-docker = pkgs.callPackage ./neoprism-docker.nix {
        inherit
          version
          neoprism-bin
          neoprism-ui-assets
          extraPackages
          ;
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
  neoprismMidnightPackages =
    let
      outputs = mkNeoprismPackages {
        buildFeatures = [ "midnight" ];
        extraPackages = [ pkgs.pkgsInternal.did-midnight-serde ];
      };
      renameOutputs = name: value: {
        inherit value;
        name =
          builtins.replaceStrings
            [ "neoprism-bin" "neoprism-docker" ]
            [ "neoprism-midnight-bin" "neoprism-midnight-docker" ]
            name;
      };
    in
    pkgs.lib.attrsets.mapAttrs' renameOutputs outputs;
in
{
  # docs-site
  docs-site = pkgs.callPackage ./docs-site.nix {
    inherit version;
    neoprism-bin = neoprismPackages.neoprism-bin;
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

  # misc
  scala-did-docker = pkgs.callPackage ./scala-did-docker.nix { };
  did-midnight-serde = pkgs.pkgsInternal.did-midnight-serde;
}
// neoprismPackages
// neoprismMidnightPackages
