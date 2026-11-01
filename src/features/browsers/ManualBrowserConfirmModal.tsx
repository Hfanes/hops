import { ModalShell } from "../../components/common/ModalShell";
import type { BrowserConfig, ManualBrowserValidationResult } from "../../types";

/** A manual browser Hops does not recognize, waiting for the user to trust it. */
export interface ManualBrowserConfirmation {
  mode: "add" | "edit";
  browserId: string | null;
  browser: BrowserConfig;
  validation: ManualBrowserValidationResult;
}

export function ManualBrowserConfirmModal({
  confirmation,
  isConfirming,
  onConfirm,
  onClose,
}: {
  confirmation: ManualBrowserConfirmation;
  isConfirming: boolean;
  onConfirm: () => void;
  onClose: () => void;
}) {
  return (
    <ModalShell
      title={<h3 id="manual-browser-confirm-title">Confirm manual browser</h3>}
      titleId="manual-browser-confirm-title"
      onClose={onClose}
    >
      <p className="setting-help">{confirmation.validation.message}</p>
      <label>
        Executable path
        <input value={confirmation.browser.path} readOnly />
      </label>
      <label>
        Name
        <input value={confirmation.browser.name} readOnly />
      </label>
      <div className="inline-actions">
        <button type="button" onClick={onConfirm} disabled={isConfirming}>
          {isConfirming ? "Confirming..." : "Confirm and trust"}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={isConfirming}
          onClick={onClose}
        >
          Cancel
        </button>
      </div>
    </ModalShell>
  );
}
