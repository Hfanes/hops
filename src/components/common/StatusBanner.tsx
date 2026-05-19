import { FiX } from "react-icons/fi";
import type { StatusState } from "../../types";

export function StatusBanner({
  status,
  onDismiss,
}: {
  status: StatusState;
  onDismiss: () => void;
}) {
  if (!status.text) {
    return null;
  }

  return (
    <div className={`status ${status.kind}`} role="status" aria-live="polite">
      <span>{status.text}</span>
      <button
        type="button"
        className="status-close"
        onClick={onDismiss}
        aria-label="Dismiss status message"
        title="Dismiss"
      >
        <FiX aria-hidden="true" />
      </button>
    </div>
  );
}
