# 08 — Frontend refactor

Do after 01 (removes unused wrappers) and 05 (changes the path normalization in `useBrowserManager.ts`).

Re-verified on 2026-11-26 against `7ef46d9`, after items 01 to 07. Line
references below point at that commit.

## Context and problem

### `src/App.tsx` (826 lines)

A god component: 18 `useState`s plus the bootstrap, onboarding, reset, rerun
onboarding, route tester, open config folder and close-window logic, and
wiring for eight hooks, six of them destructured field by field
(`App.tsx:99-234`). `SettingsTab` gets 21 props and `OnboardingFlow` 20.

Correction to the original review: these props are not passed straight
through. `SettingsTab` and `OnboardingFlow` are leaf components that use
every prop themselves; the only other components they render are
`BrowserIcon` and icons. The drilling is one level deep (App → tab). The
real cost is that App owns all of the state and logic, so every new setting
touches App, the hook and the tab.

Constants such as `shellClassName = "app-shell"` (`App.tsx:533-540`) add
indirection without value. `LoadingState`, `Topbar` and `SidebarNav` take a
`className` prop only to receive those constants.

### `src/features/browsers/useBrowserManager.ts` (730 lines)

- The `setX(current => { const next = cloneSet(current); next.add/delete(id);
  return next; })` pattern appears 18 times: 12 in `useBrowserManager.ts`
  and 6 in `useRuleManager.ts`. Six of them first check `has` and return the
  same Set when nothing changes; the other twelve always build a new one. `cloneSet` (`lib/appTypes.ts:23`) is only a
  `new Set(values)` wrapper.
- `validateManualBrowserConfig` (`useBrowserManager.ts:156-236`) repeats the
  "revert browser, then show alert or set status" block three times.
- `{ name: "", path: "", privateFlag: "" }` is written inline twice
  (`:600`, `:623`) although `EMPTY_BROWSER_DRAFT` exists.
- `validateManualBrowserConfig` is also used for detected browsers
  (`saveBrowserChanges`, `:247`), so its name is misleading.

### Repeated error formatting

`error instanceof Error ? error.message : String(error)` appears 21 times in
`src/` (App, `useAppUpdates`, `useBrowserManager`, `PickerWindow`,
`useRuleManager`, `useSystemIntegration`, `useConfigStore`). The
`error.name === "TimeoutError"` check at `App.tsx:514` is a different check
and stays.

## Open questions

1. Prop drilling: React Context for the config store and status (one
   `AppContext`, or one per concern), or keep props but group them into
   objects? Context is lighter on code; props are more explicit. Since the
   drilling is only one level deep, see the correction above.
2. How to split `App.tsx`: `useOnboarding`, `useRouteTester`,
   `useConfigActions` (reset, rerun onboarding, open folder) and an
   `OnboardingScreen` / `MainScreen` split? Any preference on naming or
   granularity?
3. `useIdSet()` hook (`{ ids, add, remove, clear }`) vs. a plain
   `withAdded` / `withRemoved` helper? The hook removes more boilerplate.
4. Is it worth adding a frontend test runner (e.g. Vitest) for the extracted
   hooks? `AGENTS.md` says there is none today; adding one is a dependency
   decision.

## Decisions

1. **Keep props, grouped into objects** (option C). The drilling is one
   level deep, so Context would hide dependencies without saving much. It
   would also re-render every consumer whenever `config` changes, which
   happens on each keystroke in an inline browser or rule edit. Grouping
   props by the hook that owns them (for example `integration`,
   `configActions`) means a new setting no longer touches App. The
   "no component receives props it only forwards" criterion already holds
   today.
2. **Split App into hooks, screens and a modal host** (option B):
   - `features/onboarding/useOnboarding.ts`: step, start-with-Windows choice,
     continue and finish.
   - `features/router/useRouteTester.ts`: input, decision, preview and open.
   - `features/settings/useConfigActions.ts`: reset, rerun onboarding, open
     config folder, the action panel.
   - `hooks/useAppBootstrap.ts`: initial load, recovery notice, config path,
     about info.
   - `hooks/useCloseGuard.ts`: `onCloseRequested` and the flush of pending
     saves.
   - `OnboardingScreen`, `MainScreen` and `AppModals` components.

   Cross-hook dependencies (reset clears the route decision and the rule and
   browser edit state; rerun resets the onboarding step) are passed in as
   callbacks. This takes the two most delicate effects out of App and leaves
   it comfortably under 300 lines.
3. **`useIdSet` hook built on pure `withAdded` / `withRemoved` helpers**
   (option A). Call sites become `pending.add(id)`, and resets become
   `clear()`. The hook updates state through functional `setState`, so calls
   from timers, async `onSuccess` / `onError` callbacks and `finally` blocks
   never act on a stale Set. Both helpers return the same Set when nothing
   changes. Today only six of the eighteen call sites do that. Applying it
   everywhere only removes useless re-renders: the tabs read the Sets with
   `.has` alone, and no effect, `useMemo` or `React.memo` depends on their
   identity.
4. **No frontend test runner in this item** (option A). `bun test` is not
   free here: `bun run build` type-checks all of `src` with `tsc`, so test
   files that import `bun:test` need `@types/bun`, or have to be excluded
   from type checking. `@types/bun` also loads Bun and Node globals into the
   app's types unless the tsconfigs are split. Without a DOM, what can be
   tested is near-trivial (`errorMessage`, the Set helpers, field copies).
   `rejectAfter` cannot even run without a DOM, since it uses
   `window.setTimeout`. The real risk in this refactor is the stateful logic
   it moves: debounces, save ordering, the close guard, onboarding steps and
   the manual-browser confirmation flow. Testing that needs happy-dom or
   jsdom, Testing Library, IPC mocks (`@tauri-apps/api/mocks`) and
   characterization tests that render the whole App, which is a bigger
   project than this refactor and a separate dependency decision. The safety
   net stays the manual verification list plus one commit per extraction.
   Hook tests can be proposed later as their own item.

## Proposal

- `src/lib/errors.ts` with `errorMessage(error: unknown): string`.
- `useIdSet` in `src/hooks/`.
- One `reportValidationFailure(message, options)` helper inside
  `useBrowserManager`; rename `validateManualBrowserConfig` to
  `validateBrowserConfig`.
- Split `App.tsx` into the hooks and screens listed in decision 2, and group
  tab props by the hook that owns them (decision 1).
- Remove the className constants, and the `className` props that only
  receive them.

## Out of scope

- Visual or behavior changes.
- CSS reorganization.

## Acceptance criteria

- [ ] `App.tsx` well under 300 lines; no component receives props it only
      forwards.
- [ ] No remaining inline `instanceof Error ? ... : String(...)`.
- [ ] No manual `cloneSet` add/remove boilerplate.
- [ ] `bun run build` passes; every screen behaves as before.

## Verification

- `bun run build`
- Manual in `bun run tauri dev`: onboarding (fresh config), add/edit/delete
  a manual browser (including the confirmation flow), hide/show browsers,
  rules (add, edit, reorder, invalid regex), router tester, reset config,
  rerun onboarding, about/update check, close the window with pending saves.

## Notes and risks

- No frontend tests exist, so manual verification is the safety net. Keep
  commits small (one extraction each) so a regression is easy to bisect.
