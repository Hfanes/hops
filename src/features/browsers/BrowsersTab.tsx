import type { BrowserConfig } from "../../types";

export function BrowsersTab({
  browsers,
  runningBrowserIds,
  savingBrowserIds,
  pendingBrowserIds,
  failedBrowserIds,
  onUpdateBrowser,
  onFlushBrowserSave,
  onToggleBrowserHidden,
}: {
  browsers: BrowserConfig[];
  runningBrowserIds: Set<string>;
  savingBrowserIds: Set<string>;
  pendingBrowserIds: Set<string>;
  failedBrowserIds: Set<string>;
  onUpdateBrowser: (browserId: string, patch: Partial<BrowserConfig>) => void;
  onFlushBrowserSave: (browserId: string) => void;
  onToggleBrowserHidden: (browserId: string, isHidden: boolean) => void;
}) {
  return (
    <section className="tab-body">
      <div className="browser-list">
        {browsers.map((browser) => (
          <article
            key={browser.id}
            className={`card ${browser.isHidden ? "muted" : ""}`}
          >
            <div className="card-title">
              <strong>{browser.name}</strong>
              <div className="badges">
                <span className="badge">{browser.source}</span>
                {runningBrowserIds.has(browser.id) ? (
                  <span className="badge running">running</span>
                ) : null}
                {browser.isHidden ? (
                  <span className="badge warning">hidden</span>
                ) : null}
              </div>
            </div>

            <p className="setting-help inline-save-state">
              {savingBrowserIds.has(browser.id)
                ? "Saving browser..."
                : pendingBrowserIds.has(browser.id)
                  ? "Unsaved browser changes..."
                  : failedBrowserIds.has(browser.id)
                    ? "Browser save failed. Keep editing to retry."
                    : "Browser changes save automatically."}
            </p>

            <label>
              Display name
              <input
                value={browser.name}
                onChange={(event) =>
                  onUpdateBrowser(browser.id, {
                    name: event.currentTarget.value,
                  })
                }
                onBlur={() => onFlushBrowserSave(browser.id)}
              />
            </label>

            <label>
              Executable path
              <input
                value={browser.path}
                onChange={(event) =>
                  onUpdateBrowser(browser.id, {
                    path: event.currentTarget.value,
                  })
                }
                onBlur={() => onFlushBrowserSave(browser.id)}
              />
            </label>

            <label>
              Private mode flag
              <input
                value={browser.privateFlag ?? ""}
                placeholder="--incognito"
                onChange={(event) =>
                  onUpdateBrowser(browser.id, {
                    privateFlag: event.currentTarget.value.trim() || null,
                  })
                }
                onBlur={() => onFlushBrowserSave(browser.id)}
              />
            </label>

            <div className="inline-actions">
              {browser.isHidden ? (
                <button
                  type="button"
                  className="secondary"
                  onClick={() => onToggleBrowserHidden(browser.id, false)}
                >
                  Restore
                </button>
              ) : (
                <button
                  type="button"
                  className="secondary"
                  onClick={() => onToggleBrowserHidden(browser.id, true)}
                >
                  Hide from picker
                </button>
              )}
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}
