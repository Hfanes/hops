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

Claims re-verified against the code on 2026-11-25: the Tauri docs confirm that
every registered command is allowed for every window unless
`AppManifest::commands` is used. The picker only invokes the three commands
listed above, and nothing calls `preview_route`, `route_and_open` or
`src/api.ts`.

1. **Capabilities only, no runtime window-label checks.** Tauri rejects
   the call before it reaches Rust. Label checks inside the commands would
   repeat the same code in several places and tie the commands to window
   names.
2. **Narrow the plugin permissions in this item.** `process:default` →
   `process:allow-restart` (the frontend only calls `relaunch()`).
   `notification:default` → `allow-is-permission-granted`,
   `allow-request-permission` and `allow-notify`, the only commands that
   `isPermissionGranted`, `requestPermission` and `sendNotification` reach
   through the plugin's init script in 2.3.3. `updater:default` stays: it is
   already close to what is used. Least privilege is Tauri's own
   recommendation and it is the same question this item answers. Risk: a
   future plugin version could call another command from its init script and
   the notification would fail silently, so check notifications whenever the
   plugin is updated.
3. **Remove** `preview_route`, `route_and_open`, their wrappers
   `previewRoute` / `routeAndOpen`, and `src/api.ts`. A future CLI would call
   the Rust functions directly, not Tauri commands, and git history keeps the
   code.
4. **Check the URL against the picker session.** `open_url` rejects the
   request unless `request.url` equals the URL stored in
   `PickerState.session`. The picker always sends `session.url` unchanged, so
   an exact comparison is enough and the frontend API stays the same. We
   rejected dropping `url` from the request and reading it from the session
   in Rust: if a new link replaced the session between render and click, that
   would open the new URL in the browser picked for the old one, while the
   check fails visibly instead.

## Proposal

- `build.rs`: `tauri_build::try_build(Attributes::new().app_manifest(
  AppManifest::new().commands(&[...])))` listing every command.
- `capabilities/default.json` (main): add `allow-<command>` permissions for
  the settings commands.
- `capabilities/picker.json`: add only `allow-get-picker-state`,
  `allow-hide-picker-window`, `allow-open-url`.
- Remove the dead commands, wrappers and `src/api.ts`.
- `capabilities/default.json`: replace `process:default` and
  `notification:default` with the narrow permissions from decision 2.
- `open_url`: reject URLs that do not match the current picker session.
- The command list lives in `src-tauri/app_commands.rs`, included by
  `build.rs` and by `src/capability_tests.rs`. The tests check that every
  command is granted to some window, that the picker has exactly its three
  commands, and that no command is granted to both windows. They cannot see
  `invoke_handler`, so a command registered there but missing from the list
  is still only caught by walking through the screens.

## Out of scope

- Changing what any command does (except question 4, if decided).
- The manual browser trust model (item 02).

## Acceptance criteria

- [ ] Invoking a settings-only command from the picker window fails with a
      permission error.
- [x] Settings window and picker work exactly as before.
- [x] No unused command or wrapper remains; `src/api.ts` is gone.
- [x] `open_url` with a URL other than the picker session's is rejected.
- [ ] Update notifications and restart after an update still work. Needs a
      signed release, so it is checked on the first release after the merge.

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
