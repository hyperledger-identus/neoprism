set working-directory := '../..'

# Automatically bump version using git-cliff
bump-version:
    #!/usr/bin/env bash
    set -euxo pipefail
    NEW_VERSION=$(git-cliff --bump --context | jq -r .[0].version | sed s/^v//)
    just release set-version "$NEW_VERSION"

# Set project version manually
set-version VERSION:
    echo "Setting new version to {{ VERSION }}"
    echo "{{ VERSION }}" > version
    cargo set-version "{{ VERSION }}"
    just build-config
    git-cliff -t "{{ VERSION }}" > CHANGELOG.md

# Build and release multi-arch cardano-testnet Docker image
publish-testnet-image:
    #!/usr/bin/env bash
    set -euxo pipefail
    TAG=$(date +"%Y%m%d-%H%M%S")

    echo "Building amd64 image..."
    nix build .#cardano-testnet-docker-linux-amd64 -o result-amd64

    echo "Building arm64 image..."
    nix build .#cardano-testnet-docker-linux-arm64 -o result-arm64

    echo "Loading images into Docker..."
    docker load < ./result-amd64
    docker load < ./result-arm64

    echo "Tagging images with $TAG..."
    docker tag cardano-testnet:latest-amd64 "patextreme/cardano-testnet:$TAG-amd64"
    docker tag cardano-testnet:latest-arm64 "patextreme/cardano-testnet:$TAG-arm64"

    rm -rf ./result-amd64 ./result-arm64

    echo "Pushing architecture-specific images..."
    docker push "patextreme/cardano-testnet:$TAG-amd64"
    docker push "patextreme/cardano-testnet:$TAG-arm64"

    echo "Creating and pushing multi-arch manifest..."
    docker manifest create "patextreme/cardano-testnet:$TAG" \
      "patextreme/cardano-testnet:$TAG-amd64" \
      "patextreme/cardano-testnet:$TAG-arm64"
    docker manifest push "patextreme/cardano-testnet:$TAG"

    echo "✓ Released: patextreme/cardano-testnet:$TAG"
