# 01 — Command access per window

## Context and problem

`src-tauri/build.rs:1` calls plain `tauri_build::build()`. Per the Tauri 2
capabilities docs, without `AppManifest::commands` **every command registered
in `invoke_handler` is allowed for every window and webview**. The
capabilities files suggest isolation (`src-tauri/capabilities/picker.json`
grants only `core:default`), but that only scopes core and plugin permissions,
not app commands.

So the picker window, which displays URLs that come from outside, can call
`save_config`, `reset_config`, `register_hops_as_browser`, and so on.
`save_config` + `open_url` together can launch any `.exe`: save a manual
browser with `manualTrust: "userConfirmed"` pointing at it, then open a URL
with it. React escaping and the CSP (`tauri.conf.json`) make an XSS unlikely
today, so this is defense in depth, not a live exploit.

Commands each window actually uses (from `src/services/*.ts`):

- **picker** (`src/services/picker.ts`): `get_picker_state`,
  `hide_picker_window`, `open_url`.
- **main / settings** (`src/services/tauri.ts`): everything else.

Dead code found:

- `preview_route` and `route_and_open` commands (`src-tauri/src/commands.rs`)
  and their wrappers `previewRoute` / `routeAndOpen` in `src/services/tauri.ts`
  have no callers. The settings UI uses the `*_with_config` variants.
- `src/api.ts` only re-exports `./services/tauri` and nothing imports it.

## Open questions

1. Is a compile-time allow-list (`AppManifest::commands`) plus per-window
   capabilities enough, or do we also want runtime checks inside sensitive
   commands (e.g. reject `save_config` unless `window.label() == "main"`)?
2. Should plugin permissions be narrowed at the same time? For example
   `process:default` (includes exit) → `process:allow-restart` only, and
   `notification:default` → only what `useAppUpdates` needs. Or leave that for
   a later item?
3. Remove `preview_route` / `route_and_open` outright, or keep them for a
   possible CLI or debug use?
4. Should `open_url` from the picker be restricted to the URL of the current
   picker session (stored in `PickerState`) instead of any URL?

## Decisions

_Filled in during the discussion._

## Proposal

- `build.rs`: `tauri_build::try_build(Attributes::new().app_manifest(
  AppManifest::new().commands(&[...])))` listing every command.
- `capabilities/default.json` (main): add `allow-<command>` permissions for
  the settings commands.
- `capabilities/picker.json`: add only `allow-get-picker-state`,
  `allow-hide-picker-window`, `allow-open-url`.
- Remove the dead commands, wrappers and `src/api.ts` (subject to question 3).

## Out of scope

- Changing what any command does (except question 4, if decided).
- The manual browser trust model (item 02).

## Acceptance criteria

- [ ] Invoking a settings-only command from the picker window fails with a
      permission error.
- [ ] Settings window and picker work exactly as before.
- [ ] No unused command or wrapper remains; `src/api.ts` is gone (if decided).

## Verification

- `cargo check --manifest-path src-tauri\Cargo.toml`
- `cargo test --manifest-path src-tauri\Cargo.toml --lib`
- `bun run build`
- Manual, packaged build: open the picker from a link, pick a browser, press
  Esc; in settings, save a rule, refresh browsers, register/unregister, use the
  router tester.
- Manual: from the picker devtools (debug build), `invoke("save_config", ...)`
  must be rejected.

## Notes and risks

- Missing a command in the manifest or capability breaks that feature
  silently at runtime. Walk through every screen.
- Permission identifiers are generated from command names; confirm the exact
  names in `src-tauri/gen/schemas` after the first build.
