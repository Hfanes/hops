import type { ReactNode } from "react";
import { BrowserIcon } from "../../components/common/BrowserIcon";
import type { AppConfig } from "../../types";
import type { BrowserManager } from "../browsers/useBrowserManager";
import type { SystemIntegration } from "../settings/useSystemIntegration";
import type { Onboarding } from "./useOnboarding";

export function OnboardingFlow({
  statusBanner,
  config,
  onboarding,
  integration,
  browsers,
}: {
  statusBanner: ReactNode;
  config: AppConfig;
  onboarding: Onboarding;
  integration: SystemIntegration;
  browsers: BrowserManager;
}) {
  return (
    <>
      {statusBanner}

      {onboarding.step === 0 ? (
        <section className="tab-body">
          <h3>1. Detect your browsers</h3>
          <p className="setting-help">
            We scanned common install locations. Add any missing browser
            manually.
          </p>

          <div className="inline-actions">
            <button
              type="button"
              className="secondary"
              onClick={() => void browsers.handleRefreshBrowsers()}
              disabled={browsers.isRefreshing}
            >
              {browsers.isRefreshing ? "Refreshing..." : "Refresh browsers"}
            </button>
          </div>

          <div className="browser-list">
            {config.browsers.length === 0 ? (
              <article className="card">
                <p>
                  No browsers detected yet. Refresh scan or add one manually
                  below.
                </p>
              </article>
            ) : (
              config.browsers.map((browser) => (
                <article key={browser.id} className="card">
                  <div className="card-title">
                    <span className="browser-title">
                      <BrowserIcon iconKey={browser.iconKey} />
                      <strong>{browser.name}</strong>
                    </span>
                    <div className="badges">
                      <span className="badge">{browser.source}</span>
                    </div>
                  </div>
                  <p>{browser.path}</p>
                </article>
              ))
            )}
          </div>

          <article className="card">
            <h3>Add manual browser</h3>
            <label>
              Name
              <input
                value={browsers.browserDraft.name}
                onChange={(event) => {
                  const value = event.currentTarget.value;
                  browsers.setBrowserDraft((current) => ({
                    ...current,
                    name: value,
                  }));
                }}
                placeholder="Portable Browser"
              />
            </label>
            <label>
              Executable path
              <input
                value={browsers.browserDraft.path}
                onChange={(event) => {
                  const value = event.currentTarget.value;
                  browsers.setBrowserDraft((current) => ({
                    ...current,
                    path: value,
                  }));
                }}
                placeholder="C:\\Tools\\Browser\\browser.exe"
              />
            </label>
            <label>
              Private mode flag
              <input
                value={browsers.browserDraft.privateFlag}
                onChange={(event) => {
                  const value = event.currentTarget.value;
                  browsers.setBrowserDraft((current) => ({
                    ...current,
                    privateFlag: value,
                  }));
                }}
                placeholder="--incognito"
              />
            </label>
            <div className="inline-actions">
              <button type="button" onClick={() => void browsers.addManualBrowser()}>
                Add browser
              </button>
            </div>
          </article>

          <div className="inline-actions">
            <button type="button" onClick={() => onboarding.setStep(1)}>
              Continue
            </button>
          </div>
        </section>
      ) : null}

      {onboarding.step === 1 ? (
        <section className="tab-body">
          <h3>2. Register Hops in Windows Default Apps</h3>
          <p className="setting-help">
            This writes per-user keys in <code>HKCU</code> so Windows can list
            Hops as a browser candidate.
          </p>

          <div className="inline-actions">
            <button
              type="button"
              onClick={() => void integration.registerBrowserIntegration()}
              disabled={integration.isRegistering}
            >
              Register Hops
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => void integration.refreshRegistrationStatus()}
              disabled={integration.isRegistering}
            >
              Refresh status
            </button>
          </div>

          <article className="card">
            <p>
              Registered in Default Apps list:{" "}
              <strong>{integration.registrationStatus?.registered ? "Yes" : "No"}</strong>
            </p>
          </article>

          <div className="inline-actions">
            <button
              type="button"
              className="secondary"
              onClick={() => onboarding.setStep(0)}
            >
              Back
            </button>
            <button
              type="button"
              onClick={() => onboarding.setStep(2)}
              disabled={!integration.registrationStatus?.registered}
            >
              Continue
            </button>
          </div>
        </section>
      ) : null}

      {onboarding.step === 2 ? (
        <section className="tab-body">
          <h3>3. Set Hops as default for HTTP and HTTPS</h3>
          <p className="setting-help">
            Windows requires user confirmation. Open Default Apps and set Hops
            for both <code>http</code> and <code>https</code>.
          </p>

          <div className="inline-actions">
            <button
              type="button"
              className="secondary"
              onClick={() => void integration.openDefaultAppsSettings()}
            >
              Open Windows Default Apps
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => void integration.refreshRegistrationStatus()}
            >
              Refresh status
            </button>
          </div>

          <article className="card">
            <p>
              HTTP default:{" "}
              <strong>
                {integration.registrationStatus?.isDefaultHttp ? "Hops" : "Not Hops"}
              </strong>
            </p>
            <p>
              HTTPS default:{" "}
              <strong>
                {integration.registrationStatus?.isDefaultHttps ? "Hops" : "Not Hops"}
              </strong>
            </p>
          </article>

          <div className="inline-actions">
            <button
              type="button"
              className="secondary"
              onClick={() => onboarding.setStep(1)}
            >
              Back
            </button>
            <button
              type="button"
              onClick={() => void onboarding.continueToFinish()}
              disabled={onboarding.isCheckingDefaults}
            >
              {onboarding.isCheckingDefaults ? "Checking..." : "Continue"}
            </button>
          </div>
        </section>
      ) : null}

      {onboarding.step === 3 ? (
        <section className="tab-body">
          <h3>4. Finish onboarding</h3>
          <p className="setting-help">
            Hops will keep running in tray and process external links in the
            background.
          </p>
          <label className="toggle">
            <input
              type="checkbox"
              checked={onboarding.startWithWindows}
              onChange={(event) =>
                onboarding.setStartWithWindows(event.currentTarget.checked)
              }
              disabled={onboarding.isFinishing}
            />
            <span>Start with Windows</span>
          </label>
          <p className="setting-help">
            Launch Hops when you sign in. After setup, it starts hidden in the
            tray.
          </p>
          {!integration.registrationStatus?.isFullyDefault ? (
            <p className="status warning">
              Hops is not yet default for both HTTP and HTTPS. You can finish
              now and complete this later in Settings.
            </p>
          ) : null}

          <div className="inline-actions">
            <button
              type="button"
              className="secondary"
              onClick={() => onboarding.setStep(2)}
            >
              Back
            </button>
            <button
              type="button"
              onClick={() => void onboarding.finish()}
              disabled={onboarding.isFinishing}
            >
              {onboarding.isFinishing ? "Finishing..." : "Finish and open Hops"}
            </button>
          </div>
        </section>
      ) : null}
    </>
  );
}
