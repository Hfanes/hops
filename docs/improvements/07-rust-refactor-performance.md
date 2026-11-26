# 07 — Rust refactor and performance

Do after 02, 04 and 05, which change the same files. Read their decisions first; some points below may already be done.

Re-verified on 2026-11-25 against `ee233e9`, after items 01 to 06. Line
references below point at that commit.

## Context and problem

### Duplication in `browsers.rs`

- ~~`classify_browser_path`: four almost identical family-token blocks.~~
  Done in item 02, which removed the token matching entirely.
- `resolve_browser_metadata` (`browsers.rs:505-552`): builds the same
  `ResolvedBrowserMetadata` from a definition twice (it was three times
  before item 02).
- ~~`validate_browser_for_launch` and `validate_manual_browser_request` repeat
  four checks.~~ Since item 03, `validate_browser_for_launch`
  (`browsers.rs:373-385`) only adds the on-disk check on top of
  `validate_browser_entry` (`browsers.rs:390-431`). What is left:
  `validate_browser_entry` and `validate_manual_browser_request`
  (`browsers.rs:433-499`) repeat three checks (empty path, `.exe` suffix, not
  Hops) with different error messages, so a shared helper has to keep each
  caller's wording. The existence checks also differ on purpose: the manual
  request uses `exists()`, the launch uses `is_file()`.
- `KnownBrowserDefinition::private_flag_override` is `None` in every
  definition (`browsers.rs:655-777`), so it is dead weight.
  `use_family_private_flag` is `false` only for Tor Browser.

### Duplication in `picker.rs`

- `handle_incoming_url` (`picker.rs:639-652`): the `CtrlShiftClick` branch is
  redundant. With `force_picker` the route decision never loads the running
  process snapshot (`picker.rs:456-458`), so the generic `else` branch would
  do exactly the same. Still true.
- `show_picker_window` and `show_picker_window_with_running_processes`
  (`picker.rs:465-517`), and `build_picker_session` and its
  `_with_running_processes` twin (`picker.rs:340-413`), duplicate each other.
  The plain versions can call the `_with_running_processes` ones with
  `current_running_processes`. `show_picker_window` is also called from the
  `show_picker_for_url` command (`commands.rs:215`).
- Three functions carry `#[allow(clippy::too_many_arguments)]` with the
  comment "Splitting these arguments into a struct is part of item 07"
  (`picker.rs:361`, `463`, `488`). This file did not list it; it belongs with
  the picker cleanup (for example a `PickerRequest { url, source, reason,
  preferred_browser_id, preferred_private_mode }`).

### Test helpers copied across modules

`test_app_config` is in `browsers.rs`, `routing.rs`, `picker.rs` (and an
inline copy in `config.rs:840`). `manual_browser` is in `routing.rs` and
`picker.rs`, but the copies differ: `routing.rs` sets `icon_key` to
`"generic"`, `picker.rs` to the browser ID, so a shared builder needs the icon
key as a parameter or the tests must not depend on it. `test_rule` is also in
both, with different signatures. `temp_executable_path` is in `browsers.rs`
and `config.rs`. Temp files are cleaned up manually at the end of each test
and leak when an assertion fails (`config.rs` already has a small `cleanup`
helper and some `remove_dir_all` calls, but no `Drop` guard).

### Shared constant

`PICKER_SESSION_EVENT` is defined in `lib.rs:29` and again in
`src/features/picker/PickerWindow.tsx:9`, with nothing tying them together.

### Performance (small, but on the link-click hot path)

- **Idle destroy thread** (`picker.rs:245-282`): every picker hide calls
  `thread::spawn` + `sleep(15s)`. Rapid show/hide cycles pile up sleeping OS
  threads. The token makes this correct, just wasteful. Only hides of a
  visible picker schedule one (`picker.rs:317-325`).
- **Main thread work**: the single-instance callback (`lib.rs:75-84`) runs
  `handle_incoming_url` (config stat, process scan, `spawn`) on the main
  thread. Usually fast, but a busy disk freezes the UI. Confirmed in
  `tauri-plugin-single-instance` 2.4.2 (`platform_impl/windows.rs`): the
  second instance sends `WM_COPYDATA` with `SendMessageW`, which blocks until
  the resident window procedure has run the callback, and only then exits. So
  the clicked-link process stays alive until the picker is shown, which may
  matter for the focus permission from `allow_other_instance_to_take_focus`
  (`lib.rs:55-61`).
- **Repeated normalization**: `resolve_browser_metadata` calls
  `normalize_path(suffix)` for every known suffix on every call
  (`browsers.rs:518`), and `normalize_config` reaches it up to three times
  per browser (two `hydrate_*` passes and `normalize_manual_browser_entries`)
  on each uncached load. `is_known_browser_match` no longer exists.
- **Needless clone**: `detect_browsers_from_known_paths`
  (`browsers.rs:225-227`) clones the whole `seen_paths` set.
