import {
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { FiChevronDown, FiChevronUp } from "react-icons/fi";
import { BrowserIcon } from "../../components/common/BrowserIcon";
import type {
  BrowserConfig,
  BrowserRegistrationStatus,
  SettingsActionPanel,
  SettingsDraft,
  ThemePreference,
} from "../../types";

type SettingsDraftUpdater = (current: SettingsDraft) => SettingsDraft;

export function SettingsTab({
  startWithWindowsEnabled,
  isUpdatingStartWithWindows,
  settingsDraft,
  visibleBrowsers,
  theme,
  settingsActionPanel,
  isResettingConfig,
  isStartingOnboarding,
  registrationStatus,
  isRegistering,
  onUpdateStartWithWindows,
  onUpdateSettingsDraft,
  onUpdateThemePreference,
  onSetSettingsActionPanel,
  onResetConfig,
  onRerunOnboarding,
  onOpenDefaultAppsSettings,
  onRegisterBrowserIntegration,
  onUnregisterBrowserIntegration,
  onRefreshRegistrationStatus,
}: {
  startWithWindowsEnabled: boolean | null;
  isUpdatingStartWithWindows: boolean;
  settingsDraft: SettingsDraft | null;
  visibleBrowsers: BrowserConfig[];
  theme: ThemePreference;
  settingsActionPanel: SettingsActionPanel;
  isResettingConfig: boolean;
  isStartingOnboarding: boolean;
  registrationStatus: BrowserRegistrationStatus | null;
  isRegistering: boolean;
  onUpdateStartWithWindows: (enabled: boolean) => void;
  onUpdateSettingsDraft: (updater: SettingsDraftUpdater) => void;
  onUpdateThemePreference: (theme: ThemePreference) => void;
  onSetSettingsActionPanel: Dispatch<SetStateAction<SettingsActionPanel>>;
  onResetConfig: () => void;
  onRerunOnboarding: (resetFirst: boolean) => void;
  onOpenDefaultAppsSettings: () => void;
  onRegisterBrowserIntegration: () => void;
  onUnregisterBrowserIntegration: () => void;
  onRefreshRegistrationStatus: () => void;
}) {
  const [isRegistrationHelpVisible, setIsRegistrationHelpVisible] =
    useState(false);
  const [isDefaultBrowserPickerOpen, setIsDefaultBrowserPickerOpen] =
    useState(false);
  const defaultBrowserPickerRef = useRef<HTMLLabelElement | null>(null);
  const selectedDefaultBrowser =
    visibleBrowsers.find(
      (browser) => browser.id === settingsDraft?.defaultBrowserId,
    ) ?? null;

  function selectDefaultBrowser(browserId: string) {
    onUpdateSettingsDraft((current) => ({
      ...current,
      defaultBrowserId: browserId,
    }));
    setIsDefaultBrowserPickerOpen(false);
  }

  useEffect(() => {
    if (!isDefaultBrowserPickerOpen) {
      return;
    }

    function onPointerDown(event: PointerEvent) {
      const picker = defaultBrowserPickerRef.current;
      if (picker && !picker.contains(event.target as Node)) {
        setIsDefaultBrowserPickerOpen(false);
      }
    }

    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setIsDefaultBrowserPickerOpen(false);
      }
    }

    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("keydown", onKeyDown, true);
    };
  }, [isDefaultBrowserPickerOpen]);

  return (
    <section className="tab-body">
      <article className="card">
        <h3>General</h3>

        <label className="toggle">
          <input
            type="checkbox"
            checked={startWithWindowsEnabled ?? false}
            onChange={(event) =>
              onUpdateStartWithWindows(event.currentTarget.checked)
            }
            disabled={isUpdatingStartWithWindows}
          />
          <span>Start with Windows</span>
        </label>
        <p className="setting-help">
          Launch Hops when you sign in. Hops starts hidden in the tray after
          onboarding.
        </p>

        <label className="toggle">
          <input
            type="checkbox"
            checked={settingsDraft?.alwaysShowPicker ?? false}
            onChange={(event) => {
              const checked = event.currentTarget.checked;
              onUpdateSettingsDraft((current) => ({
                ...current,
                alwaysShowPicker: checked,
              }));
            }}
          />
          <span>Always show picker</span>
        </label>
        <p className="setting-help">
          If enabled, Hops skips rules and default browser and always asks you
          where to open.
        </p>

        <label className="toggle">
          <input
            type="checkbox"
            checked={settingsDraft?.useDefaultsWhenNotRunning ?? false}
            onChange={(event) => {
              const checked = event.currentTarget.checked;
              onUpdateSettingsDraft((current) => ({
                ...current,
                useDefaultsWhenNotRunning: checked,
              }));
            }}
          />
          <span>Use defaults when browser is not already running</span>
        </label>
        <p className="setting-help">
          If disabled and a matched rule browser is closed, Hops goes to picker.
          If enabled, Hops falls back to your configured default browser.
        </p>

        <label className="default-browser-picker" ref={defaultBrowserPickerRef}>
          Default browser
          <button
            type="button"
            className="settings-select-control default-browser-trigger"
            aria-expanded={isDefaultBrowserPickerOpen}
            onClick={() =>
              setIsDefaultBrowserPickerOpen((current) => !current)
            }
          >
            {selectedDefaultBrowser ? (
              <>
                <BrowserIcon iconKey={selectedDefaultBrowser.iconKey} />
                <span>{selectedDefaultBrowser.name}</span>
              </>
            ) : (
              <span>None</span>
            )}
            {isDefaultBrowserPickerOpen ? (
              <FiChevronUp aria-hidden="true" />
            ) : (
              <FiChevronDown aria-hidden="true" />
            )}
          </button>
          {isDefaultBrowserPickerOpen ? (
            <div className="default-browser-options" role="listbox">
              <button
                type="button"
                className={`default-browser-option ${
                  settingsDraft?.defaultBrowserId ? "" : "selected"
                }`}
                role="option"
                aria-selected={!settingsDraft?.defaultBrowserId}
                onClick={() => selectDefaultBrowser("")}
              >
                None
              </button>
              {visibleBrowsers.map((browser) => {
                const isSelected =
                  settingsDraft?.defaultBrowserId === browser.id;
                return (
                  <button
                    key={browser.id}
                    type="button"
                    className={`default-browser-option ${
                      isSelected ? "selected" : ""
                    }`}
                    role="option"
                    aria-selected={isSelected}
                    onClick={() => selectDefaultBrowser(browser.id)}
                  >
                    <BrowserIcon iconKey={browser.iconKey} />
                    <span>{browser.name}</span>
                  </button>
                );
              })}
            </div>
          ) : null}
        </label>
      </article>

      <article className="card">
        <h3>Style</h3>

        <label>
          Theme
          <select
            className="settings-select-control"
            value={settingsDraft?.themePreference ?? theme}
            onChange={(event) => {
              onUpdateThemePreference(
                event.currentTarget.value as ThemePreference,
              );
            }}
          >
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </label>
        <p className="setting-help">Theme changes are saved immediately.</p>

        <label className="toggle">
          <input
            type="checkbox"
            checked={settingsDraft?.disableTransparency ?? false}
            onChange={(event) => {
              const checked = event.currentTarget.checked;
              onUpdateSettingsDraft((current) => ({
                ...current,
                disableTransparency: checked,
              }));
            }}
          />
          <span>Turn off transparency in picker</span>
        </label>
        <p className="setting-help">
          Stored now for future picker styling. When picker is built, this will
          force a solid background.
        </p>
      </article>

      <article className="card">
        <h3>Configuration Recovery</h3>
        <p className="setting-help">
          Reset clears your rules, fallback browser choice, toggles, and manual
          browser entries. Detected browsers are scanned again immediately. It
          does not reopen onboarding.
        </p>

        <div className="inline-actions">
          <button
            type="button"
            className="secondary"
            onClick={() =>
              onSetSettingsActionPanel((current) =>
                current === "reset" ? "none" : "reset",
              )
            }
            disabled={isResettingConfig || isStartingOnboarding}
          >
            Reset config
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() =>
              onSetSettingsActionPanel((current) =>
                current === "rerun-onboarding" ? "none" : "rerun-onboarding",
              )
            }
            disabled={isResettingConfig || isStartingOnboarding}
          >
            Rerun onboarding
          </button>
        </div>

        {settingsActionPanel === "reset" ? (
          <div className="action-panel">
            <p className="setting-help">
              This removes your current routing rules and manual browsers and
              restores defaults without reopening onboarding.
            </p>
            <div className="inline-actions">
              <button
                type="button"
                onClick={onResetConfig}
                disabled={isResettingConfig}
              >
                {isResettingConfig ? "Resetting..." : "Confirm reset"}
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() => onSetSettingsActionPanel("none")}
                disabled={isResettingConfig}
              >
                Cancel
              </button>
            </div>
          </div>
        ) : null}

        {settingsActionPanel === "rerun-onboarding" ? (
          <div className="action-panel">
            <p className="setting-help">
              Choose whether onboarding should reuse your current configuration
              or start from a fresh reset.
            </p>
            <div className="inline-actions">
              <button
                type="button"
                onClick={() => onRerunOnboarding(false)}
                disabled={isStartingOnboarding}
              >
                {isStartingOnboarding ? "Starting..." : "Keep current config"}
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() => onRerunOnboarding(true)}
                disabled={isStartingOnboarding || isResettingConfig}
              >
                Reset first
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() => onSetSettingsActionPanel("none")}
                disabled={isStartingOnboarding}
              >
                Cancel
              </button>
            </div>
          </div>
        ) : null}
      </article>

      <article className="card">
        <h3>Windows Default App Registration</h3>
        {registrationStatus ? (
          <>
            <p className="setting-help">
              Registered in Default Apps list:{" "}
              <strong>{registrationStatus.registered ? "Yes" : "No"}</strong>
            </p>
            <p className="setting-help">
              Default for `http`:{" "}
              <strong>{registrationStatus.isDefaultHttp ? "Yes" : "No"}</strong>
              {registrationStatus.currentHttpProgId
                ? ` (current: ${registrationStatus.currentHttpProgId})`
                : ""}
            </p>
            <p className="setting-help">
              Default for `https`:{" "}
              <strong>
                {registrationStatus.isDefaultHttps ? "Yes" : "No"}
              </strong>
              {registrationStatus.currentHttpsProgId
                ? ` (current: ${registrationStatus.currentHttpsProgId})`
                : ""}
            </p>
          </>
        ) : (
          <p className="setting-help">
            Registration status is unavailable (usually because this is not
            Windows).
          </p>
        )}

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
            onClick={onRegisterBrowserIntegration}
            disabled={isRegistering}
          >
            Register Hops
          </button>
          <button
            type="button"
            className="secondary"
            onClick={onUnregisterBrowserIntegration}
            disabled={isRegistering}
          >
            Unregister Hops
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

        <button
          type="button"
          className="text-toggle"
          aria-expanded={isRegistrationHelpVisible}
          aria-controls="registration-help"
          onClick={() => setIsRegistrationHelpVisible((current) => !current)}
        >
          {isRegistrationHelpVisible ? "Show less" : "Show more"}
          {isRegistrationHelpVisible ? (
            <FiChevronUp aria-hidden="true" />
          ) : (
            <FiChevronDown aria-hidden="true" />
          )}
        </button>

        {isRegistrationHelpVisible ? (
          <div id="registration-help">
            <p className="setting-help">
              Register writes only to <code>HKCU</code> (current user) so no
              admin rights are needed. Unregister removes those same keys.
            </p>
            <p className="setting-help">
              Before unregistering, switch HTTP/HTTPS defaults away from Hops in
              Windows to avoid stale associations.
            </p>
            <p className="setting-help">
              Keys touched: <code>HKCU\Software\Classes\HopsURL</code>,{" "}
              <code>HKCU\Software\Classes\HopsHTML</code>,{" "}
              <code>HKCU\Software\Classes\Hops</code>,{" "}
              <code>HKCU\Software\Hops\Capabilities</code>,{" "}
              <code>HKCU\Software\RegisteredApplications\Hops</code>.
            </p>
          </div>
        ) : null}
      </article>
    </section>
  );
}
