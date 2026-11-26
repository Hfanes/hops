import {
  DeleteConfirmModal,
  type DeleteConfirmation,
} from "./components/common/DeleteConfirmModal";
import {
  FeedbackModal,
  type FeedbackMessage,
} from "./components/common/FeedbackModal";
import { AddBrowserModal } from "./features/browsers/AddBrowserModal";
import { ManualBrowserConfirmModal } from "./features/browsers/ManualBrowserConfirmModal";
import type { BrowserManager } from "./features/browsers/useBrowserManager";
import { AddRuleModal } from "./features/rules/AddRuleModal";
import type { RuleManager } from "./features/rules/useRuleManager";
import type { BrowserConfig, FormModal } from "./types";

/**
 * The modals of the main screen: the add browser and add rule forms, the
 * manual browser confirmation, delete confirmations and feedback messages.
 */
export function AppModals({
  formModal,
  onCloseFormModal,
  browsers,
  rules,
  visibleBrowsers,
  isSaving,
  deleteConfirmation,
  onCloseDeleteConfirmation,
  feedback,
  onCloseFeedback,
}: {
  formModal: FormModal;
  onCloseFormModal: () => void;
  browsers: BrowserManager;
  rules: RuleManager;
  visibleBrowsers: BrowserConfig[];
  isSaving: boolean;
  deleteConfirmation: DeleteConfirmation | null;
  onCloseDeleteConfirmation: () => void;
  feedback: FeedbackMessage | null;
  onCloseFeedback: () => void;
}) {
  function confirmDelete() {
    const confirmation = deleteConfirmation;
    if (!confirmation) {
      return;
    }

    onCloseDeleteConfirmation();
    if (confirmation.kind === "manual-browser") {
      browsers.deleteManualBrowser(confirmation.browserId);
      return;
    }

    rules.deleteRule(confirmation.ruleId);
  }

  return (
    <>
      {formModal === "manual-browser" ? (
        <AddBrowserModal
          browserDraft={browsers.browserDraft}
          isSaving={isSaving}
          onSetBrowserDraft={browsers.setBrowserDraft}
          onAdd={() => void browsers.addManualBrowser()}
          onClose={onCloseFormModal}
        />
      ) : formModal === "rule" ? (
        <AddRuleModal
          ruleDraft={rules.ruleDraft}
          visibleBrowsers={visibleBrowsers}
          isSaving={isSaving}
          onSetRuleDraft={rules.setRuleDraft}
          onAdd={() => void rules.addRule()}
          onClose={onCloseFormModal}
        />
      ) : null}

      {browsers.manualBrowserConfirmation ? (
        <ManualBrowserConfirmModal
          confirmation={browsers.manualBrowserConfirmation}
          isConfirming={browsers.isConfirmingManualBrowser}
          onConfirm={() => void browsers.confirmManualBrowser()}
          onClose={() => browsers.setManualBrowserConfirmation(null)}
        />
      ) : null}

      {deleteConfirmation ? (
        <DeleteConfirmModal
          confirmation={deleteConfirmation}
          onConfirm={confirmDelete}
          onClose={onCloseDeleteConfirmation}
        />
      ) : null}

      {feedback ? (
        <FeedbackModal feedback={feedback} onClose={onCloseFeedback} />
      ) : null}
    </>
  );
}
