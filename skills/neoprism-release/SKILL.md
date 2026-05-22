---
name: neoprism-release
description: Execute the full NeoPRISM release process — version bump, changelog generation, tag & release creation, Docker image publishing, docs deployment, and verification. Use this skill whenever the user asks to "release neoprism", "create a new release", "bump the version and release", "do a release of neoprism", "publish a new version", "cut a release", or mentions anything related to releasing, publishing, or shipping a new version of the NeoPRISM project. Also use it when the user asks about release steps, the release checklist, or wants to verify that a release was done correctly. This skill covers the full end-to-end release workflow documented in docs/src/references/release-process.md.
---

# NeoPRISM Release Process

This skill guides you through the complete NeoPRISM release workflow. The release process is largely automated — the skill helps you orchestrate the steps, run the right commands, and verify outcomes.

## Important: Context Matters

The skill automatically infers or tests most prerequisites. Only one question needs user input:

1. **Do they want a dry run?** For a safe walkthrough without actually pushing, stop after step 4 (commit the version bump) and present the diff. If not asked, the skill assumes a full release.

**Everything else is handled automatically:**

- **Version**: The automated bump tool (`git-cliff`) computes the next version from conventional commits since the last tag. Just run `just release::bump-version` and show the result. If the user wants a specific version, they can override after seeing it.
- **Write access**: Tested at push time — attempt `git push` and handle errors gracefully with a clear message.
- **GitHub authentication**: Whatever mechanism the user has configured (SSH, HTTPS token, credential helper) works. Push errors will surface auth issues naturally.
- **Nix environment**: Detected automatically — either commands run inside `nix develop` or are prefixed with `nix develop -c`.

## Prerequisites Check

Before executing steps, verify:

- Repository: `hyperledger-identus/neoprism` (check `git remote -v`)
- Current branch: `main` (clean, no uncommitted changes)
- Nix is available: `nix develop --command bash -c "which just && which git-cliff && which jq"`
- Git user is configured: `git config user.name` and `git config user.email` are set
- Docker is available locally (if you intend to manually verify images after the release)

## Release Steps

### Step 1: Ensure on latest `main` branch

Confirm the user is on the latest `main` with no uncommitted changes:

```bash
git checkout main
git pull origin main
git status  # should say "nothing to commit, working tree clean"
```

Explain to the user that this ensures the release starts from the latest state of the codebase.

### Step 2: Checkout the `release` branch

Create a fresh `release` branch from `main`. If a `release` branch already exists locally, warn the user before deleting/recreating it:

```bash
# Check if release branch exists and warn the user
git branch --list release
# If it exists locally and they confirm:
git branch -D release
# Create fresh from main:
git checkout -b release
```

Explain: the `release` branch is a short-lived branch used to prepare the release PR. It gets merged back into `main` after CI passes.

### Step 3: Bump the version

Run the automated version bump inside `nix develop`. This is the key automated step:

```bash
nix develop -c just release::bump-version
```

**What this does automatically:**

1. Uses `git-cliff` to determine the next version based on conventional commits since the last tag
2. Writes the new version to both `version` (root file) and `Cargo.toml` (workspace metadata)
3. Rebuilds `CHANGELOG.md` from the full commit history via `git-cliff`
4. Regenerates Docker Compose configurations (`just build-config`) with the new version

After it completes, show the user a summary of what changed:

- `git diff --stat` — which files were modified
- `cat version` — the new version number
- The beginning of `CHANGELOG.md` — the newly generated changelog entries

If the bump fails (e.g., no conventional commits since last tag, or git-cliff cannot determine the next version), explain the issue and suggest:

- Check that there are commits since the last tag: `git log $(git describe --tags --abbrev=0)..HEAD --oneline`
- Check that commits follow conventional commit format
- Alternatively, the user can set a version manually: `just release::set-version X.Y.Z`

### Step 4: Commit the version bump

```bash
git add .
git commit -s -m 'chore(release): prepare for the next release'
```

Show the user the final commit diff (`git show HEAD --stat`) and ask for confirmation before proceeding to push.

**The `-s` flag adds a Signed-off-by trailer** — this is required by the DCO (Developer Certificate of Origin) process that Hyperledger projects use.

### Step 5: Push and open a pull request

```bash
git push origin release
```

After pushing, open a PR from `release` into `main` on GitHub. The user can do this in a browser, or you can provide the GitHub URL:

```
https://github.com/hyperledger-identus/neoprism/compare/main...release
```

**Important:** The PR title should match the commit message: `chore(release): prepare for the next release`.

Explain to the user:

