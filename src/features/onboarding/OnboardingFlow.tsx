import type { Dispatch, ReactNode, SetStateAction } from "react";
import { BrowserIcon } from "../../components/common/BrowserIcon";
import type { AppConfig, BrowserDraft, BrowserRegistrationStatus } from "../../types";

export function OnboardingFlow({
  statusBanner,
  config,
  onboardingStep,
  browserDraft,
  isRefreshing,
  isRegistering,
  registrationStatus,
  isCheckingOnboardingDefaults,
  onboardingStartWithWindows,
  isFinishingOnboarding,
  onRefreshBrowsers,
  onSetBrowserDraft,
  onAddManualBrowser,
  onSetOnboardingStep,
  onRegisterBrowserIntegration,
  onRefreshRegistrationStatus,
  onOpenDefaultAppsSettings,
  onContinueToFinish,
  onSetOnboardingStartWithWindows,
  onFinishOnboarding,
}: {
  statusBanner: ReactNode;
  config: AppConfig;
  onboardingStep: number;
  browserDraft: BrowserDraft;
  isRefreshing: boolean;
  isRegistering: boolean;
  registrationStatus: BrowserRegistrationStatus | null;
  isCheckingOnboardingDefaults: boolean;
  onboardingStartWithWindows: boolean;
  isFinishingOnboarding: boolean;
  onRefreshBrowsers: () => void;
  onSetBrowserDraft: Dispatch<SetStateAction<BrowserDraft>>;
  onAddManualBrowser: () => void;
  onSetOnboardingStep: (step: number) => void;
  onRegisterBrowserIntegration: () => void;
  onRefreshRegistrationStatus: () => void;
  onOpenDefaultAppsSettings: () => void;
  onContinueToFinish: () => void;
  onSetOnboardingStartWithWindows: (enabled: boolean) => void;
  onFinishOnboarding: () => void;
}) {
  return (
    <>
      {statusBanner}

      {onboardingStep === 0 ? (
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
              onClick={onRefreshBrowsers}
              disabled={isRefreshing}
            >
              {isRefreshing ? "Refreshing..." : "Refresh browsers"}
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
                value={browserDraft.name}
                onChange={(event) => {
                  const value = event.currentTarget.value;
                  onSetBrowserDraft((current) => ({
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
                value={browserDraft.path}
                onChange={(event) => {
                  const value = event.currentTarget.value;
                  onSetBrowserDraft((current) => ({
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
              <button type="button" onClick={onAddManualBrowser}>
                Add browser
              </button>
            </div>
          </article>

          <div className="inline-actions">
            <button type="button" onClick={() => onSetOnboardingStep(1)}>
              Continue
            </button>
          </div>
        </section>
      ) : null}

      {onboardingStep === 1 ? (
        <section className="tab-body">
          <h3>2. Register Hops in Windows Default Apps</h3>
          <p className="setting-help">
            This writes per-user keys in <code>HKCU</code> so Windows can list
            Hops as a browser candidate.
          </p>

          <div className="inline-actions">
            <button
              type="button"
              onClick={onRegisterBrowserIntegration}
              disabled={isRegistering}
            >
              Register Hops
            </button>
            <button
              type="button"
              className="secondary"
              onClick={onRefreshRegistrationStatus}
              disabled={isRegistering}
            >
              Refresh status
            </button>
          </div>

          <article className="card">
            <p>
              Registered in Default Apps list:{" "}
              <strong>{registrationStatus?.registered ? "Yes" : "No"}</strong>
            </p>
          </article>

          <div className="inline-actions">
            <button
              type="button"
              className="secondary"
              onClick={() => onSetOnboardingStep(0)}
            >
              Back
            </button>
            <button
              type="button"
              onClick={() => onSetOnboardingStep(2)}
              disabled={!registrationStatus?.registered}
            >
              Continue
            </button>
          </div>
        </section>
      ) : null}

      {onboardingStep === 2 ? (
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
              onClick={onOpenDefaultAppsSettings}
            >
              Open Windows Default Apps
            </button>
            <button
              type="button"
              className="secondary"
              onClick={onRefreshRegistrationStatus}
            >
              Refresh status
            </button>
          </div>

          <article className="card">
            <p>
              HTTP default:{" "}
              <strong>
                {registrationStatus?.isDefaultHttp ? "Hops" : "Not Hops"}
              </strong>
            </p>
            <p>
              HTTPS default:{" "}
              <strong>
                {registrationStatus?.isDefaultHttps ? "Hops" : "Not Hops"}
              </strong>
            </p>
          </article>

          <div className="inline-actions">
            <button
              type="button"
              className="secondary"
              onClick={() => onSetOnboardingStep(1)}
            >
              Back
            </button>
            <button
              type="button"
              onClick={onContinueToFinish}
              disabled={isCheckingOnboardingDefaults}
            >
              {isCheckingOnboardingDefaults ? "Checking..." : "Continue"}
            </button>
          </div>
        </section>
      ) : null}

      {onboardingStep === 3 ? (
        <section className="tab-body">
          <h3>4. Finish onboarding</h3>
          <p className="setting-help">
            Hops will keep running in tray and process external links in the
            background.
          </p>
          <label className="toggle">
            <input
              type="checkbox"
              checked={onboardingStartWithWindows}
              onChange={(event) =>
                onSetOnboardingStartWithWindows(event.currentTarget.checked)
              }
              disabled={isFinishingOnboarding}
            />
            <span>Start with Windows</span>
          </label>
          <p className="setting-help">
            Launch Hops when you sign in. After setup, it starts hidden in the
            tray.
          </p>
          {!registrationStatus?.isFullyDefault ? (
            <p className="status warning">
              Hops is not yet default for both HTTP and HTTPS. You can finish
              now and complete this later in Settings.
            </p>
          ) : null}

          <div className="inline-actions">
            <button
              type="button"
              className="secondary"
              onClick={() => onSetOnboardingStep(2)}
            >
              Back
            </button>
            <button
              type="button"
              onClick={onFinishOnboarding}
              disabled={isFinishingOnboarding}
            >
              {isFinishingOnboarding ? "Finishing..." : "Finish and open Hops"}
            </button>
          </div>
        </section>
      ) : null}
    </>
  );
}
