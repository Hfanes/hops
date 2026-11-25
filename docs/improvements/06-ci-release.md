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
   top level for both jobs; only the release job needs it. _(Wrong, see
   below: `prepare` needs it too.)_

### Re-verification (2026-11-25, against `969c4af`)

- **`prepare` also needs `contents: write`.** It runs `gh release list` to see
  whether a draft for the tag already exists. The GitHub REST docs for
  "List releases" say: "Only users with push access will receive listings for
  draft releases." With `contents: read` the drafts are hidden, the check sees
  "no release yet", and every push to `main` after a version bump builds
  another draft. Claim 5 only holds for `version-check.yml`, which already
  uses `contents: read`.
- **`pages.yml` uses other tags** than the list in claim 1:
  `actions/checkout@v5`, `oven-sh/setup-bun@v2`, `actions/configure-pages@v5`,
  `actions/upload-pages-artifact@v4`, `actions/deploy-pages@v4`. Its
  top-level `pages: write` and `id-token: write` are only needed by `deploy`.
- **The repository is public**, so GitHub-hosted runners, Windows included,
  cost nothing. The cost concern in open question 3 does not apply.
- **`main` has no branch protection or ruleset.** A new CI workflow reports a
  failure but does not block a merge unless its job is set as a required
  status check.
- The repository default for `GITHUB_TOKEN` is already `read`
  (`default_workflow_permissions: read`), so workflows without a
  `permissions` block get read-only access.
- Only the `TAURI_SIGNING_PRIVATE_KEY` secret exists (set 2026-05-19). There
  is no `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secret.
- Current state of the checks: `cargo fmt --check` passes, `bun run build`
  passes, `cargo test --lib` passes (97 tests), and
  `cargo clippy --all-targets -- -D warnings` fails with 16 errors in 4 lints:
  `needless_borrow`-style "reference immediately dereferenced" (9),
  `too_many_arguments` (3), `unnecessary_operation` (2),
  `unnecessary_sort_by` (2).

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

Agreed on 2026-11-25.

1. **Pin every action by commit SHA, with the version in a comment, and let
   Dependabot update them.** `.github/dependabot.yml` covers `github-actions`,
   weekly, one PR per action (no groups), so each update is reviewed on its
   own as `AGENTS.md` asks, with a 3-day `cooldown` before a new release is
   proposed. Why: a re-tagged or compromised action would otherwise run in the
   job that holds the updater signing key. `dtolnay/rust-toolchain` has no
   release tags, so it is pinned to a `master` commit with
   `toolchain: stable` passed explicitly, and its SHA is bumped by hand.
2. **Keep the current updater key and document its status.** No rotation.
   The key has no password (or an empty one): only
   `TAURI_SIGNING_PRIVATE_KEY` is set, and releases sign without
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Why: in CI a password would live in
   GitHub Secrets next to the key, so anyone able to read one could read the
   other. A password only protects the local copy of the key file, which
   should be kept in a password manager. Rotating changes the `pubkey` and
   needs a transition release; a mistake there stops updates for every
   install, which costs more than it protects.
3. **One `ci.yml` job on `windows-latest`** running `bun install
   --frozen-lockfile`, `bun run build`, `cargo fmt --check`,
   `cargo clippy --all-targets -- -D warnings` and `cargo test --lib`, on
   pull requests to `main` and on manual runs. Why: most of the backend is
   behind `cfg(target_os = "windows")`, so a Linux job would not test it, and
   Windows runners are free for this public repository. The existing clippy
   errors are fixed in this branch; the three `too_many_arguments` functions in
   `picker.rs` get an `allow`, because splitting their arguments belongs to
   item 07.
   The Rust steps (toolchain, cache, fmt, clippy, tests) only run when the PR
   changes something under `src-tauri/` or `ci.yml` itself; pushes to `main`
   and manual runs always run them. Why: on a cold cache they take about 4
   minutes, and a PR that only touches docs, the website or the frontend
   cannot break them. The job always runs and only skips steps, because a
   required check that never reports would block the merge.
   CI runs on pull requests to `main` and on pushes to `main` (every merge),
   not on every branch. Why: the PR run tests the code merged with `main`,
   which is what the ruleset gates; the `main` run checks the result of the
   merge and fills the Rust cache, which GitHub only shares from `main` to new
   PRs (a PR's cache is only visible to that PR). Running on every branch push
   as well would run twice for each push to a branch with an open PR. For
   early feedback on unfinished work, open a draft PR.
   **Ruleset on `main`:** require the `checks` job (`CI / checks`) and the
   `version-check` job to pass before merging, with the repository owner
   allowed to bypass. "Require branches to be up to date" stays off. Why:
   without a ruleset a red check only warns, and a careless merge puts
   broken code on `main` and can create a release draft. The owner bypass
   keeps an exit if CI breaks for reasons outside the code (a GitHub outage, a
   broken action), and leaving "up to date" off avoids rebasing every open PR
   whenever another one merges. The ruleset lives in the repository settings,
   not in code, and is created only after `ci.yml` is on `main`, because
   GitHub only knows a check once it has run there.
4. **No Authenticode signing, and nothing more to do for it in this item.**
   Installers stay unsigned and SmartScreen keeps warning on first download.
5. **`pages.yml` gets the same treatment:** SHA pins, `contents: read` at the
   top, `pages: read` for `build` (configure-pages reads the site settings)
   and `pages: write` plus `id-token: write` only on `deploy`.

Also applied, in the same spirit: `persist-credentials: false` on every
checkout, since no job pushes with git (the `prepare` job's
`git ls-remote` works without credentials on a public repository, and
`gh` and tauri-action use the token from `env`).

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

- [x] No `uses:` line references a tag or branch, only SHAs.
- [ ] A PR that breaks a Rust test or the TS build fails CI.
- [x] Each job has minimal `permissions`.
- [x] The signing key status is documented (encrypted or not, rotation plan).

## Verification

- Open a test PR with a deliberately failing test and confirm CI blocks it.
- Run the release workflow manually (`workflow_dispatch`) on a test tag, or
  dry-run the `prepare` job, and check the draft is produced.

## Notes and risks

- A wrong SHA or a missing permission breaks the release only when a release
  is due. Test with `workflow_dispatch` before the next version bump.
- `clippy -D warnings` may fail on existing code at first. Fix or `allow` in
  the same PR.
- `pages: read` for the Pages `build` job is not documented by
  configure-pages. If that job fails with a 403 on the Pages API, it needs
  `pages: write`. Check with a manual run of the Website workflow.
- Branch protection: `main` had no ruleset at the time of this item. The
  ruleset from decision 3 is created after this branch is merged.
