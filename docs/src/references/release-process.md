# Release Process

This document describes the step-by-step process for creating a new release of NeoPRISM.

## Versioning

NeoPRISM follows [Semantic Versioning](https://semver.org/) (semver). The current version is stored in two places:

| File | Location |
|------|----------|
| [`version`](https://github.com/hyperledger-identus/neoprism/blob/main/version) | Repository root — single source of truth |
| [`Cargo.toml`](https://github.com/hyperledger-identus/neoprism/blob/main/Cargo.toml) | Workspace root — must match `version` |

## Prerequisites

Before starting a release, ensure you have:

- **Write access** to the [hyperledger-identus/neoprism](https://github.com/hyperledger-identus/neoprism) repository
- **A GitHub personal access token** with `repo` scope for pushing tags
- **Conventional commits** — all commits in the release should follow the [Conventional Commits](https://www.conventionalcommits.org/) standard. PR titles are validated against the conventional commits format by `.github/workflows/pr-lint.yml` (using `amannn/action-semantic-pull-request@v6`), and `git-cliff` uses conventional commit messages to generate the changelog automatically.
- **Nix** — all release commands run inside `nix develop`

## Release Steps

### 1. Ensure you are on the latest `main` branch

```bash
git checkout main
git pull origin main
```

### 2. Checkout the `release` branch

Create the `release` branch to the current `main`:

```bash
git checkout -b release
```

### 3. Bump the version

Run the automated version bump command from inside `nix develop`:

```bash
nix develop -c just release::bump-version
```

The `release::bump-version` recipe does all of the following automatically:

1. **Determines the next version** — uses `git-cliff` to compute the next version based on conventional commits since the last tag
2. **Updates `version` and `Cargo.toml`** — writes the new version to both files
3. **Generates the changelog** — runs `git-cliff` to rebuild `CHANGELOG.md` from the commit history
4. **Regenerates Docker Compose configs** — runs `just build-config` to update the auto-generated Docker Compose files with the new version

### 4. Commit the version bump

```bash
git add .
git commit -s -m 'chore(release): prepare for the next release'
```

### 5. Push and open a pull request

```bash
git push origin release
```

Open a pull request from `release` into `main` on GitHub. Once CI checks pass, merge the PR.

### 6. Create and push the tag

After the PR is merged to `main`, create a tag for the release and push it:

```bash
git checkout main
git pull origin main
VERSION=$(cat version)
git tag "v$VERSION"
git push origin "v$VERSION"
```

### 7. Trigger the release workflow

> **Note:** The release workflow is triggered **manually** — it does not run automatically on tags.

Go to the [Release workflow](https://github.com/hyperledger-identus/neoprism/actions/workflows/release.yml) on GitHub Actions and click **Run workflow**. Enter the version number (without the `v` prefix) in the `tag` input field.

The workflow will:

1. Check out the tagged commit
2. Build Docker images for **linux/amd64** and **linux/arm64** using Nix
3. Create a multi-arch Docker manifest and push it to Docker Hub under `$DOCKERHUB_ORG/identus-neoprism:<VERSION>`

### 8. Release the docs

Trigger the [Deploy Docs Site](https://github.com/hyperledger-identus/neoprism/actions/workflows/deploy-docs.yml) workflow manually to publish the updated mdBook site to GitHub Pages.

### 9. Create a GitHub Release

Navigate to the [Releases page](https://github.com/hyperledger-identus/neoprism/releases) and click **Draft a new release**. Select the `v<VERSION>` tag.

**Optional:** Curate the notable changes or highlight the breaking changes or other notes which require attention.

### 10. Verify the release

- **Docker Hub**: Confirm the multi-arch image is available at `docker pull $DOCKERHUB_ORG/identus-neoprism:<VERSION>`
- **GitHub Releases**: Confirm the release is published on the [Releases page](https://github.com/hyperledger-identus/neoprism/releases)
- **Docs site**: Confirm the updated documentation is live on GitHub Pages

## Release workflow reference

The CI pipeline for releases is defined in `.github/workflows/release.yml`. Key details:

| Aspect | Detail |
|--------|--------|
| Trigger | `workflow_dispatch` with a version tag |
| Architecture | Builds both `amd64` and `arm64`, combines into a multi-arch manifest |
| Registry | Docker Hub (`$DOCKERHUB_ORG/identus-neoprism`) |
| Build system | Nix (`nix build .#neoprism-docker-linux-amd64`, `nix build .#neoprism-docker-linux-arm64`) |

## Related files

| File | Purpose |
|------|---------|
| [`version`](https://github.com/hyperledger-identus/neoprism/blob/main/version) | Current version number |
| [`Cargo.toml`](https://github.com/hyperledger-identus/neoprism/blob/main/Cargo.toml) | Workspace metadata (version must match `version` file) |
| [`justfile`](https://github.com/hyperledger-identus/neoprism/blob/main/justfile) | Main justfile (loads the `release` module from `tools/just-recipes/release.just`) |
| [`tools/just-recipes/release.just`](https://github.com/hyperledger-identus/neoprism/blob/main/tools/just-recipes/release.just) | Contains the `release::bump-version` and `release::set-version` recipes |
| [`CHANGELOG.md`](https://github.com/hyperledger-identus/neoprism/blob/main/CHANGELOG.md) | Auto-generated changelog (rebuilt by `release::bump-version` via `git-cliff`) |
| [`cliff.toml`](https://github.com/hyperledger-identus/neoprism/blob/main/cliff.toml) | `git-cliff` configuration controlling changelog format |
| [`.github/workflows/release.yml`](https://github.com/hyperledger-identus/neoprism/blob/main/.github/workflows/release.yml) | Release CI workflow |
| [`.github/workflows/deploy-docs.yml`](https://github.com/hyperledger-identus/neoprism/blob/main/.github/workflows/deploy-docs.yml) | Docs site deployment |
