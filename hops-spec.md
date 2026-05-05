# Hops — Product Spec

> A lightweight Windows tray app that intercepts every link click and routes it to the right browser — automatically or via a picker.

---

## The core idea

When you click a link outside a browser (in Slack, VS Code, a PDF, an email client, a terminal...), the OS needs to hand it to _something_. Normally that's your default browser. **Hops registers itself as the default browser** so it receives every URL first, then decides what to actually open based on your rules and preferences.

Hops is **not a browser**. It never renders web content. It just inspects the URL and spawns the right browser process with that URL as an argument.

---

## Routing logic

When a URL arrives, Hops checks these steps in order. The first match wins.

```
1. Always ask enabled?          → show picker
2. URL matches a rule?          → open in rule's browser (if running, or if "use when offline" is on)
3. Default browser configured?  → open there (same running check)
4. Nothing matched              → show picker
```

If a rule matches but the target browser is not running, and "use defaults when not running" is **off**, Hops falls through to the picker so you can consciously choose which profile or browser to start.

---

## Picker window

The picker is a small floating frameless window that appears centred on screen. It is always on top. It closes the moment it loses focus (click anywhere outside = abort, URL is not opened).

### Layout

```
┌─────────────────────────────────────┐
│ https://github.com/org/repo   📋 ✏️  │  ← header: url + copy + edit
├─────────────────────────────────────┤
│  Google Chrome          [default]   │  ← bold   = currently running
│  Mozilla Firefox              🛡    │  ← bold   = running, shield = private mode
│  Brave                        🛡    │  ← normal = NOT running
│  Microsoft Edge               🛡    │
├─────────────────────────────────────┤
│ ☐ Always ask            Settings → │  ← footer
└─────────────────────────────────────┘
```

- **Bold name** = browser process is currently running
- _Normal name_ = browser is installed but not running
- **🛡 shield button** = open in private/incognito mode (only shown for browsers that support it)
- **[default]** badge on the configured default browser
- **📋 Copy** — copies the URL to clipboard and closes the window
- **✏️ Edit** — turns the URL into an editable input field; press Enter or click a browser to open the edited URL; press Esc to cancel edit

### Keyboard shortcuts

| Key                   | Action                                                          |
| --------------------- | --------------------------------------------------------------- |
| `Esc`                 | Abort — close without opening anything                          |
| `Alt` (hold)          | All browser buttons switch to private mode — a hint bar appears |
| `Alt` + click browser | Open in private/incognito mode                                  |
| Click outside window  | Abort — close without opening anything                          |

---

## URL matching rules

Rules are the heart of Hops. Each rule has a **pattern**, a **pattern type**, a **target browser**, and an optional **private mode** flag. Rules are checked in the order you define them — first match wins.

### Pattern types

#### 1. Hostname

Matches the domain only, ignoring protocol, path, and query string. The most common and safest type to use.

```
Pattern:  github.com
Matches:  https://github.com/anything
          http://github.com
Does NOT: https://notgithub.com
          https://sub.github.com   ← use *.github.com for subdomains
```

#### 2. Hostname + subdomains (wildcard prefix)

Use `*.` prefix to match a domain and all its subdomains.

```
Pattern:  *.notion.so
Matches:  https://www.notion.so
          https://myworkspace.notion.so/page/123
Does NOT: https://notion.so         ← add a second rule for the root if needed
```

#### 3. Prefix

Matches any URL that starts with the given string. Useful for specific paths or apps.

```
Pattern:  https://linear.app/myteam
Matches:  https://linear.app/myteam/issue/ENG-123
          https://linear.app/myteam/projects
Does NOT: https://linear.app/otherteam
```

#### 4. Contains

Simple case-insensitive substring match anywhere in the URL. Quick to set up, but can be too broad — prefer Hostname when possible.

```
Pattern:  figma
Matches:  https://www.figma.com/file/abc
          https://figma.com
          https://anything.com?redirect=figma.com   ← unintended match, be careful
```

#### 5. Full URL

Exact match. Useful for pinning one specific URL to a browser (e.g. an internal dashboard).

```
Pattern:  https://app.datadoghq.com/dashboard/abc-123
Matches:  https://app.datadoghq.com/dashboard/abc-123
Does NOT: https://app.datadoghq.com/dashboard/abc-123?tab=metrics
```

#### 6. Glob

Shell-style wildcards. `*` matches anything (including `/`). `?` matches one character. Case-insensitive.

```
Pattern:  *.mycompany.com/*
Matches:  https://app.mycompany.com/login
          https://docs.mycompany.com/guide/setup

Pattern:  https://jira.*/browse/ENG-*
Matches:  https://jira.mycompany.com/browse/ENG-512
```

#### 7. Regex

Full regular expression match against the entire URL. Most powerful, but write carefully.

```
Pattern:  ^https?://(www\.)?youtube\.com/watch
Matches:  https://www.youtube.com/watch?v=abc
          http://youtube.com/watch?v=xyz

Pattern:  ^https://mail\.google\.com/mail/u/[01]/
Matches:  https://mail.google.com/mail/u/0/#inbox    ← Google account 1
          https://mail.google.com/mail/u/1/#inbox    ← Google account 2
Does NOT: https://mail.google.com/mail/u/2/          ← account 3, different browser
```

> **Tip:** Regex errors (invalid pattern) should be caught in the UI at save time and shown inline — never crash silently.

### Rule ordering

Rules are evaluated top-to-bottom. Drag-to-reorder in the UI. More specific rules should go above broader ones.

```
✓ Correct order:
  1. https://mail.google.com/mail/u/0   → Chrome (work account)
  2. google.com                          → Firefox

✗ Wrong order — rule 2 will never match because rule 1 already catches it:
  1. google.com
  2. https://mail.google.com/mail/u/0
```

