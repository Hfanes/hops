# 06 — CI and release hardening

## Context and problem

Workflows: `.github/workflows/release.yml`, `version-check.yml`, `pages.yml`.

1. **Actions pinned by mutable tags.** `actions/checkout@v6`,
   `oven-sh/setup-bun@v2`, `dtolnay/rust-toolchain@stable`,
   `swatinem/rust-cache@v2`, `tauri-apps/tauri-action@v0.6.2`. A compromised
   or re-tagged action runs in the job that holds `TAURI_SIGNING_PRIVATE_KEY`,
   the key that signs updates every install trusts.
2. **No password for the signing key.** `release.yml` passes
   `TAURI_SIGNING_PRIVATE_KEY` but not `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
   Either the key is unencrypted, or the password is not needed. Confirm.
3. **No quality gates on PRs.** `version-check.yml` only checks that versions
   agree. Nothing runs `cargo test`, `cargo clippy`, `cargo fmt --check`, or
   `bun run build` (`tsc`) before merging to `main`, and a merge to `main` can
   create a release draft.
4. **No Authenticode signing.** The NSIS installer and exe are unsigned, so
   Windows SmartScreen warns users on download and install.
5. **Broad workflow permissions.** `release.yml` sets `contents: write` at the
   top level for both jobs; only the release job needs it.

## Open questions

1. Pin by commit SHA (with a version comment), and keep them updated with
   Dependabot or Renovate for `github-actions`? That fits the "review updates
   one at a time" rule in `AGENTS.md`.
2. Is the updater key encrypted? If not, rotate to an encrypted key? Note that
   rotating changes the `pubkey` in `tauri.conf.json`, so installs on the old
   key need one release signed with the old key that ships the new pubkey.
3. PR checks: one Windows job (`cargo test`, `clippy -D warnings`,
   `fmt --check`, `bun install --frozen-lockfile`, `bun run build`)? Windows
   runners are slower and pricier; is Linux enough for the parts that are not
   Windows-specific? (Most tests use Windows APIs.)
4. Authenticode: worth the cost now (certificate, or Azure Trusted Signing),
   or later? This item could just document the option.
5. Should `pages.yml` get the same pinning and least-privilege review?

## Decisions

_Filled in during the discussion._

## Proposal

- Pin every action to a SHA; add Dependabot config for `github-actions`.
- Per-job `permissions`; `contents: read` by default.
- New `ci.yml` on `pull_request` to `main` with tests, clippy, fmt and the
  frontend build.
- Confirm or rotate the signing key setup.
- Authenticode as decided.

## Out of scope

- Changing the release flow itself (draft, then manual publish), which works.

## Acceptance criteria

- [ ] No `uses:` line references a tag or branch, only SHAs.
- [ ] A PR that breaks a Rust test or the TS build fails CI.
- [ ] Each job has minimal `permissions`.
- [ ] The signing key status is documented (encrypted or not, rotation plan).

## Verification

- Open a test PR with a deliberately failing test and confirm CI blocks it.
- Run the release workflow manually (`workflow_dispatch`) on a test tag, or
  dry-run the `prepare` job, and check the draft is produced.

## Notes and risks

- A wrong SHA or a missing permission breaks the release only when a release
  is due. Test with `workflow_dispatch` before the next version bump.
- `clippy -D warnings` may fail on existing code at first. Fix or `allow` in
  the same PR.
