# 03 — Config robustness

## Context and problem

### A. An uninstalled browser blocks every save

`save_config` → `save_config_internal` → `validate_config` (`config.rs:227`)
calls `validate_browser_for_launch` for **every** browser (`config.rs:236`),
and that requires the executable to exist (`browsers.rs:379`).

If the user uninstalls a browser that is still in the config, every save
fails with "Browser executable does not exist", even for unrelated changes
such as the theme or a rule. The only fix is to delete the browser first, and
detected browsers cannot be deleted from the UI, only hidden.

### B. A corrupt config makes the app unusable

`load_or_init_config_from_path` (`config.rs:147-151`) returns an error when
`config.json` is not valid JSON (for example after a bad hand edit). Every
command that loads the config fails, link routing stops, and the settings
window shows "Could not load configuration" with no way out except fixing the
file by hand.

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

_Filled in during the discussion._

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

- [ ] With a configured browser whose exe was deleted, saving unrelated
      settings succeeds.
- [ ] Opening a link with that browser gives a clear error (and the decided
      fallback, e.g. the picker).
- [ ] A syntactically broken `config.json` is backed up and Hops keeps working;
      the user is told what happened.
- [ ] Tests cover both scenarios.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml --lib`
- `bun run build`
- Manual: add a manual browser pointing to a copied exe, delete the exe, change
  the theme. Then corrupt `config.json` and click a link.

## Notes and risks

- Moving the existence check to launch time must not weaken item 02's trust
  checks; those still run at launch.
- `validate_browser_for_launch` (`browsers.rs:356`) is also changed by item 02.
