import type { Dispatch, SetStateAction } from "react";
import { ModalShell } from "../../components/common/ModalShell";
import type { BrowserDraft } from "../../types";

export function AddBrowserModal({
  browserDraft,
  isSaving,
  onSetBrowserDraft,
  onAdd,
  onClose,
}: {
  browserDraft: BrowserDraft;
  isSaving: boolean;
  onSetBrowserDraft: Dispatch<SetStateAction<BrowserDraft>>;
  onAdd: () => void;
  onClose: () => void;
}) {
  return (
    <ModalShell
      title={<h3 id="manual-browser-title">Add manual browser</h3>}
      titleId="manual-browser-title"
      onClose={onClose}
    >
      <label>
        Name
        <input
          value={browserDraft.name}
          onChange={(event) => {
            const value = event.currentTarget.value;
            onSetBrowserDraft((current) => ({
              ...current,
              name: value,
            }));
          }}
          placeholder="Portable Chrome"
        />
      </label>
      <label>
        Executable path
        <input
          value={browserDraft.path}
          onChange={(event) => {
            const value = event.currentTarget.value;
            onSetBrowserDraft((current) => ({
              ...current,
              path: value,
            }));
          }}
          placeholder="C:\\Tools\\Chrome\\chrome.exe"
        />
      </label>
      <label>
        Private mode flag
        <input
          value={browserDraft.privateFlag}
          onChange={(event) => {
            const value = event.currentTarget.value;
            onSetBrowserDraft((current) => ({
              ...current,
              privateFlag: value,
            }));
          }}
          placeholder="--incognito"
        />
      </label>
      <div className="inline-actions">
        <button type="button" onClick={onAdd} disabled={isSaving}>
          Add browser
        </button>
        <button type="button" className="secondary" onClick={onClose}>
          Cancel
        </button>
      </div>
    </ModalShell>
  );
}
