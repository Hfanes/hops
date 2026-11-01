import { ModalShell } from "./ModalShell";

/** What the user is about to delete, with the details shown in the prompt. */
export type DeleteConfirmation =
  | {
      kind: "manual-browser";
      browserId: string;
      browserName: string;
      affectedRuleCount: number;
    }
  | {
      kind: "rule";
      ruleId: string;
      ruleLabel: string;
      pattern: string;
    };

export function DeleteConfirmModal({
  confirmation,
  onConfirm,
  onClose,
}: {
  confirmation: DeleteConfirmation;
  onConfirm: () => void;
  onClose: () => void;
}) {
  return (
    <ModalShell
      title={<h3 id="delete-confirm-title">Confirm delete</h3>}
      titleId="delete-confirm-title"
      onClose={onClose}
    >
      {confirmation.kind === "manual-browser" ? (
        <p className="setting-help">
          Delete manual browser "{confirmation.browserName}"? This also removes{" "}
          {confirmation.affectedRuleCount === 0
            ? "no rules"
            : confirmation.affectedRuleCount === 1
              ? "1 rule that targets it"
              : `${confirmation.affectedRuleCount} rules that target it`}
          .
        </p>
      ) : (
        <p className="setting-help">
          Delete {confirmation.ruleLabel}
          {confirmation.pattern.trim() ? ` for "${confirmation.pattern}"` : ""}?
          This cannot be undone.
        </p>
      )}
      <div className="inline-actions">
        <button type="button" className="danger" onClick={onConfirm}>
          Delete
        </button>
        <button type="button" className="secondary" onClick={onClose}>
          Cancel
        </button>
      </div>
    </ModalShell>
  );
}
