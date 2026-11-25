# 02 — Manual browser trust model

Do after 01, which removes the easiest way to abuse this.

## Context and problem

Manual browsers have a trust level (`ManualBrowserTrust`: `Verified` or
`UserConfirmed`). An unrecognized executable needs explicit user confirmation
before Hops launches it (`validate_browser_for_launch`, `browsers.rs:356-404`).
In practice that gate is easy to get around, and some of the code around it is
dead.

1. **The display name decides recognition.** `resolve_browser_metadata`
   (`browsers.rs:489-498`) checks the user-supplied name first: any `.exe`
   named "Google Chrome" becomes a known Chrome, gets `Verified` trust, and
   skips confirmation. `is_known_browser_match` (`browsers.rs:665-670`) does
   the same.
2. **Substring matching on a haystack.** `classify_browser_path`
   (`browsers.rs:564-631`) builds `"{path} {name hint} {exe stem}"` and looks
   for tokens with `contains`. Short tokens match unrelated paths: `"arc"`
   matches `C:\Users\Marcus\...` or `search.exe`, `"zen"` matches
   `citizen.exe`, `"chrome"` matches any folder containing it.
3. **Dead branch.** `validate_private_flag_value` (`browsers.rs:735-739`)
   returns `Ok(None)` in both arms of an `if`, so its `manual_trust` parameter
   has no effect, and `validate_private_flag_for_request`
   (`browsers.rs:689-703`) only exists to compute that unused value.

The trust level also controls which private-mode flag is allowed, so wrong
recognition can mean a wrong or missing private flag.

### Re-verified after 01 (commit `61e595d`)

Line references above are still correct; 01 did not touch `browsers.rs`.
Two facts the original review missed:

4. **`UserConfirmed` is asserted by the client.** The backend never shows the
   confirmation itself. The frontend sends `allowUserConfirmed: true` to
   `validate_manual_browser`, gets `UserConfirmed` back, and later sends it
   inside `save_config`; `validate_config` (`config.rs:236`) accepts whatever
   `manualTrust` the payload carries. Anything that can call `save_config`
   (the main window, after 01) or write the config file can mark any `.exe` as
   confirmed, however strict recognition becomes.
5. **`Verified` is never read for a decision.** Every launch and save
   recomputes recognition (`browsers.rs:383`), and
   `normalize_manual_browser_entries` (`browsers.rs:92-119`) overwrites the
   stored value on load. The only reader is the badge in
   `BrowsersTab.tsx:56-61`. `Known` vs `RecognizedFamily` likewise only changes
   the message text; the frontend never reads `recognition`.

The false positives also apply to the whole path, not only the file name:
with a Windows user called Marcus, every manual `.exe` under
`C:\Users\Marcus\` is classified as Chromium, skips confirmation, and is
launched with `--incognito` in private mode.

## Open questions

1. **What is trust for?** (a) A UX safeguard against picking the wrong file
   by mistake, or (b) a real boundary that resists a tampered config or a
   compromised webview? This decides everything below.
2. If (a): accept that it is bypassable, simplify, and document it as such?
3. If (b): recognize only by executable file name (exact match, no
   substrings) and/or known install path suffix, and ignore the display name
   entirely? Consider signature/publisher checks (Authenticode) too heavy?
4. How should existing configs be migrated? A manual browser that is
   `Verified` today might become unrecognized and need confirmation again.
   Silent downgrade with a prompt on next launch, or keep existing trust?
5. Should `Verified` vs `UserConfirmed` stay as two values, or collapse into a
   single "confirmed" flag plus "recognized family" metadata?

## Decisions

1. **Trust is a UX safeguard (a), not a security boundary.** `UserConfirmed`
   is asserted by the client, and the config file sits in `%APPDATA%`, where
   anything running as the user can write it; such a process can already run
   whatever it wants, so protecting the file gains nothing. This is how other
   desktop apps (and other browser pickers) treat user-scoped config too. The
   goal is to stop Hops from misidentifying executables.
2. **Strict recognition.** A browser is recognized only by an exact executable
   name (`executable_aliases`) or a known install path suffix. The display
   name is only shown, and the substring "haystack" matching is removed.
   Accepted cost: renamed portable browsers (e.g. `FirefoxPortable.exe`) now
   need confirmation and get no private flag. Letting the user pick a family
   when confirming is a possible follow-up, not part of this item.
3. **No Authenticode/publisher checks**: too heavy for what (1) needs.
4. **Migration: keep existing entries working.** A manual browser stored as
   `Verified` that is no longer recognized becomes `UserConfirmed` on load,
   and loses its private flag, since its family is no longer known. The user
   picked that executable on purpose; dropping the flag avoids passing
   `--incognito` to something that may not be a browser.
5. **Keep both trust values** to keep the change small and the config format
   unchanged. `Verified` stays a derived badge, recomputed on every load.
   `BrowserRecognition::RecognizedFamily` is removed: without name
   recognition it meant the same as `Known`.

## Proposal

Depends on question 1. Likely direction (b):

- Recognition from the executable file name and known install path suffixes
  only, exact matches against a table. The name hint is used for display only.
- Remove the haystack/token approach, or restrict tokens to whole path
  segments.
- Delete the dead branch and the `manual_trust` parameter.
- Tests for false positives (`Marcus`, `citizen.exe`) and for the
  name-spoofing case.

## Out of scope

- The general family-table refactor of `classify_browser_path` and
  `resolve_browser_metadata` (item 07), though this item may already reshape
  that code. If so, update item 07 to say what is left.
- Command ACL (item 01).

## Acceptance criteria

- [x] A random `.exe` named after a known browser still requires confirmation.
- [x] Paths containing `arc`, `zen`, `chrome` as substrings of unrelated words
      are not recognized as browsers.
- [x] No unreachable branch or unused parameter remains in the trust code.
- [x] Existing configs load and behave as decided in question 4.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml --lib` (new tests for the
  cases above).
- Manual: add a portable Chrome, Firefox and an unknown `.exe`; confirm the
  prompts and private flags match expectations.
- Manual: load a config from before the change.

## Notes and risks

- Stricter recognition can break legitimate portable or renamed browsers;
  the confirmation flow is the fallback.
- `browsers.rs:356` is also touched by item 03.
