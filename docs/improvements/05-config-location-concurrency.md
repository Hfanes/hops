# 05 — Config location and concurrency

Do after 03; both change `config.rs`.

## Context and problem

### A. Config lives in Roaming AppData

`config_file_path` (`config.rs:16-42`) uses `%APPDATA%\Hops\config.json`
(Roaming) on Windows and only falls back to Tauri's `app_data_dir()`. The
config stores absolute executable paths, which are machine-specific. In a
roaming-profile environment the config follows the user to machines where
those paths do not exist (which, until item 03, also blocks saving). There are
also two possible locations depending on whether `APPDATA` is set.

### B. Lost update in `refresh_browsers`

`refresh_browsers` (`commands.rs`) calls `load_or_init_config`, which takes
and releases the `ConfigCache` lock, runs detection, then calls
`save_config_internal`, which takes the lock again. A `save_config` that runs
in between (e.g. a debounced settings save) is overwritten. Commands are
`async` and run on different threads, so this can actually happen.

### C. Three path normalizations

- `browsers.rs:984` `normalize_path`: `/`→`\`, lowercase, **no trim**.
- `config.rs:187` `normalize_browser_path`: trim, `/`→`\`, lowercase.
- `useBrowserManager.ts:77` `normalizeBrowserPath`: trim, `/`→`\`, lowercase.

`is_browser_running` (`browsers.rs:1094`) does not trim, while
`browser_executable_names` (`browsers.rs:997`) does. A path with surrounding
whitespace can be found in the process scan but never reported as running.

## Open questions

1. **A.** Move to `%LOCALAPPDATA%` (Tauri `app_local_data_dir()`)? Is anything
   in the config worth roaming (theme, rules) and would splitting it be worth
   the complexity? (Probably not.)
2. **A.** Migration: on startup, if the new file is missing and the old one
   exists, copy it, then delete or keep the old one? What about the
   "Open config folder" button and the path shown in the status bar?
3. **B.** Fix by adding a `ConfigCache::update(|config| ...)` that does
   load-modify-save under one lock, and route `refresh_browsers` (and any
   similar command) through it? Should `save_config` from the frontend also
   detect stale writes (version or etag), or is serialization enough?
4. **C.** Trim paths once when the config is normalized, so every later
   comparison can skip trimming? Is keeping a TS copy of normalization needed
   at all, or can the duplicate-path check move fully to the backend?

## Decisions

_Filled in during the discussion._

## Proposal

- Single resolved config path from Tauri's local data dir, with a one-time
  migration from Roaming.
- `ConfigCache::update` for read-modify-write commands.
- One `normalize_path` in Rust (trimmed), used everywhere; paths trimmed in
  `normalize_config`; the TS copy removed or kept in sync as decided.

## Out of scope

- Invalid JSON recovery and validation timing (item 03).
- General refactors of `browsers.rs` (item 07).

## Acceptance criteria

- [ ] Fresh installs write config under Local AppData; existing users keep
      their settings after upgrading.
- [ ] A test shows a concurrent save during refresh is not lost (or the
      update path is covered by a unit test of `ConfigCache::update`).
- [ ] Only one path normalization function exists in Rust; whitespace in a
      configured path does not break running detection.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml --lib`
- `bun run build`
- Manual: install the previous version, configure rules, upgrade, check that
  rules survive and "Open config folder" opens the new location.

## Notes and risks

- Migration bugs lose user config. Keep the old file until the new one has
  been written and read back.
- Autostart and the updater do not depend on the config path, but double-check.