- CI checks (lints, tests, builds) will run automatically on the PR
- Once CI passes, the PR gets merged into `main`
- **Do not merge the PR until CI is green**
- Merging should use the exact commit preserved for tagging

### Step 6: Create and push the tag

After the PR is merged to `main`:

```bash
git checkout main
git pull origin main
VERSION=$(cat version)
git tag "v$VERSION"
git push origin "v$VERSION"
```

Explain: the tag (`vX.Y.Z`) is what triggers downstream consumers and is used by the release workflow to identify which commit to build. The version is read from the `version` file (single source of truth).

### Step 7: Trigger the release workflow

The release CI workflow is triggered **manually** via GitHub Actions — it does NOT run automatically on tags.

Guide the user to:

1. Go to <https://github.com/hyperledger-identus/neoprism/actions/workflows/release.yml>
2. Click **Run workflow** (dropdown button on the right)
3. Enter the version number **without the `v` prefix** in the `tag` input field (e.g., `0.15.0` not `v0.15.0`)
4. Click **Run workflow**

**What the workflow does:**

- Checks out the tagged commit
- Builds Docker images for **linux/amd64** and **linux/arm64** using Nix
- Creates a multi-arch Docker manifest and pushes to Docker Hub under `$DOCKERHUB_ORG/identus-neoprism:<VERSION>`

**Note:** Docker Hub authentication is handled entirely by the CI workflow using pre-configured repository-level GitHub Actions variables and secrets (`DOCKERHUB_ORG`, `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`). No user Docker credentials are needed.

### Step 8: Release the docs

Trigger the docs deployment workflow:

Guide the user to:

1. Go to <https://github.com/hyperledger-identus/neoprism/actions/workflows/deploy-docs.yml>
2. Click **Run workflow**
3. Select the `main` branch (or the tagged commit)
4. Click **Run workflow**

This publishes the updated mdBook documentation site to GitHub Pages.

### Step 9: Create a GitHub Release

Guide the user to:

1. Go to <https://github.com/hyperledger-identus/neoprism/releases>
2. Click **Draft a new release**
3. Select the `v<VERSION>` tag from the dropdown
4. The release title should be the version (e.g., `v0.15.0`)
5. The description can be auto-generated from the CHANGELOG.md content for this version, or manually curated

**Optional but recommended:** Curate the release notes:

- Highlight breaking changes prominently
- Call out notable features or fixes
- Mention any migration steps or deprecations
- Thank contributors (you can get the contributor list from the commit history)

### Step 10: Verify the release

After all steps are complete, verify these three things:

```bash
# 1. Check the tag exists remotely
git tag -l "v*" | tail -5
git ls-remote --tags origin | grep "v$(cat version)"

# 2. Check Docker Hub
echo "Check: docker pull $DOCKERHUB_ORG/identus-neoprism:$(cat version)"
# (run this manually after the release workflow finishes)

# 3. Check the docs site
echo "Check: https://hyperledger-identus.github.io/neoprism/"
```

Report the results to the user:

| What to verify | How |
|---|---|
| **Docker Hub** | Confirm the multi-arch image at `docker pull $DOCKERHUB_ORG/identus-neoprism:<VERSION>` |
| **GitHub Releases** | Confirm the release is published on the [Releases page](https://github.com/hyperledger-identus/neoprism/releases) |
| **Docs site** | Confirm the updated documentation is live on [GitHub Pages](https://hyperledger-identus.github.io/neoprism/) |

## Common Issues & Troubleshooting

| Problem | Likely Cause | Solution |
|---|---|---|
| `git-cliff --bump` fails | No conventional commits since last tag | Check with `git log $(git describe --tags --abbrev=0)..HEAD --oneline` |
| `git push origin release` fails | No write access, or branch protection | Verify GitHub token and permissions |
| `git push origin vX.Y.Z` fails | Tag already exists | Check with `git tag -l "v*"` and delete locally+remotely if needed |
| Release workflow fails in CI | Build issue or missing secrets | Check workflow logs at GitHub Actions |
| Docker image not found | Workflow still running or failed | Check workflow status before verifying |

## Reference: Key Files

| File | Purpose |
|---|---|
| `version` | Single source of truth for current version |
| `Cargo.toml` | Workspace metadata (version must match `version` file) |
| `CHANGELOG.md` | Auto-generated changelog |
| `cliff.toml` | `git-cliff` configuration |
| `justfile` | Top-level justfile (loads `release` module) |
| `tools/just-recipes/release.just` | Release recipes (`bump-version`, `set-version`) |
| `.github/workflows/release.yml` | Release CI workflow |
| `.github/workflows/deploy-docs.yml` | Docs site deployment |
| `docs/src/references/release-process.md` | Full release documentation |
