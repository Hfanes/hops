import { PatternTypePicker } from "../../components/common/PatternTypePicker";
import type { BrowserConfig, RuleConfig } from "../../types";

export function RulesTab({
  rules,
  visibleBrowsers,
  regexErrors,
  dirtyRuleIds,
  savingRuleIds,
  onMoveRule,
  onDeleteRule,
  onUpdateRule,
  onSaveRule,
}: {
  rules: RuleConfig[];
  visibleBrowsers: BrowserConfig[];
  regexErrors: Record<string, string>;
  dirtyRuleIds: Set<string>;
  savingRuleIds: Set<string>;
  onMoveRule: (ruleId: string, direction: -1 | 1) => void;
  onDeleteRule: (ruleId: string) => void;
  onUpdateRule: (ruleId: string, patch: Partial<RuleConfig>) => void;
  onSaveRule: (ruleId: string) => void;
}) {
  return (
    <section className="tab-body">
      {!visibleBrowsers.length ? (
        <p>Add or detect at least one browser before creating rules.</p>
      ) : null}
      <section className="rules-section">
        <h3>Existing rules</h3>
        <p className="setting-help">
          Rules are evaluated top to bottom. First enabled match wins. Field
          edits stay local until you save that rule.
        </p>
        <div className="rule-list">
          {rules.map((rule, index) => (
            <article key={rule.id} className="card">
              <div className="card-title">
                <strong>Rule {index + 1}</strong>
                <div className="inline-actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => onMoveRule(rule.id, -1)}
                  >
                    Up
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => onMoveRule(rule.id, 1)}
                  >
                    Down
                  </button>
                  <button
                    type="button"
                    className="secondary danger"
                    onClick={() => onDeleteRule(rule.id)}
                  >
                    Delete
                  </button>
                </div>
              </div>

              <label>
                Pattern
                <input
                  value={rule.pattern}
                  onChange={(event) =>
                    onUpdateRule(rule.id, {
                      pattern: event.currentTarget.value,
                    })
                  }
                  placeholder="github.com"
                />
              </label>

              {regexErrors[rule.id] ? (
                <p className="field-error">
                  Regex error: {regexErrors[rule.id]}
                </p>
              ) : null}
              {!regexErrors[rule.id] && dirtyRuleIds.has(rule.id) ? (
                <p className="setting-help inline-save-state">
                  Unsaved rule changes.
                </p>
              ) : null}

              <PatternTypePicker
                name={`rule-${rule.id}-pattern-type`}
                value={rule.patternType}
                onChange={(value) =>
                  onUpdateRule(rule.id, { patternType: value })
                }
              />

              <label>
                Target browser
                <select
                  value={rule.browserId}
                  onChange={(event) =>
                    onUpdateRule(rule.id, {
                      browserId: event.currentTarget.value,
                    })
                  }
                >
                  {visibleBrowsers.map((browser) => (
                    <option key={browser.id} value={browser.id}>
                      {browser.name}
                    </option>
                  ))}
                </select>
              </label>

              <div className="dual-toggle">
                <label className="toggle">
                  <input
                    type="checkbox"
                    checked={rule.privateMode}
                    onChange={(event) =>
                      onUpdateRule(rule.id, {
                        privateMode: event.currentTarget.checked,
                      })
                    }
                  />
                  <span>Private mode</span>
                </label>
                <label className="toggle">
                  <input
                    type="checkbox"
                    checked={rule.enabled}
                    onChange={(event) =>
                      onUpdateRule(rule.id, {
                        enabled: event.currentTarget.checked,
                      })
                    }
                  />
                  <span>Enabled</span>
                </label>
              </div>

              <div className="inline-actions">
                <button
                  type="button"
                  onClick={() => onSaveRule(rule.id)}
                  disabled={
                    !dirtyRuleIds.has(rule.id) ||
                    !!regexErrors[rule.id] ||
                    savingRuleIds.has(rule.id)
                  }
                >
                  {savingRuleIds.has(rule.id) ? "Saving..." : "Save rule"}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </section>
  );
}
