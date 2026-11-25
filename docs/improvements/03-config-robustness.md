# 03 — Config robustness

## Context and problem

### A. An uninstalled browser blocks every save

`save_config` → `save_config_internal` → `validate_config` (`config.rs:227`)
calls `validate_browser_for_launch` for **every** browser (`config.rs:236`),
and that requires the executable to exist (`browsers.rs:387`).

If the user uninstalls a browser that is still in the config, every save
fails with "Browser executable does not exist", even for unrelated changes
such as the theme or a rule. The only fix is to delete the browser first, and
detected browsers cannot be deleted from the UI, only hidden.

`validate_config` is also called by `preview_route_with_config` and
`route_and_open_with_config` (`commands.rs:123`, `commands.rs:135`), so the
route tester in settings fails the same way.

"Refresh browsers" (`refresh_browsers`, `commands.rs:91`) already drops
detected browsers whose exe is gone: detection skips missing files and
`merge_detected_browsers` replaces the whole detected list. It saves without
validation, and `normalize_config` then silently deletes every rule that
pointed at the dropped browser and clears the default browser if it was that
one (`config.rs:216-224`). Manual browsers are never dropped by refresh.

When automatic routing picks a browser that fails to launch,
`handle_incoming_url` returns the error and the caller only `eprintln!`s it
(`lib.rs:78`, `lib.rs:111`). The packaged app has no console, so the link is
lost with no feedback. The picker path shows the error in its status line
(`PickerWindow.tsx:145`).

### B. A corrupt config makes the app unusable

`load_or_init_config_from_path` (`config.rs:147-151`) returns an error when
`config.json` is not valid JSON (for example after a bad hand edit). Every
command that loads the config fails, link routing stops, and the settings
window shows "Could not load configuration" with no way out except fixing the
file by hand. A link that arrives while the file is broken fails before
routing and is lost silently, for the same `eprintln!` reason as above.

`serde_json::from_str::<AppConfig>` fails the same way for syntax errors, an
empty file, a missing required field, a wrong type, and an unknown enum value
(e.g. `"patternType": "wildcard"`). Checks after parsing are already lenient:
`normalize_config` drops dangling rules and default browser, and an invalid
regex in the file just never matches (`compiled_regex`, `routing.rs:288`).
Duplicate browser paths in the file are not checked on load.

## Open questions

1. **A.** Where should "executable exists" be checked? Options:
   - only at launch (`open_url_with_browser`), with a clear error;
   - at save, but only for browsers that changed compared to the stored config;
   - at save as a warning, not an error.
2. **A.** Should a missing executable mark the browser in the UI (e.g. a
   "missing" badge) and skip it in the picker? Should "Refresh browsers" drop
   detected browsers that no longer exist?
3. **B.** On invalid JSON: rename to `config.json.bak` (or timestamped) and
   start from defaults with detected browsers? Or keep the app in a "config
   broken" state with a button to reset or open the folder?
4. **B.** Should the user be notified (status banner, Windows notification)?
   Link routing may run with no window open.
5. **B.** Also handle JSON that parses but fails semantic checks (unknown
   enum values, etc.), or only syntax errors?

## Decisions

1. **"Executable exists" is checked only at launch.** `validate_browser_for_launch`
   is split: the path-only checks (empty path, `.exe`, Hops itself, private
   flag, item 02 trust) still run at save, and the filesystem check runs only
   when launching. A missing browser no longer blocks unrelated saves or the
   route tester. New manual browsers are still checked for existence when
   added (`validate_manual_browser_request`), so nothing is lost there.
2. **A launch failure falls back to the picker.** When automatic routing picks
   a browser that cannot be launched (missing exe, or any other launch error),
   Hops shows the picker with the reason (`RouteReason::BrowserLaunchFailed`
   plus a notice line) instead of losing the link. The notice ends with how
   to fix it (open Settings from the tray and remove or replace the browser),
   since nothing is removed automatically and every link routed there keeps
   landing in the picker until the user does. Browsers whose exe is
   missing are left out of the picker, and the Browsers tab shows a "missing"
   badge (checked when the tab opens, like the running state).
   "Refresh browsers" keeps dropping detected browsers that are gone, but now
   says how many rules were removed with them instead of deleting them
   silently.
3. **Invalid config: back up and recover automatically.** The broken file is
   renamed to `config.json.broken-<unix seconds>` (a counter is added if that
   name is taken, so a second failure never overwrites the first backup), and
   Hops starts from defaults plus detected browsers, with onboarding marked
   as done, since the user is not new. Hops keeps routing links; the old
   rules can be restored from the backup.
4. **Tell the user both ways.** A Windows notification is shown from Rust when
   the recovery happens, because no window may be open. The settings window
   also shows a one-time warning in the status banner with the backup path
   (`take_config_recovery_notice`, cleared once read).
5. **Every serde error counts as a broken config**: syntax errors, an empty
   file, missing fields, wrong types and unknown enum values all go through
   the recovery in (3). A lenient per-entry parse is a possible follow-up;
   the backup keeps everything in the meantime.

## Proposal

- **A.** Split validation into structural checks (ids, references, regex,
  duplicate paths) at save, and filesystem checks at launch. Surface missing
  executables in the browser list.
- **B.** On parse failure: back up the broken file, recover with defaults plus
  detected browsers, and report the recovery through a field the frontend can
  show once.
- Unit tests for both paths using temp directories.

## Out of scope

- Config location and the refresh race (item 05).
- Trust rules inside `validate_browser_for_launch` (item 02).

## Acceptance criteria

- [x] With a configured browser whose exe was deleted, saving unrelated
      settings succeeds.
- [x] Opening a link with that browser gives a clear error (and the decided
      fallback, e.g. the picker).
- [x] A syntactically broken `config.json` is backed up and Hops keeps working;
      the user is told what happened.
- [x] Tests cover both scenarios.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml --lib`
- `bun run build`
- Manual: add a manual browser pointing to a copied exe, delete the exe, change
  the theme. Then corrupt `config.json` and click a link.

## Notes and risks

- Moving the existence check to launch time must not weaken item 02's trust
  checks; those still run at launch.
- `validate_browser_for_launch` (`browsers.rs:364`) was also changed by item 02.