---

## Settings

### Browsers tab

- Lists all detected + manually added browsers
- **Refresh** button — re-scans the system for installed browsers
  - Detection strategy: check known install paths first, then fall back to `HKEY_LOCAL_MACHINE\SOFTWARE\Clients\StartMenuInternet` registry key
- **Add manually** — paste any `.exe` path and give it a name (for portable browsers, nightly builds, etc.)
- **Remove** — hide a browser from the picker (doesn't uninstall)
- Each browser shows its name and full executable path

#### Known browsers and their private mode flags

| Browser | Private flag      |
| ------- | ----------------- |
| Chrome  | `--incognito`     |
| Firefox | `-private-window` |
| Edge    | `--inprivate`     |
| Brave   | `--incognito`     |
| Opera   | `--private`       |
| Vivaldi | `--incognito`     |

#### Browser profiles (known complexity)

Chrome, Brave, and Edge support multiple profiles. When a user has 2 Brave profiles and clicks Brave, the OS may show a profile selector — this is browser behaviour Hops cannot control. A future improvement: detect profile directories from `%LOCALAPPDATA%\<browser>\User Data\`, list them as separate entries, and launch with `--profile-directory=<name>`.

---

### Settings tab

| Option                        | Default | Description                                                                                                                                            |
| ----------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Always show picker            | Off     | Skips all rules and always shows the picker window                                                                                                     |
| Default browser               | None    | Browser to use when no rule matches                                                                                                                    |
| Use defaults when not running | Off     | When off, shows picker if the target browser isn't already open. When on, always opens in the configured browser even if it needs to be launched cold. |
| Turn off transparency         | Off     | Replaces the frosted glass picker background with a solid dark colour — helps legibility on some setups                                                |

---

### Defaults tab (Rules manager)

- List of all rules in evaluation order
- Drag-to-reorder
- Add rule form: pattern input + type selector + browser selector + private mode toggle
- Pattern type selector: Hostname / Hostname+subdomains / Prefix / Contains / Full URL / Glob / Regex
- Inline validation: regex patterns are validated on save, error shown next to the field
- Delete button per rule

---

## System tray

Hops lives in the system tray and never shows a window on startup.

- **Left click tray icon** → open Settings
- **Right click tray icon** → context menu: Settings / Quit
- The app must stay running at all times in the background with minimal resource use — this is the core constraint

---

## Registering as default browser

Hops writes to:

- `HKEY_CURRENT_USER\Software\Classes\Hops` — registers the URI handler
- `HKEY_CURRENT_USER\Software\Hops\Capabilities` — application metadata
- `HKEY_CURRENT_USER\Software\RegisteredApplications` — makes Hops appear in "Default Apps"

After writing registry keys, Hops opens the Windows **Default Apps** settings page (`ms-settings:defaultapps`) because Windows 10/11 requires the user to confirm the change there manually — this is a Windows security requirement and cannot be bypassed.

The Settings UI shows a prominent **"Set as Default Browser"** button and a green confirmation badge once Hops is confirmed as default. After the user goes through the flow, check if the association was actually saved.

Config is saved to: `%APPDATA%\Hops\config.json`

---

## Performance requirements

Hops is in the critical path of every link click. Any noticeable delay between clicking a link and the browser opening (or picker appearing) will feel broken.

| Scenario                           | Target     |
| ---------------------------------- | ---------- |
| URL → browser launch (rule match)  | < 150ms    |
| URL → picker window visible        | < 100ms    |
| App startup (tray ready)           | < 500ms    |
| Memory footprint (idle, tray only) | < 30MB RAM |
| Rule matching (1000 rules)         | < 5ms      |

### To achieve this:

- **Config is loaded on URL arrival**, not kept in memory permanently — file is small, read is fast, avoids stale state
- **Browser detection** only runs on refresh or first launch, result is cached in config
- **Running browser check** (`tasklist`) is called on every URL arrival — it's fast (~20ms) but if it becomes a bottleneck, replace with WMI or a process snapshot API
- **No heavy framework startup in the picker path** — the picker window should render with data already available, not show a loading state
- **Regex rules** are compiled on every URL arrival — if rule count grows large, consider caching compiled `Regex` objects in memory keyed by pattern string

---

## Future ideas (not in v1)

- **Browser profiles** — detect and list Chrome/Brave/Edge profiles as separate launchable targets
- **macOS support** — register via `Info.plist` LSHandlerURLScheme, same JS/Rust codebase
- **Linux support** — register via `.desktop` file + `xdg-mime`
- **Rule import/export** — share your ruleset as a JSON file
- **Usage stats** — small local counter of which browser is used most, visible in settings
- **Temporary override** — hold a key while clicking a link to force the picker, even when a rule would auto-route
- **Rule testing tool** — paste a URL in settings and see which rule would match it (and which browser would open)

---

## Tech stack

| Layer         | Choice                           | Why                                                                                        |
| ------------- | -------------------------------- | ------------------------------------------------------------------------------------------ |
| UI            | React + TypeScript + tailwindcss | Familiar JS ecosystem, component model fits the tabbed settings UI                         |
| Desktop shell | Tauri 2.0                        | Much smaller binary than Electron (~5MB vs ~100MB), Rust backend handles OS calls natively |
| Styling       | Plain CSS with CSS variables     | No build complexity, full control                                                          |
| Config        | JSON file on disk                | Human-readable, easy to back up, no database dependency                                    |
| Windows API   | `winreg` Rust crate              | Registry reads/writes for browser registration and detection                               |

---
