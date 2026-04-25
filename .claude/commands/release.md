---
allowed-tools: Bash, Read, Edit
argument-hint: "[major|minor|patch|X.Y.Z]  (default: patch)"
---
Cut a new release: bump version, tag, push, watch CI, then deploy the Worker.

The argument `$ARGUMENTS` is one of:
- empty or `patch` → bump the patch component (default)
- `minor` → bump minor, reset patch to 0
- `major` → bump major, reset minor and patch to 0
- a literal `X.Y.Z` → use that exact version

Steps:

1. **Pre-flight: commit any pending work first.**
   - Run `git status --porcelain`. If output is non-empty, run the `/commit-push-deploy` logic before anything else (split into logical commits per CLAUDE.md, push to origin/main, deploy the Worker). Do NOT bundle the version bump into those commits — the version bump is its own commit so the tag points at a clean "Bump version to X.Y.Z" commit.
   - Run `git fetch origin && git status -sb`. If `main` is behind `origin/main`, stop and ask the user — don't release stale code.

2. **Determine current and new versions.**
   - Read current version from `Cargo.toml` (line `version = "X.Y.Z"`).
   - Compute new version:
     - `patch` (default): increment Z
     - `minor`: increment Y, set Z = 0
     - `major`: increment X, set Y = 0, Z = 0
     - explicit `X.Y.Z`: validate it matches `^\d+\.\d+\.\d+$` and is greater than current; otherwise stop and ask
   - Verify the tag `vX.Y.Z` does not already exist locally (`git rev-parse vX.Y.Z` should fail) or remotely (`git ls-remote --tags origin vX.Y.Z` should be empty). If it does, stop and ask — never overwrite a published tag.

3. **Bump version in BOTH files** (CI enforces sync):
   - `Cargo.toml`: change `version = "OLD"` → `version = "NEW"` on the package line only (don't touch dependency version strings).
   - `npm/package.json`: change `"version": "OLD"` → `"version": "NEW"`.
   - Run `cargo build` so `Cargo.lock` updates with the new version. Don't skip this — an out-of-sync lockfile will fail CI.

4. **Commit and push the bump.**
   ```
   git add Cargo.toml Cargo.lock npm/package.json
   git commit -m "Bump version to X.Y.Z"
   git push origin main
   ```

5. **Tag and push.**
   ```
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
   Pushing the tag triggers `.github/workflows/release.yml`, which builds 6 platform artifacts, publishes to npm, and creates the GitHub Release with auto-generated notes (`generate_release_notes: true` is already set in the workflow — so commits since the previous tag and a "Full Changelog" compare link are populated automatically).

6. **Watch the release workflow.**
   ```
   gh run list --limit 1 --workflow release.yml
   gh run watch <run-id>
   ```
   Run the watch in the foreground so the user sees progress.

7. **If the workflow fails**, follow CLAUDE.md "If the Release Fails":
   - Most common cause: `Cargo.toml` / `npm/package.json` version mismatch. Step 3 prevents this, but check anyway.
   - If you need to retry: delete the tag locally and remotely, fix the issue, push a new commit, re-tag, push the tag.

8. **Verify the release.**
   ```
   gh release view vX.Y.Z
   ```
   Confirm:
   - 6 assets present (darwin-arm64, darwin-x64, linux-arm64, linux-x64, win32-arm64, win32-x64)
   - Release is not a draft. If it is, run `gh release edit vX.Y.Z --draft=false`.
   - Notes contain commit list and "Full Changelog: ...compare/vPREV...vX.Y.Z" link.

9. **Deploy the Cloudflare Worker.**
   ```
   make deploy
   ```
   Same as `/deploy` — pushes the latest dashboard/worker code so anything Worker-side that landed alongside the release goes live.

10. **Report back.**
    - New version, tag URL, release URL, npm version, Worker version ID.
    - If the workflow is still running when this command finishes, say so explicitly with the run URL.

Notes:
- Never use `--no-verify`, `--no-gpg-sign`, or `--force` on tag pushes unless the user explicitly asks.
- Never amend the version-bump commit after the tag is pushed — the tag would become detached.
- The release workflow handles npm publish; do NOT run `npm publish` manually.
