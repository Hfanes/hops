# 04 — Windows registration

## Context and problem

`register_hops_as_browser_windows` (`registration.rs:194-306`, baseline
numbering of that file) writes these keys under `HKCU`:

- `Software\Classes\HopsURL`: http/https handler. Needed.
- `Software\Classes\HopsHTML` + `FileAssociations` `.htm`/`.html`
  (`registration.rs:230-244`, `288-296`). **Broken.** The command is
  `"hops.exe" --url "%1"`. For a file, `%1` is a filesystem path, and
  `extract_url_from_args` (`picker.rs:85`) only accepts `http://` or
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

_Filled in during the discussion._

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

- [ ] `hops://anything` no longer launches Hops after registering (fresh and
      existing installs).
- [ ] `.html` behavior matches the decision; no path that just opens settings.
- [ ] After uninstall, no `Hops*` keys remain under `HKCU\Software\Classes`,
      `HKCU\Software\Hops`, or `RegisteredApplications`.
- [ ] Registration status in settings still reports correctly.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml --lib` (a test that the
  table contains no custom scheme and only `HKCU` paths).
- Manual, packaged build: register, set as default in Windows, click links,
  open an `.html` file, unregister, uninstall, and inspect `regedit`.

## Notes and risks

- Windows may refuse to list Hops as a browser without an `.html`
  association. Test option 2(a) before committing to it.
- Registry changes need `SHChangeNotify` (already done) to show up in
  Settings.
