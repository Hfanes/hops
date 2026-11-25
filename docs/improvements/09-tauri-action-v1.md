# 09 — Move the release build to tauri-action v1

Do after 06, which pinned `tauri-apps/tauri-action` to `v0.6.2` by SHA and
added Dependabot. Line references point at commit `8ed2175`.

## Context and problem

`release.yml` builds the installer, signs the updater artifacts and uploads
them, with `latest.json`, to a draft release through
`tauri-apps/tauri-action@84b9d35… # v0.6.2` (`release.yml:128-141`).
`tauri-action` 1.0.0 came out on 2026-06-29. Dependabot proposed it in #34,
which was closed without merging, because the update only changes the SHA
and v1 is not a drop-in replacement for how Hops uses the action.

### This has gone wrong before

Until 2026-06-01 the workflow used `tauri-action@v0` with v1 input names
(`releaseAssetNamePattern`, `uploadUpdaterJson`, `uploadUpdaterSignatures`),
probably copied from the action's README on its development branch before v1
was released. `v0` did not know those inputs and ignored them without an
error. Commit `aa899bd` moved to `v0.6.2` and its input names, which is what
the workflow uses today. Moving to v1 is the same mistake in the other
direction.

### What v1 changes for Hops

From the v1.0.0 release notes and the `action.yml` of each version:

1. **Renamed inputs.** Hops sets two inputs that v1 renamed:
   - `assetNamePattern: "hops[ext]"` → `releaseAssetNamePattern`
   - `includeUpdaterJson: true` → `uploadUpdaterJson`

   With only the SHA changed, v1 ignores both. The installer is uploaded
   under the default name instead of `hops.exe`. `uploadUpdaterJson` defaults
   to `true` in v1, so `latest.json` would most likely still be uploaded,
   but the download URLs inside it point at whatever the assets are called.
   Nothing fails in CI: `tauri-action` only runs in a release.
2. **`latest.json` URLs change format.** "The download urls in `latest.json`
   will now use the github url instead of the browser download url": the
   API asset URL (`https://api.github.com/repos/{owner}/{repo}/releases/assets/{id}`)
   instead of `https://github.com/Hfanes/hops/releases/download/v0.1.6/hops.exe`.
   The API URL only returns the file when the request sends
   `Accept: application/octet-stream`. `tauri-plugin-updater` 2.10.1 (the
   version Hops pins) sets that header on downloads unless the app overrides
   it (`updater.rs:657-661`), and Hops does not set updater headers. So it
   should work, but no release has been tested this way. Requests to
   `api.github.com` without a token are rate limited per IP (60 per hour),
   which one update per user does not reach.
3. **`.sig` files can be hidden again.** Commit `0d4d034` (2026-05-19) set
   `uploadUpdaterSignatures: false` to keep `hops.exe.sig` and `hops.msi.sig`
   off the release page. `v0.6.2` has no such input, so since `aa899bd` the
   `.sig` files are uploaded again (the v0.1.6 release lists them). v1 has
   `uploadUpdaterSignatures` again (default `true`); the signatures still go
   into `latest.json` either way.
4. **Behavior changes on existing releases.**
   - "The action will now fail if `draft: true` is set but the relevant
     release is not a draft." A manual run of `release.yml` for a tag whose
     release is already published now fails instead of uploading to it.
   - "The action will now update the name and body of existing releases."
     A manual retry on an existing draft overwrites any notes written by hand
     in that draft with `releaseBody`.
5. **Other breaking changes that do not apply to Hops:** Tauri v1 and
   unstable v2 support dropped (Hops uses Tauri 2.11), `includeRelease` and
   `includeDebug` removed, automatic project initialization removed,
   `updaterJsonKeepUniversal` removed (macOS only), `.app.tar.gz` naming
   (macOS only), Gitea support removed.

### Why move at all

`v0.6.2` keeps working, and it is pinned by SHA, so nothing changes under us.
The reasons to move are that fixes only land on v1 now, that Dependabot will
keep proposing every v1 release (closing #34 only skips 1.0.0), and point 3
above.

## Open questions

1. Move to v1 now, or stay on `v0.6.2` and tell Dependabot to ignore
   `tauri-action` major updates until we are ready? An `ignore` rule in
   `.github/dependabot.yml` stops the PRs, but also hides v1 security fixes
   (Dependabot security updates are separate from version updates and still
   apply).
2. `.sig` files on the release page: hide them again with
   `uploadUpdaterSignatures: false`, or keep them? They are harmless (a
   signature is public by design) but clutter the download list.
3. How to test the updater with the new `latest.json` URLs before real users
   get it? Drafts are not visible to the updater, and a manual run on the
   published `v0.1.6` release would now fail (point 4). Options:
   - Ship it with the next real version bump: inspect the draft's assets and
     `latest.json` before publishing, then update one install by hand right
     after publishing, before announcing it.
   - Point a local build's updater endpoint at a test release in a fork or a
     scratch repository, and update from it.
4. Keep `releaseBody` as is, now that v1 overwrites the body of an existing
   draft on retries?

## Decisions

_Filled in during the discussion._

## Proposal

- Bump `tauri-apps/tauri-action` to the latest v1 release by SHA, with the
  version in a comment.
- Rename `assetNamePattern` to `releaseAssetNamePattern` and
  `includeUpdaterJson` to `uploadUpdaterJson`. Keep `updaterJsonPreferNsis`.
- Set `uploadUpdaterSignatures` as decided.
- Update the comment at the top of `release.yml` about manual runs: a manual
  run for a published release now fails.
- Test as decided before the release reaches users.

## Out of scope

- The release flow itself (draft, then manual publish) and the `prepare` job.
- Moving the updater endpoint away from GitHub Releases.
- Bumping `tauri-plugin-updater` or other Tauri crates.

## Acceptance criteria

- [ ] `release.yml` uses a v1 SHA and only input names that exist in that
  version's `action.yml`.
- [ ] A draft built with v1 has `hops.exe`, `hops.msi` and `latest.json`,
  with the same names as v0.1.6, and the `.sig` files as decided.
- [ ] Each URL in that `latest.json` downloads the right file with
  `Accept: application/octet-stream`, and the signature matches.
- [ ] An installed Hops updates to the new release through the in-app
  updater.

## Verification

- Compare the input names in `release.yml` with the `action.yml` of the
  pinned v1 commit.
- After the first v1 build, list the draft's assets
  (`gh release view <tag> --json assets`) and download its `latest.json`.
- Download each URL in `latest.json` with
  `curl -L -H "Accept: application/octet-stream"` and check the file.
- Update an installed older version through the in-app updater, using the
  test path from open question 3.

## Notes and risks

- The failure mode is silent: a wrong input name is ignored, the release
  build passes, and the broken asset names or URLs only show up when someone
  installs or updates. Check the draft before publishing.
- Once a `latest.json` with broken URLs is published, every install that
  checks for updates fails until a fixed release replaces it. Deleting or
  fixing the release's `latest.json` asset by hand is the fastest way out.
- Dependabot keeps opening a PR for each new v1 release until this item is
  done or an `ignore` rule is added. Do not merge one of those PRs as is.
