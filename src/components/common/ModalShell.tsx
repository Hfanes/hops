import type { ReactNode } from "react";
import { FiX } from "react-icons/fi";

export function ModalShell({
  title,
  titleId,
  children,
  onClose,
}: {
  title: ReactNode;
  titleId: string;
  children: ReactNode;
  onClose: () => void;
}) {
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="modal-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="modal-title">
          {title}
          <button
            type="button"
            className="icon-button secondary"
            onClick={onClose}
            aria-label={`Close ${titleId}`}
            title="Close"
          >
            <FiX aria-hidden="true" />
          </button>
        </div>
        {children}
      </section>
    </div>
  );
}
