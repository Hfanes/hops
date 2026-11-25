# 08 — Frontend refactor

Do after 01 (removes unused wrappers) and 05 (changes the path normalization in `useBrowserManager.ts`).

## Context and problem

### `src/App.tsx` (813 lines)

A god component: about 20 `useState`s plus the onboarding, reset, rerun
onboarding, route tester and close-window logic, and wiring for six hooks.
`SettingsTab` and `OnboardingFlow` receive 20+ props each, most of them passed
straight through. Constants such as `shellClassName = "app-shell"`
(`App.tsx:521-528`) add indirection without value.

### `src/features/browsers/useBrowserManager.ts` (680 lines)

- The `setX(current => { const next = cloneSet(current); next.add/delete(id);
  return next; })` pattern appears 18 times across the frontend, most of them
  here.
- `validateManualBrowserConfig` (`useBrowserManager.ts:149-229`) repeats the
  "revert browser, then show alert or set status" block three times.
- `{ name: "", path: "", privateFlag: "" }` is written inline twice
  (`:550`, `:573`) although `EMPTY_BROWSER_DRAFT` exists.
- `validateManualBrowserConfig` is also used for detected browsers, so its
  name is misleading.

### Repeated error formatting

`error instanceof Error ? error.message : String(error)` appears 21 times in
`src/`.

## Open questions

1. Prop drilling: React Context for the config store and status (one
   `AppContext`, or one per concern), or keep props but group them into
   objects? Context is lighter on code; props are more explicit.
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

_Filled in during the discussion._

## Proposal

- `src/lib/errors.ts` with `errorMessage(error: unknown): string`.
- `useIdSet` in `src/hooks/`.
- One `reportValidationFailure(message, options)` helper inside
  `useBrowserManager`; rename `validateManualBrowserConfig` to
  `validateBrowserConfig`.
- Split `App.tsx` into hooks and screens as decided; Context if decided.
- Remove the className constants.

## Out of scope

- Visual or behavior changes.
- CSS reorganization.

## Acceptance criteria

- [ ] `App.tsx` well under 300 lines; no component receives props it only
      forwards (if Context is chosen).
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
