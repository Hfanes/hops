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

_Filled in during the discussion._

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

- [ ] A random `.exe` named after a known browser still requires confirmation
      (if direction (b)).
- [ ] Paths containing `arc`, `zen`, `chrome` as substrings of unrelated words
      are not recognized as browsers.
- [ ] No unreachable branch or unused parameter remains in the trust code.
- [ ] Existing configs load and behave as decided in question 4.

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
