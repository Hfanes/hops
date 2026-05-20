# Hops

Hops is a lightweight Windows tray app that routes external links to the right browser based on your rules.

Hops is not a web browser. It receives URLs, applies routing rules, and launches your chosen browser.

## Why Hops Must Be Registered In Windows Default Apps

Windows only sends external `http/https` link clicks to the current default app handler.

If Hops is not registered and selected as default for `http` and `https`, apps like Discord, Slack, terminal, and email clients will bypass Hops and open links with the current Windows default browser.

Hops writes registration under `HKCU` (current user) so:

- no admin rights are required
- rollback is simple and local to your user profile
- it avoids machine-wide (`HKLM`) side effects

## Current Features

- Settings UI (React + TypeScript)
- Automatic browser detection from Windows registry entries + known install paths
- Manual browser support with validation and one-time trust confirmation for unknown executables
- Duplicate executable-path protection for detected and manual browser entries
- Safe private-mode flag handling for supported browsers
- Per-browser `Hide from picker` support, plus delete for manual browser entries
- Rule management
- Rule ordering (first match wins)
- Rule enable/disable toggle
- Rule pattern types: hostname, hostname+subdomains, prefix, contains, full URL, glob, regex
- Routing preview and route-and-open test tools
- Picker window for manual browser choice when routing needs user input
- Background tray runtime
- Single-instance URL activation handling
- First-run onboarding flow (browser detection, registration, default-app guidance, optional Start with Windows)
- Optional Start with Windows setting
- Register / unregister Hops in Windows Default Apps catalog
- Registration status checks for `http` and `https` using the effective Windows association API, with registry fallback

## Routing Logic

Hops evaluates URLs in this order:

1. `alwaysShowPicker` enabled -> picker flow
2. First enabled matching rule:
   - if target browser is running -> open target
   - if `useDefaultsWhenNotRunning=false` and target is not running -> picker flow
   - if `useDefaultsWhenNotRunning=true` and target is not running -> fallback to configured default browser
3. Default browser fallback (same running check)
4. Otherwise -> picker flow

The picker opens a small window near the cursor when a route needs user input (CTRL + SHIFT + LEFT CLICK). Holding Alt opens supported browsers in private mode.

## Browser Detection And Manual Browser Rules

Hops detects many known browsers automatically from Windows registry entries and common install locations. That includes mainstream Chromium, Firefox, Edge, and Opera variants, plus supported forks such as Brave, Vivaldi, LibreWolf, Waterfox, Floorp, Zen, Arc, Helium, and Tor Browser.

Detected browsers and manual browsers are merged into one list. If you add a manual browser that points to the same executable path as a detected browser, the manual entry wins and the detected duplicate is suppressed.

When you add a manual browser, Hops validates the executable path and classifies it into one of these trust states:

- `verified`: the executable is recognized as a known browser or a recognized browser family
- `user confirmed`: the executable is not recognized as a supported browser, but you explicitly approved it once so Hops is allowed to launch it

If a manual browser path changes later, Hops revalidates it. A browser that was previously trusted may need confirmation again if the new path is no longer recognized.

Hops does not allow arbitrary private-mode flags for known or recognized browsers. Instead, it derives or constrains the safe flag by browser family:

- Chromium-family browsers use `--incognito`
- Firefox-family browsers use `--private-window`
- Microsoft Edge uses `--inprivate`
- Opera uses `--private`
- Tor Browser does not get an extra private-mode flag injected

Unsupported custom flags such as arbitrary profile or command-line options are rejected. Unknown executables can still be added manually, but they require an explicit confirmation first and do not accept custom private-mode flags.

Hops also blocks duplicate executable paths. You cannot add the same path twice, whether it already exists as a detected browser or as another manual entry. This keeps one trusted browser record per executable and avoids duplicate picker entries.

In the browser list, detected browsers can be hidden with `Hide from picker` without being removed. Manual browsers can also be hidden, and they can be deleted entirely if you no longer want Hops to keep them.

## Lightweight And Performance Choices

- Event-driven URL handling (no constant polling loop)
- Single-instance plugin prevents duplicate long-lived processes
- Tray-first behavior keeps UI hidden unless needed
- Optional Start with Windows launches Hops hidden in the tray after onboarding
- Windows subsystem is configured as GUI app to avoid console flashes on URL activation
- Config stored as small JSON file at `%APPDATA%\Hops\config.json`
- Browser list detection runs on demand (refresh / initial load), not continuously
- Running-process check uses fast `tasklist` snapshot at route time
- Plain config saves only write JSON; they do not trigger a running-browser rescan

## Windows Registry Keys Touched

When clicking `Register Hops`, the app writes:
HKCU - HKEY_CURRENT_USER

- `HKCU\Software\Classes\HopsURL`
- `HKCU\Software\Classes\HopsHTML`
- `HKCU\Software\Classes\Hops`
- `HKCU\Software\Hops\Capabilities`
- `HKCU\Software\RegisteredApplications` value `Hops=Software\Hops\Capabilities`

This does not automatically force system defaults. You still choose Hops in Windows Default Apps for `http` and `https`.

On first launch, Hops opens an onboarding flow that guides:

1. browser detection/manual browser add
2. registering Hops in Default Apps catalog
3. opening Windows Default Apps so you set `http` and `https` to Hops
4. choosing whether Hops starts with Windows

## Revert / Rollback

1. In Windows Default Apps, switch `http` and `https` away from Hops.
2. In Hops Settings, click `Unregister Hops`.

`Unregister Hops` removes the keys listed above from `HKCU`.

## Dev

- Install deps: `bun install`
- Run app: `bun run tauri dev`
- Frontend build: `bun run build`
- Rust check: `cargo check --manifest-path src-tauri\Cargo.toml`
- `bun run tauri dev` itself runs under a terminal-owned dev process. Validate "no extra console window" behavior with a packaged build, not only dev mode.

## Documentation Policy

Important runtime behavior changes (routing, registration, tray/background behavior, rollback path) should always be reflected in this README.

## License

Hops is licensed under the MIT License. See [LICENSE](LICENSE).
