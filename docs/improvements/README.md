# Hops improvement plan

This folder tracks a set of fixes and refactors that came out of a review of
the codebase on 2026-11-25. Each change has its own file, and each one is
discussed before it is implemented, usually in its own session. This README is
the shared context, so a session only needs to read this file and the one item
it is working on.

## Where this comes from

The review looked at security, robustness, duplication, oversized modules and
performance. The foundations were in good shape: URLs are parsed with the
`url` crate and limited to http/https before they reach a browser, config
writes are atomic, the running-process scan is lazy, and compiled patterns
are cached. Most of what it found is attack surface that can be removed,
validation that does not do what it looks like it does, two bugs that can
leave the app stuck, and duplicated code that has built up over time.

Line references in the item files point at commit `1450a83` (release 0.1.6
merged into `main`). Earlier items will move code around, so check the
references before editing.

## How we work through it

Pick the next item and start by discussing its open questions. Once we agree,
the answers go into the item's **Decisions** section, so the file alone
explains what was chosen and why. Then implement it on its own branch,
staying inside the item's scope, run the verification steps, and update the
table below with the status and the PR.

## Items

| #  | Item                                                                 | Status   | PR |
| -- | -------------------------------------------------------------------- | -------- | -- |
| 01 | [Command access per window](01-command-acl.md)                       | Proposed |    |
| 02 | [Manual browser trust model](02-manual-browser-trust.md)             | Proposed |    |
| 03 | [Config robustness](03-config-robustness.md)                         | Proposed |    |
| 04 | [Windows registration](04-windows-registration.md)                   | Proposed |    |
| 05 | [Config location and concurrency](05-config-location-concurrency.md) | Proposed |    |
| 06 | [CI and release hardening](06-ci-release.md)                         | Proposed |    |
| 07 | [Rust refactor and performance](07-rust-refactor-performance.md)     | Proposed |    |
| 08 | [Frontend refactor](08-frontend-refactor.md)                         | Proposed |    |

## Order

The suggested order is 01, 03, 04, 02, 05, 06, 07, 08: security and the bugs
users can hit first, refactors last so they do not collide with behavior
changes. A few items overlap and should respect that order:

- 07 refactors `browsers.rs`, `registration.rs` and `config.rs`, which 02, 04
  and 05 all change.
- 02 and 03 both change `validate_browser_for_launch`.
- 08 rewrites `useBrowserManager.ts`, which holds the TypeScript path
  normalization that 05 changes.
- 01 removes unused commands; later items should not bring them back.
