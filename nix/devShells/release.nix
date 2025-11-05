{ pkgs  }:

let
  rootDir = "$ROOT_DIR";
  inherit (pkgs.rustTools) rust;
  scripts = rec {
    bumpVersion = pkgs.writeShellApplication {
      name = "bumpVersion";
      runtimeInputs = with pkgs; [
        git-cliff
        jq
      ];
      text = ''
        cd "${rootDir}"
        NEW_VERSION=$(git-cliff --bump --context | jq -r .[0].version | sed s/^v//)
        ${setVersion}/bin/setVersion "$NEW_VERSION"
      '';
    };

    setVersion = pkgs.writeShellApplication {
      name = "setVersion";
      runtimeInputs = with pkgs; [
        rust
        cargo-edit
      ];
      text = ''
        cd "${rootDir}"
        NEW_VERSION=$1
        echo "Setting new version to $NEW_VERSION"
        echo "$NEW_VERSION" > version
        cargo set-version "$NEW_VERSION"
        git-cliff -t "$NEW_VERSION" > CHANGELOG.md
      '';
    };

    releaseTestnetImage = pkgs.writeShellApplication {
      name = "releaseTestnetImage";
      runtimeInputs = with pkgs; [
        nix
        docker
      ];
      text = ''
        cd "${rootDir}"
        TAG=$(date +"%Y%m%d-%H%M%S")
        nix build .#cardano-testnet-docker-linux-amd64 -o result-amd64
        nix build .#cardano-testnet-docker-linux-arm64 -o result-arm64
        docker load < ./result-amd64
        docker load < ./result-arm64
        docker tag cardano-testnet:latest-amd64 "patextreme/cardano-testnet:$TAG-amd64"
        docker tag cardano-testnet:latest-arm64 "patextreme/cardano-testnet:$TAG-arm64"

        rm -rf ./result-amd64
        rm -rf ./result-arm64

        docker push "patextreme/cardano-testnet:$TAG-amd64"
        docker push "patextreme/cardano-testnet:$TAG-arm64"

        # create multi-arch image
        docker manifest create  "patextreme/cardano-testnet:$TAG" \
          "patextreme/cardano-testnet:$TAG-amd64" \
          "patextreme/cardano-testnet:$TAG-arm64"
        docker manifest push "patextreme/cardano-testnet:$TAG"
      '';
    };

    releasePrismNodeImage = pkgs.writeShellApplication {
      name = "releasePrismNodeImage";
      runtimeInputs = with pkgs; [
        nix
        docker
      ];
      text = ''
        cd "${rootDir}"
        TAG=$(date +"%Y%m%d-%H%M%S")
        nix build .#prism-node-docker -o result
        docker load < ./result
        rm -rf ./result
        docker tag prism-node-fastsync:latest "patextreme/prism-node-fastsync:$TAG"
        docker push "patextreme/prism-node-fastsync:$TAG"
      '';
    };
  };
in
pkgs.mkShell {
  packages = builtins.attrValues scripts;

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Working on project root directory: ${rootDir}"
    cd "${rootDir}"
  '';
}
