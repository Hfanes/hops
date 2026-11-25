# 04 — Windows registration

## Context and problem

`register_hops_as_browser_windows` (`registration.rs:194-306`, baseline
numbering of that file) writes these keys under `HKCU`:

- `Software\Classes\HopsURL`: http/https handler. Needed.
- `Software\Classes\HopsHTML` + `FileAssociations` `.htm`/`.html`
  (`registration.rs:230-244`, `288-296`). **Broken.** The command is
  `"hops.exe" --url "%1"`. For a file, `%1` is a filesystem path, and
  `extract_url_from_args` (`picker.rs:88`) only accepts `http://` or
  `https://`, so opening an `.html` file with Hops just shows the settings
  window. Windows also often wants a `.html` handler before it lists an app as
  a full browser.
- `Software\Classes\Hops` custom URI scheme (`registration.rs:246-263`).
  Unused: nothing in the code or README handles `hops://`. Any web page can
  trigger it and launch Hops. Attack surface with no benefit.
- `Software\Hops\Capabilities` + `RegisteredApplications`. Needed.

**Uninstall.** `tauri.conf.json` defines no NSIS `installerHooks`, so
uninstalling Hops leaves the `HKCU` keys behind. Windows keeps listing "Hops"
in Default Apps, pointing to an executable that no longer exists.

**Code shape.** Around 100 lines of repeated `create_subkey` + `set_value` +
`map_err` blocks. Since this item changes the keys anyway, the refactor to a
data table belongs here rather than in item 07.

## Open questions

1. Remove the `Hops:` custom scheme entirely? (Recommended.) Is there any
   planned use, e.g. `hops://open?url=...`?
2. `.htm`/`.html`: (a) drop the file association, (b) support it by passing
   local files to the default browser (needs `file://` handling, a new code
   path), or (c) keep the association only for Windows "browser" eligibility
   and hand the file to the default/configured browser untouched?
3. Uninstall cleanup: NSIS `installerHooks` (`NSIS_HOOK_POSTUNINSTALL`) that
   deletes the keys, or have Hops expose a `--unregister` flag that the
   uninstaller runs? The hook keeps it declarative; the flag reuses Rust code.
4. Should existing installs that already have the `Hops` scheme registered be
   cleaned up on the next registration or startup?

## Decisions

Re-verification notes (2026-11-25):

- The Tauri NSIS template can run the old uninstaller with `/UPDATE` during
  an upgrade (`$UpdateMode`). Any uninstall cleanup must skip when
  `$UpdateMode = 1`, or every update would wipe the registration and Hops
  would stop being the default browser.
- `README.md` ("Windows Registry Keys Touched") lists the keys and must be
  updated to match.
- `tauri-plugin-autostart` (via `auto-launch`) writes a value under
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, which is also left
  behind on uninstall.

1. **Custom scheme: removed.** No planned use (such as an extension calling
   `hops://open?url=...`). Today it only opens settings, and any web page can
   trigger it. If it is ever needed, it comes back with its own validation.
2. **`.htm`/`.html`: association dropped (option a).** Tested on 0.1.6
   installed: with `Capabilities\FileAssociations` and `Classes\HopsHTML`
   removed, Windows still lists Hops in Default Apps with HTTP and HTTPS and
   offers "Make Hops your default browser". Option (c) is not needed.
3. **Uninstall cleanup: NSIS `NSIS_HOOK_POSTUNINSTALL` hook**, guarded by
   `$UpdateMode <> 1`. It is declarative, does not depend on the exe still
   existing, and avoids a `--unregister` flag that the single-instance plugin
   would forward to the running tray instance. A Rust test checks that
   `hooks.nsh` deletes every root key in the registration table, so the two
   lists cannot drift. The Tauri template already deletes the autostart `Run`
   value (skipped on update); the hook also deletes the matching
   `Explorer\StartupApproved\Run` value that `auto-launch` writes.

   **Accepted trade-off:** installing a newer `.exe` over an existing install
   with the default "uninstall before installing" choice runs the full
   uninstaller without `/UPDATE` (`PageLeaveReinstall` in the template), so
   the hook removes the registration and the user must click Register and pick
   Hops in Default Apps again. The in-app updater passes `/UPDATE` and is not
   affected, and neither is choosing "do not uninstall". Decided not to handle
   or document it: few users, and updates normally go through the updater.
4. **Existing installs: an `NSIS_HOOK_POSTINSTALL` hook deletes
   `HKCU\Software\Classes\Hops`**, which runs on every install and update, and
   `register` also deletes it. No work is added at startup.

## Proposal

- Describe registration as a table of `(key path, value name, value)` and
  apply it in a loop; unregister by walking the same table.
- Drop the custom scheme (and clean it from existing installs).
- Resolve `.html` as decided.
- Add uninstall cleanup.

## Out of scope

- `HKLM` (machine-wide) registration, which `AGENTS.md` forbids without a
  design discussion.
- Changing how URL arguments are parsed, beyond what `.html` needs.

## Acceptance criteria

- [x] `hops://anything` no longer launches Hops after registering (fresh and
      existing installs).
- [x] `.html` behavior matches the decision; no path that just opens settings.
- [x] After uninstall, no `Hops*` keys remain under `HKCU\Software\Classes`,
      `HKCU\Software\Hops`, or `RegisteredApplications`.
- [x] Registration status in settings still reports correctly.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml --lib` (a test that the
  table contains no custom scheme and only `HKCU` paths).

## Notes and risks

- Windows may refuse to list Hops as a browser without an `.html`
  association. Tested: it still lists Hops (Decision 2).
- Registry changes need `SHChangeNotify` (already done) to show up in
  Settings.
