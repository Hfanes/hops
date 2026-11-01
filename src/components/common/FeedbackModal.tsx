import { ModalShell } from "./ModalShell";

/** A blocking message the user must acknowledge (errors and warnings). */
export interface FeedbackMessage {
  title: string;
  message: string;
  kind: "error" | "warning";
  confirmLabel?: string;
}

export function FeedbackModal({
  feedback,
  onClose,
}: {
  feedback: FeedbackMessage;
  onClose: () => void;
}) {
  return (
    <ModalShell
      title={<h3 id="feedback-modal-title">{feedback.title}</h3>}
      titleId="feedback-modal-title"
      onClose={onClose}
    >
      <p className={`status ${feedback.kind}`}>{feedback.message}</p>
      <div className="inline-actions">
        <button type="button" onClick={onClose}>
          {feedback.confirmLabel ?? "OK"}
        </button>
      </div>
    </ModalShell>
  );
}
