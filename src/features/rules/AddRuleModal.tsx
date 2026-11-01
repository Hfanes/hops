import type { Dispatch, SetStateAction } from "react";
import { ModalShell } from "../../components/common/ModalShell";
import { PatternTypePicker } from "../../components/common/PatternTypePicker";
import type { BrowserConfig, RuleDraft } from "../../types";

export function AddRuleModal({
  ruleDraft,
  visibleBrowsers,
  isSaving,
  onSetRuleDraft,
  onAdd,
  onClose,
}: {
  ruleDraft: RuleDraft;
  visibleBrowsers: BrowserConfig[];
  isSaving: boolean;
  onSetRuleDraft: Dispatch<SetStateAction<RuleDraft>>;
  onAdd: () => void;
  onClose: () => void;
}) {
  return (
    <ModalShell
      title={
        <div>
          <h3 id="rule-modal-title">Add rule</h3>
          <p className="setting-help">New rules are created as enabled.</p>
        </div>
      }
      titleId="rule-modal-title"
      onClose={onClose}
    >
      <label>
        Pattern
        <input
          value={ruleDraft.pattern}
          onChange={(event) => {
            const value = event.currentTarget.value;
            onSetRuleDraft((current) => ({
              ...current,
              pattern: value,
            }));
          }}
          placeholder="*.notion.so"
        />
      </label>
      <PatternTypePicker
        name="new-rule-pattern-type"
        value={ruleDraft.patternType}
        onChange={(value) =>
          onSetRuleDraft((current) => ({
            ...current,
            patternType: value,
          }))
        }
      />
      <label>
        Browser
        <select
          value={ruleDraft.browserId}
          onChange={(event) => {
            const value = event.currentTarget.value;
            onSetRuleDraft((current) => ({
              ...current,
              browserId: value,
            }));
          }}
        >
          <option value="">Choose browser</option>
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
            checked={ruleDraft.privateMode}
            onChange={(event) => {
              const checked = event.currentTarget.checked;
              onSetRuleDraft((current) => ({
                ...current,
                privateMode: checked,
              }));
            }}
          />
          <span>Private mode</span>
        </label>
      </div>
      <div className="inline-actions">
        <button
          type="button"
          onClick={onAdd}
          disabled={isSaving || !visibleBrowsers.length}
        >
          Add rule
        </button>
        <button type="button" className="secondary" onClick={onClose}>
          Cancel
        </button>
      </div>
    </ModalShell>
  );
}
