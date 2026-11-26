import type { RuleConfig } from "../../types";

/** Mirrors `MAX_RULE_NAME_CHARS` in config.rs, which cuts longer names. */
export const MAX_RULE_NAME_LENGTH = 60;

/** The label for the rule at `index`: its name, or "Rule N" by position. */
export function ruleLabel(rule: RuleConfig, index: number) {
  return rule.name?.trim() || defaultRuleLabel(index);
}

export function defaultRuleLabel(index: number) {
  return `Rule ${index + 1}`;
}
