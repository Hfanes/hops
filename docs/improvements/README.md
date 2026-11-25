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

### Before discussing: re-verify the item

Item files were written at a point in time and can be wrong or stale. Before
presenting the open questions:

- Re-read every file the item will touch, and every file that calls into or
  could be affected by that code (callers, wrappers, capabilities, tests).
- Check that each claim in the item is still true against the current code
  (line references, "unused", "only called from X", default behavior).
  For library or framework behavior, confirm it in the docs for the pinned
  version or in the generated files (`src-tauri/gen/schemas`,
  `node_modules`), not from memory.
- Call out any claim that turned out wrong, and correct the item file.

### How to present each open question

Present each question on its own, in this structure:

1. **Context**: what the code does today and why the question matters, with
   file references.
2. **Options**: for each option, what changes, then the pros and cons.
3. **Consequences**: what can actually happen with each option, with a
   concrete example (a scenario, an input, a sequence of events), including
   what breaks or stays exposed if we do nothing.
4. **Recommendation**: the option you suggest and how confident you are.

Leave the decision to the user. Record the answer, and the reason for it, in
the item's **Decisions** section.

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