- **Handles**: `running_processes` (`browsers.rs:853-889`) closes the snapshot
  handle manually, and so does `query_process_image_path`
  (`browsers.rs:910-929`) for each process handle; an RAII guard is safer if
  either ever gains an early return.

## Open questions

1. Idle destroy: async task (`tauri::async_runtime::spawn` + `tokio` sleep),
   a single reusable timer thread, or drop destroy entirely and keep a hidden
   picker webview? (Memory vs. latency; `AGENTS.md` favors memory.) Note:
   `tokio` is not a direct dependency. Tauri 2.11.2 does not enable its
   `time` feature; it is on today only through another crate, so using
   `tokio::time` needs a direct `tokio` entry with `features = ["time"]` (no
   new crate in the lockfile).
2. Move `handle_incoming_url` off the main thread? Window operations must
   still happen on the main thread. Is the gain worth the complexity?
3. Shared constant: generate it, or just add a test or comment that links the
   two definitions?
4. Split `browsers.rs` (1734 lines, ~760 of them tests) into
   `browsers/{detect,classify,validate,process}.rs`, or keep one file after
   the dedup?

## Decisions

1. **Idle destroy becomes a Tokio task, and only one is pending at a time.**
   `schedule_picker_idle_destroy` starts it with `tauri::async_runtime::spawn`
   and waits with `tokio::time::sleep`, so no OS thread sleeps. `PickerState`
   keeps the task's `JoinHandle`, and both scheduling a new destroy and
   `cancel_picker_idle_destroy` call `abort()` on the previous one first.
   The token check on the main thread stays: a task that already finished its
   sleep and queued the destroy with `run_on_main_thread` cannot be aborted,
   and the token stops it from destroying a picker that was just shown again.
   `tokio` becomes a direct dependency, pinned to the locked version
   (`=1.52.1`) with `features = ["time"]`. Today `time` is only on because
   `reqwest` (through the updater) enables it; Tauri does not. No new crate
   enters the lockfile. Keeping the webview alive was rejected for its
   resident memory.
2. **`handle_incoming_url` stays on the main thread.** The second instance
   blocks in `SendMessageW` until the callback returns, so today the
   clicked-link process is still alive when the picker takes focus. Moving the
   work off the main thread would let it exit first and risks bringing back
   the unfocused picker that `allow_other_instance_to_take_focus` fixed, a bug
   only a packaged build shows. The gain (no UI freeze on a very slow disk)
   is small, since the config is cached and the process scan is narrow.
3. **`PICKER_SESSION_EVENT`: a test plus comments.** A Rust test reads
   `src/features/picker/PickerWindow.tsx` with `include_str!` and checks that
   it uses the same event name, the way `capability_tests.rs` already embeds
   the capability files.
   Each definition gets a comment pointing at the other. Generating the
   constant is too much tooling for one string.
4. **Split only the process code into `browsers/process.rs`.** It is the one
   clean seam: it uses nothing from the rest of `browsers.rs` but
   `normalize_path` and `BrowserConfig`, nothing else calls into it, and it
   holds almost all the `unsafe` Win32 code plus the new RAII guard. A
   `pub(crate) use process::{...}` in `browsers.rs` keeps every import in
   `commands.rs`, `picker.rs` and `routing.rs` unchanged, and its tests move
   with it. A full split was rejected: classification is used by detection,
   validation and config normalization, so about eight private helpers would
   need `pub(super)` and the tests cross those boundaries. What stays in
   `browsers.rs` (about 800 lines of code) is in line with `config.rs` and
   `picker.rs`. Done as its own commit, with no other change in it.

## Proposal

- A `metadata_from(definition)` helper (the family token table is gone since
  item 02).
- A shared helper for the empty / `.exe` / not-Hops checks of
  `validate_browser_entry` and `validate_manual_browser_request`, keeping
  each caller's error messages.
- Drop the unused `private_flag_override` field.
- Collapse the picker twins; delete the `CtrlShiftClick` branch; replace the
  long argument lists with a request struct and remove the
  `too_many_arguments` allows.
- `#[cfg(test)] mod test_support` with shared builders and a `TempDir` guard
  that cleans up on `Drop`.
- Pre-normalized known suffixes (`LazyLock`), no clone in known path
  detection, RAII handle guard for the snapshot and process handles.
- Idle destroy and main-thread changes as decided.

## Out of scope

- Behavior changes. Every existing test must pass unchanged, except for moved
  helpers.
- Trust semantics (item 02), registration (item 04), config location (item 05).

## Acceptance criteria

- [ ] No behavior change; the full test suite passes.
- [ ] The duplications listed above are gone.
- [ ] Test helpers live in one module; failing tests no longer leak temp files.
- [ ] Hiding the picker repeatedly does not create one OS thread per hide.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml --lib`
- `cargo clippy --manifest-path src-tauri\Cargo.toml -- -D warnings`
- Manual, packaged build: click links (rule match, no match, Ctrl+Shift), open
  and close the picker quickly many times, check memory and thread count in
  Task Manager.

## Notes and risks

- Large mechanical diff. Keep it in reviewable commits (one per bullet).
