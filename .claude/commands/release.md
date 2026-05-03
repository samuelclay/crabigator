---
name: crabigator-release
description: Use when the user asks to cut a Crabigator release, optionally with a target version, including version bumps, GitHub tag/release creation, release notes, npm verification, and Worker deployment.
allowed-tools: Bash, Read, Edit
argument-hint: "[X.Y.Z]  (default: next patch)"
---
Cut a new Crabigator release: choose the version, bump the code, push to GitHub, create the release, write release notes covering everything since the previous release, then deploy the Worker.

`$ARGUMENTS` is optional:
- empty -> bump the patch component from the current `Cargo.toml` version
- `X.Y.Z` -> release that exact semver version
- `patch`, `minor`, and `major` are accepted as convenience aliases

Steps:

1. **Pre-flight the repo.**
   - Run `git branch --show-current`; releases must be cut from `main`. If not on `main`, stop and ask.
   - Run `git status --porcelain`. If output is non-empty, run the `/commit-push-deploy` logic first: inspect changes, split into logical commits per `CLAUDE.md`, push to `origin/main`, and deploy the Worker if relevant. Do not include the version bump in those commits.
   - Run `git fetch origin --tags` and `git status -sb`. If `main` is behind or diverged from `origin/main`, stop and ask before continuing.
   - Confirm `gh auth status` works. If it does not, stop and ask the user to authenticate GitHub CLI.

2. **Determine current and new versions.**
   - Read current version from `Cargo.toml` (line `version = "X.Y.Z"`).
   - Compute new version:
     - `patch` (default): increment Z
     - `minor`: increment Y, set Z = 0
     - `major`: increment X, set Y = 0, Z = 0
     - explicit `X.Y.Z`: validate it matches `^\d+\.\d+\.\d+$` and is greater than current
   - Verify the new tag `vX.Y.Z` does not already exist locally (`git rev-parse vX.Y.Z` should fail) or remotely (`git ls-remote --tags origin vX.Y.Z` should be empty). If it exists, stop and ask. Never overwrite a published tag.

3. **Identify the previous release and draft notes.**
   - Get the previous release tag before making the bump:
     ```
     PREV_TAG=$(gh release list --limit 1 --json tagName --jq '.[0].tagName')
     ```
     If GitHub has no release, use the latest local semver tag:
     ```
     PREV_TAG=$(git describe --tags --abbrev=0 --match 'v[0-9]*.[0-9]*.[0-9]*')
     ```
   - Build the complete commit list from `PREV_TAG..HEAD` before the version-bump commit:
     ```
     git log --reverse --format='- %s (%h)' "$PREV_TAG..HEAD"
     ```
   - Write `/tmp/crabigator-release-vX.Y.Z.md` with:
     - A short `## Highlights` section summarizing the user-visible changes from the commit list.
     - A `## Complete Changes` section containing every commit from `PREV_TAG..HEAD`, preserving commit subjects and short hashes.
     - A `## Full Changelog` section linking `https://github.com/samuelclay/crabigator/compare/PREV_TAG...vX.Y.Z`.
   - Do not invent changes. If the commit list is empty, stop and ask whether to cut an empty release.

4. **Bump version in both files** (CI enforces sync):
   - `Cargo.toml`: change `version = "OLD"` → `version = "NEW"` on the package line only (don't touch dependency version strings).
   - `npm/package.json`: change `"version": "OLD"` → `"version": "NEW"`.
   - Run `cargo build` so `Cargo.lock` updates with the new package version. Do not skip this; an out-of-sync lockfile will fail CI.
   - Verify all three versions match:
     - `Cargo.toml`
     - `Cargo.lock` package entry for `crabigator`
     - `npm/package.json`

5. **Test, commit, and push the bump to GitHub.**
   - Run `cargo test`.
   - Commit only the version files:
   ```
   git add Cargo.toml Cargo.lock npm/package.json
   git commit -m "Bump version to X.Y.Z"
   git push origin main
   ```

6. **Tag and push.**
   ```
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
   Pushing the tag triggers `.github/workflows/release.yml`, which builds 6 platform artifacts, publishes to npm, and creates the GitHub Release.

7. **Watch the release workflow.**
   ```
   gh run list --limit 1 --workflow release.yml
   gh run watch <run-id>
   ```
   Run the watch in the foreground. If the workflow fails, follow `CLAUDE.md` "If the Release Fails": delete the tag locally and remotely, fix the issue, push a new commit, re-tag, and push the tag again.

8. **Replace the GitHub release description with the drafted notes.**
   - After the workflow creates the release, run:
     ```
     gh release edit vX.Y.Z --notes-file /tmp/crabigator-release-vX.Y.Z.md
     ```
   - Verify the body contains the Highlights, Complete Changes, and Full Changelog sections:
     ```
     gh release view vX.Y.Z
     ```

9. **Verify release artifacts and npm.**
   ```
   gh release view vX.Y.Z --json tagName,url,isDraft,assets
   npm view crabigator@X.Y.Z version
   ```
   Confirm:
   - 6 assets present (darwin-arm64, darwin-x64, linux-arm64, linux-x64, win32-arm64, win32-x64)
   - Release is not a draft
   - npm has published `crabigator@X.Y.Z`

10. **Deploy the Cloudflare Worker.**
   ```
   make deploy
   ```
   This publishes any Worker/dashboard code that landed in the release. Capture the deployed Worker version ID from the output when available.

11. **Report back.**
    - New version, tag URL, release URL, npm version, Worker version ID.
    - Include the previous release tag used for the notes and the release workflow run URL.
    - If anything is still running or could not be verified, say so explicitly.

Notes:
- Never use `--no-verify`, `--no-gpg-sign`, or `--force` on tag pushes unless the user explicitly asks.
- Never amend the version-bump commit after the tag is pushed — the tag would become detached.
- The release workflow handles npm publish; do not run `npm publish` manually.
