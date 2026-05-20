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
  function saveStateText(browser: BrowserConfig) {
    if (savingBrowserIds.has(browser.id)) {
      return "Saving browser...";
    }
    if (pendingBrowserIds.has(browser.id)) {
      return "Unsaved browser changes...";
    }
    if (failedBrowserIds.has(browser.id)) {
      return "Browser save failed. Keep editing to retry.";
    }
    if (browser.source === "manual" && !browser.manualTrust) {
      return "Path changes require validation before Hops will trust this browser.";
    }
    return "Browser changes save automatically.";
  }

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
                {browser.source === "manual" && browser.manualTrust ? (
                  <span className="badge">
                    {browser.manualTrust === "verified"
                      ? "verified"
                      : "user confirmed"}
                  </span>
                ) : null}
                {runningBrowserIds.has(browser.id) ? (
                  <span className="badge running">running</span>
                ) : null}
                {browser.isHidden ? (
                  <span className="badge warning">hidden</span>
                ) : null}
              </div>
            </div>

            <p className="setting-help inline-save-state">
              {saveStateText(browser)}
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
