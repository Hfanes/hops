# Repository Guidelines

## Project Structure & Module Organization

Hops is a Windows-focused Tauri 2 app with a React/TypeScript frontend.

- `src/` contains frontend entry points, types, hooks, and Tauri API wrappers.
- `src/components/` holds reusable UI in `common/` and `layout/`.
- `src/features/` groups screens such as `rules`, `settings`, `picker`, `browsers`, `router`, `about`, and `onboarding`.
- `src/styles/` contains global CSS, tokens, layout, and responsive rules.
- `src-tauri/` contains the Rust backend, Tauri config, capabilities, icons, and Windows integration.
- `public/` is for static assets. `dist/`, `node_modules/`, and `src-tauri/target/` are generated.

## Build, Test, and Development Commands

- `bun install` installs frontend dependencies from `bun.lock`.
- `bun run dev` starts the Vite frontend only.
- `bun run tauri dev` runs the full desktop app in development mode.
- `bun run build` runs TypeScript checking and Vite production build.
- `bun run preview` serves the production frontend build locally.
- `cargo check --manifest-path src-tauri\Cargo.toml` checks the Rust backend.
- `cargo test --manifest-path src-tauri\Cargo.toml --lib` runs Rust library tests when present.

Use a packaged build to validate Windows GUI behavior; dev mode is terminal-owned.

## Dependency Guidelines

- Pin dependency versions exactly; do not use `^` or `~`.
- Keep `bun.lock` committed, and do not delete or ignore it.
- Use `bun install --frozen-lockfile` or `bun ci` for deterministic installs.
- Review dependency updates one at a time instead of doing blind bulk upgrades.
- New package versions must be at least 1 day old before they can be installed; release age gating is enabled.

## Lightweight App Principles

- Keep runtime behavior event-driven; avoid polling loops, background timers, and repeated scans unless tied to a user action.
- Do not add large frontend or Rust dependencies for small utilities; prefer platform APIs, Tauri APIs, or local helpers.
- Keep startup work minimal by deferring browser detection, update checks, route previews, and expensive UI work until needed.
- Store config as simple JSON, and avoid databases, persistent services, or extra local processes.
- Preserve tray-first behavior: stay hidden and idle unless handling a URL, showing the picker, or opening settings.
- Consider memory, startup time, installer size, and background CPU impact before adding features.

## Coding Style & Naming Conventions

Use TypeScript React function components and hooks. Keep frontend modules organized by feature, with PascalCase components (`RulesTab.tsx`) and camelCase utilities/hooks (`appTypes.ts`, `useDocumentTheme.ts`). Prefer wrappers in `src/services/tauri.ts` over direct command calls.

Follow existing formatting: two-space TypeScript indentation, double quotes in TS/TSX, trailing commas for wrapped lists, and `rustfmt` for Rust. Keep CSS in the relevant feature file or shared style layer.

## Testing Guidelines

There is no frontend test runner and no committed test files. Verify UI changes with `bun run build` and manual flows in `bun run tauri dev`. For Rust logic, add focused unit tests near the module under test and run `cargo test --manifest-path src-tauri\Cargo.toml --lib`.

## Security & Configuration Tips

Hops writes user-scoped Windows registry keys under `HKCU` for browser registration. Avoid machine-wide `HKLM` behavior without explicit design discussion. Do not commit signing keys, updater private keys, local config files, or generated artifacts.
