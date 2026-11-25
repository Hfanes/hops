# 05 — Config location and concurrency

Do after 03; both change `config.rs`.

## Context and problem

### A. Config lives in Roaming AppData

`config_file_path` (`config.rs:17-43`) uses `%APPDATA%\Hops\config.json`
(Roaming) on Windows and only falls back to Tauri's `app_data_dir()`, which is
`%APPDATA%\com.hfa.hops`, so Roaming as well. The
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

The frontend has the same race one level up: `handleRefreshBrowsers`
(`useBrowserManager.ts:364`) does not go through the save queue in
`useConfigStore.ts`, and a debounced settings save carries the whole config as
it was when it was scheduled. If it fires after the refresh, it writes back the
browser list from before the refresh.

### C. Three path normalizations

- `browsers.rs:820` `normalize_path`: `/`→`\`, lowercase, **no trim**.
- `config.rs:307` `normalize_browser_path`: trim, `/`→`\`, lowercase.
- `useBrowserManager.ts:77` `normalizeBrowserPath`: trim, `/`→`\`, lowercase.
- `picker.rs:755` (test helper `running_snapshot`): trim, `/`→`\`, lowercase.

`is_browser_running` (`browsers.rs:930`) does not trim, while
`browser_executable_names` (`browsers.rs:833`) does. Callers trim
inconsistently: `is_hops_executable`, `browser_executable_exists` and
`validate_browser_entry` trim, `merge_detected_browsers` and
`build_detected_browser` do not. `build_detected_browser` feeds the normalized
path to `stable_id`, so trimming inside `normalize_path` changes the ID of a
detected browser whose path has surrounding whitespace. Detected paths come
from the registry (already trimmed by `extract_executable_path`) or from known
install paths, so no real ID should change. A path with surrounding
whitespace can be found in the process scan but never reported as running.

### Facts checked on 2026-11-25

- Tauri 2.11.2 `app_local_data_dir()` is `dirs::data_local_dir()` joined with
  the identifier, so `%LOCALAPPDATA%\com.hfa.hops`. That folder already exists
  on every install: Tauri puts the WebView2 profile there (`EBWebView`,
  `manager/webview.rs:534-544`).
- The NSIS uninstaller's "Delete app data" checkbox removes
  `$APPDATA\${BUNDLEID}` and `$LOCALAPPDATA\${BUNDLEID}`. Today the config in
  `%APPDATA%\Hops` survives that checkbox; under the local data dir it would
  be removed with it (and kept on normal updates, which skip that step).
- `%LOCALAPPDATA%\hops` is the install directory (`installMode:
  currentUser`, product name `hops`), so the config must not go there.
- Autostart (Run value) and the updater do not read the config path.

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

1. **Location: everything moves to `%LOCALAPPDATA%\com.hfa.hops`
   (`app_local_data_dir()`), no split.** Roaming only matters with roaming
   profiles, which few users have, and even there the rules point at browser
   IDs derived from executable paths on the original machine, so they would
   mostly reference "missing" browsers. Splitting the file is not worth it.
   Accepted consequence: the uninstaller's "Delete app data" option now
   removes the config, as it should.
2. **Migration: copy, read back, then rename the old file to
   `config.json.migrated` (option c).** Runs once per run when the config
   path is first resolved, only if the new file does not exist. The legacy
   locations are `%APPDATA%\Hops\config.json` and Tauri's roaming app data
   dir (the old fallback). The old file is only renamed after the copy was
   read back byte for byte, so it stays recoverable by hand, and an older
   version of Hops does not silently keep using a stale copy. If the copy
   fails, Hops keeps using the old file for that run and retries on the next
   one, and the settings window shows it once in the status bar (the same
   one-time notice as an invalid-config recovery, combined if both happen):
   the error, that settings are unchanged, where they are still saved, and
   that Hops will try again. No Windows notification: nothing is lost, and a
   failure that repeats would show one on every start. A taken `.migrated` name gets a counter, like the `.broken-*`
   backups. Old `.broken-*` backups are not moved. "Open config folder" and
   the status bar path go through the same resolved path, so they show the
   new location without other changes.
3. **Concurrency: backend `update` plus the frontend queue (option b), no
   version or etag.** `CachedConfigState::update` loads, changes, normalizes
   and saves under one lock; `refresh_browsers` runs detection first, outside
   the lock (URL handling waits on it), then merges inside `update_config`.
   In the frontend, `enqueueConfigTask` puts backend calls that write the
   config on the same queue as saves. Refresh first flushes and awaits
   debounced browser edits (which validate before they queue, so they are
   tracked from the start), queues the debounced settings save, then runs.
   Reset drops pending edits instead, since it discards them, and queues
   behind saves already in flight. Optimistic concurrency (option c) is too
   much for a single-user app.
4. **Paths: one `normalize_path` in Rust that trims, and `normalize_config`
   stores paths trimmed (option a).** `normalize_browser_path` and the test
   helper copy in `picker.rs` are gone, and the redundant `.trim()` calls
   before `normalize_path` were dropped. The TypeScript copy stays as an
   early UI check, with a comment that it mirrors the Rust one; the backend
   still rejects duplicates in `validate_config`. Item 08 rewrites that file.

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

- [x] Fresh installs write config under Local AppData; existing users keep
      their settings after upgrading.
- [x] A test shows a concurrent save during refresh is not lost (or the
      update path is covered by a unit test of `ConfigCache::update`).
- [x] Only one path normalization function exists in Rust; whitespace in a
      configured path does not break running detection.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml --lib`: migration (copy
  and rename, existing target, no legacy file, first legacy wins, earlier
  `.migrated` kept, failed copy keeps using the old file), `update` building
  on a save made after the last load, trimmed paths in `normalize_config`,
  and running detection with a padded path.
- `bun run build`
- Manual checks, packaged build:
  - Install 0.1.6, configure rules and a manual browser, upgrade. Rules and
    browsers survive, `%LOCALAPPDATA%\com.hfa.hops\config.json` exists, and
    `%APPDATA%\Hops\config.json.migrated` replaced the old file.
  - "Open config folder" opens `%LOCALAPPDATA%\com.hfa.hops`; the status bar
    shows that path.
  - Fresh install (no `%APPDATA%\Hops`): the config is created under Local
    AppData only.
  - Change a setting and click "Refresh browsers" right away: after reopening
    settings, both the setting and the detected browsers are kept.
  - Uninstall with "Delete app data" checked: the config folder is removed.
  - Make the move fail: with a 0.1.6 config in `%APPDATA%\Hops` and no
    `config.json` in the new folder, deny creating files in it with
    `icacls "%LOCALAPPDATA%\com.hfa.hops" /deny "%USERNAME%:(WD)"` (applies
    to that folder only, so WebView2's `EBWebView` subfolder still works),
    then start Hops. Settings shows a warning that names the old path, rules
    still work, and `%APPDATA%\Hops\config.json` is not renamed. Undo with
    `icacls "%LOCALAPPDATA%\com.hfa.hops" /remove:d "%USERNAME%"`, restart,
    and the move succeeds with no warning.

## Notes and risks

- Migration bugs lose user config. Keep the old file until the new one has
  been written and read back. Done: the old file is only renamed after the
  read-back.
- Autostart and the updater do not depend on the config path. Checked.
- Finishing and rerunning onboarding still call `saveConfig`/`resetConfig`
  directly, outside the queue. They run from buttons with no debounced edits
  pending in practice, so they were left as they are; item 08 rewrites this
  code.
