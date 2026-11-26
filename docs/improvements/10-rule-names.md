# 10 — Rule names

Added on 2026-11-26, after item 08 was merged. Line references point at
`f598929`.

## Context and problem

Rules have no name. The Rules tab labels each card by position
(`RulesTab.tsx:40`, `Rule {index + 1}`), and the delete confirmation builds
the same label (`useRuleManager.ts:155`). With several rules the label says
nothing about what a rule is for. It also changes whenever a rule is moved
or one above it is deleted.

This is not a frontend-only change. The backend reads and rewrites the whole
config through `RuleConfig` (`src-tauri/src/models.rs:60`), which has no name
field. Serde ignores unknown fields, so a `name` added only in TypeScript
would be dropped on the first save. Every save runs `normalize_config` and
`validate_config` (`config.rs:462`, `config.rs:497`, `commands.rs:144-157`).
Loading only normalizes (`config.rs:264`, `config.rs:378`).

`RuleConfig` is built literally in two Rust test helpers (`picker.rs:666`,
`routing.rs:453`), and those need the new field. Routing never reads it.

## Open questions

1. How to store the name: `name: Option<String>` with `#[serde(default)]`,
   or `name: String` defaulting to `""`? Either loads existing configs
   without a version bump.
2. What is the default: a positional "Rule N" label computed when nothing is
   stored, or "Rule N" written as the name when the rule is created?
3. Validation: what maximum length, and should names be unique? A name
   longer than the limit that was written into the file by hand loads fine,
   because loading does not validate, and then fails on the next save.
4. Where is the name entered and shown: an optional field in the Add rule
   modal, inline editing on the rule card saved with the existing per-rule
   Save button, and the label in the card title and the delete confirmation?

## Decisions

1. **`name: Option<String>` with `#[serde(default)]`.** A config written
   before this item loads with `None`, so the config version stays at 1.
   Normalization trims the name and turns a blank one into `None`, so there
   is one empty state instead of two (`""` and only spaces).
2. **A positional "Rule N" label when no name is stored.** Nothing is written
   for unnamed rules. Writing "Rule 3" as the name at creation would leave a
   rule called "Rule 3" at the top of the list after a reorder.
3. **At most 60 characters, cut in `normalize_config`.** It is counted in
   characters, not bytes, so accented text and emoji are never split.
   Normalization runs on load and on every save, so a longer name written
   into the file by hand is truncated instead of failing validation.
   Rejecting it in `validate_config` would block every save, a theme change
   included, until the name was shortened. Item 03 already decided that a
   non-critical problem must not block saving the other settings, and a name
   is cosmetic. The inputs also set `maxLength={60}`, so the UI never gets
   there. Duplicate names are allowed, because rules are identified by `id`.
4. **Entered in the Add rule modal and on each rule card.** The modal has an
   optional Name field with "Rule N" as placeholder, reset with the pattern
   after a rule is added. The card has a Name input that marks the rule as
   having unsaved edits and is saved with the existing per-rule Save button.
   One `ruleLabel(rule, index)` helper gives the card title and the delete
   confirmation label.

## Proposal

- `RuleConfig.name` in Rust and TypeScript, with the normalization and
  validation decided above, and unit tests for loading an old config, a
  blank name and an over-long name.
- `name` in `RuleDraft`, an optional Name field in `AddRuleModal`, reset
  after a rule is added together with the pattern.
- A Name input on each rule card. It marks the rule as having unsaved edits
  like the other fields.
- One `ruleLabel(rule, index)` helper used by the card title and the delete
  confirmation.

## Out of scope

- Showing rule names anywhere else (picker, tray, router tester result).
- Searching or filtering rules by name.

## Acceptance criteria

- [ ] An existing config without rule names loads and saves without errors.
- [ ] A rule can be named when it is created and renamed later.
- [ ] A rule without a name shows the default label.
- [ ] The backend trims names, stores blank ones as `None` and cuts them at
      60 characters.
- [ ] `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check` and
      `bun run build` pass.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml --lib`
- `cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings`
- `cargo fmt --manifest-path src-tauri\Cargo.toml --check`
- `bun run build`
- Manual in `bun run tauri dev`: start from a config with rules and no names,
  add a named and an unnamed rule, rename one, clear a name, reorder, delete
  (check the confirmation label), restart the app and check the names
  survive.
