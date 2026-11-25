# 07 — Rust refactor and performance

Do after 02, 04 and 05, which change the same files. Read their decisions first; some points below may already be done.

## Context and problem

### Duplication in `browsers.rs`

- `classify_browser_path` (`browsers.rs:581-631`): four almost identical
  blocks for the Firefox, Edge, Opera and Chromium families. They differ only
  in the token list and the family.
- `resolve_browser_metadata` (`browsers.rs:489-524`): builds the same
  `ResolvedBrowserMetadata` from a definition three times.
- `validate_browser_for_launch` (`browsers.rs:356-404`) and
  `validate_manual_browser_request` (`browsers.rs:406-478`) repeat the same
  four checks: empty path, `.exe` suffix, not Hops, file exists.

### Duplication in `picker.rs`

- `handle_incoming_url` (`picker.rs:587-600`): the `CtrlShiftClick` branch is
  redundant. With `force_picker` the route decision never loads the running
  process snapshot, so the generic `else` branch would do exactly the same.
- `show_picker_window` and `show_picker_window_with_running_processes`
  (`picker.rs:449-498`), and `build_picker_session` and its
  `_with_running_processes` twin, duplicate each other. The plain versions can
  call the `_with_running_processes` ones with `current_running_processes`.

### Test helpers copied across modules

`test_app_config` is in `browsers.rs`, `routing.rs`, `picker.rs` (and an
inline copy in `config.rs`). `manual_browser` is in `routing.rs` and
`picker.rs`. `temp_executable_path` is in `browsers.rs` and `config.rs`. Temp
files are cleaned up manually at the end of each test and leak when an
assertion fails.

### Shared constant

`PICKER_SESSION_EVENT` is defined in `lib.rs` and again in
`src/features/picker/PickerWindow.tsx:8`, with nothing tying them together.

### Performance (small, but on the link-click hot path)

- **Idle destroy thread** (`picker.rs:237-274`): every picker hide calls
  `thread::spawn` + `sleep(15s)`. Rapid show/hide cycles pile up sleeping OS
  threads. The token makes this correct, just wasteful.
- **Main thread work**: the single-instance callback (`lib.rs`) runs
  `handle_incoming_url` (config stat, process scan, `spawn`) on the main
  thread. Usually fast, but a busy disk freezes the UI.
- **Repeated normalization**: `resolve_browser_metadata` and
  `is_known_browser_match` call `normalize_path(suffix)` for every known
  suffix on every call, and `normalize_config` runs that for every browser on
  each uncached load.
- **Needless clone**: `detect_browsers_from_known_paths`
  (`browsers.rs:217-219`) clones the whole `seen_paths` set.
- **Snapshot handle**: `running_processes` (`browsers.rs:1022-1049`) closes
  the snapshot handle manually; an RAII guard is safer if the loop ever gains
  an early return.

## Open questions

1. Idle destroy: async task (`tauri::async_runtime::spawn` + `tokio` sleep,
   no new dependency since Tauri ships tokio), or drop destroy entirely and
   keep a hidden picker webview? (Memory vs. latency; `AGENTS.md` favors
   memory.)
2. Move `handle_incoming_url` off the main thread? Window operations must
   still happen on the main thread. Is the gain worth the complexity?
3. Shared constant: generate it, or just add a test or comment that links the
   two definitions?
4. Split `browsers.rs` (1726 lines, ~600 of them tests) into
   `browsers/{detect,classify,validate,process}.rs`, or keep one file after
   the dedup?

## Decisions

_Filled in during the discussion._

## Proposal

- Family token table + one loop; a `metadata_from(definition)` helper.
- `validate_executable_path(path) -> Result<&str, String>` shared by both
  validators.
- Collapse the picker twins; delete the `CtrlShiftClick` branch.
- `#[cfg(test)] mod test_support` with shared builders and a `TempDir` guard
  that cleans up on `Drop`.
- Pre-normalized known suffixes (`LazyLock`), no clone in known path
  detection, RAII handle guard.
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
