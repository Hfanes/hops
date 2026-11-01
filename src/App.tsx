import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FiPlus } from "react-icons/fi";
import {
  getAppAboutInfo,
  getConfigFilePath,
  loadConfig,
  openConfigFolder,
  previewRouteWithConfig,
  resetConfig,
  routeAndOpenWithConfig,
  saveConfig,
  setStartWithWindowsEnabled as saveStartWithWindowsEnabled,
  showSettingsWindow,
  showPickerForUrl,
} from "./services/tauri";
import { LoadingState } from "./components/common/LoadingState";
import {
  DeleteConfirmModal,
  type DeleteConfirmation,
} from "./components/common/DeleteConfirmModal";
import {
  FeedbackModal,
  type FeedbackMessage,
} from "./components/common/FeedbackModal";
import { StatusBanner } from "./components/common/StatusBanner";
import { ScrollTopButton } from "./components/layout/ScrollTopButton";
import { SidebarNav } from "./components/layout/SidebarNav";
import { BottomStatusBar } from "./components/layout/BottomStatusBar";
import { Topbar } from "./components/layout/Topbar";
import { AboutPanel } from "./features/about/AboutPanel";
import { useAppUpdates } from "./features/about/useAppUpdates";
import { AddBrowserModal } from "./features/browsers/AddBrowserModal";
import { BrowsersTab } from "./features/browsers/BrowsersTab";
import { ManualBrowserConfirmModal } from "./features/browsers/ManualBrowserConfirmModal";
import { useBrowserManager } from "./features/browsers/useBrowserManager";
import { useRunningBrowsers } from "./features/browsers/useRunningBrowsers";
import { OnboardingFlow } from "./features/onboarding/OnboardingFlow";
import { RouterTester } from "./features/router/RouterTester";
import { AddRuleModal } from "./features/rules/AddRuleModal";
import { RulesTab } from "./features/rules/RulesTab";
import { useRuleManager } from "./features/rules/useRuleManager";
import { SettingsTab } from "./features/settings/SettingsTab";
import { useSystemIntegration } from "./features/settings/useSystemIntegration";
import {
  applySettingsDraft,
  EMPTY_STATUS,
  rejectAfter,
  ROUTE_OPEN_TIMEOUT_MS,
} from "./lib/appTypes";
import { useConfigStore } from "./hooks/useConfigStore";
import { useDocumentTheme } from "./hooks/useDocumentTheme";
import { useSidebarShortcut } from "./hooks/useSidebarShortcut";
import type {
  AppConfig,
  FormModal,
  RouteDecision,
  SettingsActionPanel,
  StatusState,
  TabId,
} from "./types";
import "./styles/App.css";

function App() {
  const [activeTab, setActiveTab] = useState<TabId>("settings");
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [status, setStatus] = useState<StatusState>(EMPTY_STATUS);
  const [isLoading, setIsLoading] = useState(true);
  const [isResettingConfig, setIsResettingConfig] = useState(false);
  const [isStartingOnboarding, setIsStartingOnboarding] = useState(false);
  const [configPath, setConfigPath] = useState<string | null>(null);
  const [onboardingStep, setOnboardingStep] = useState(0);
  const [onboardingStartWithWindows, setOnboardingStartWithWindows] =
    useState(true);
  const [isFinishingOnboarding, setIsFinishingOnboarding] = useState(false);
  const [isCheckingOnboardingDefaults, setIsCheckingOnboardingDefaults] =
    useState(false);
  const [settingsActionPanel, setSettingsActionPanel] =
    useState<SettingsActionPanel>("none");
  const [formModal, setFormModal] = useState<FormModal>(null);

  const [feedbackModal, setFeedbackModal] = useState<FeedbackMessage | null>(
    null,
  );
  const [deleteConfirmation, setDeleteConfirmation] =
    useState<DeleteConfirmation | null>(null);

  const [routeInput, setRouteInput] = useState(
    "https://github.com/openai/codex",
  );
  const [routeDecision, setRouteDecision] = useState<RouteDecision | null>(
    null,
  );
  const [isRouting, setIsRouting] = useState(false);
  const isClosingRef = useRef(false);
  const contentAreaRef = useRef<HTMLDivElement | null>(null);

  const configStore = useConfigStore({ setStatus });
  const {
    config,
    configRef,
    theme,
    settingsDraft,
    isSaving,
    applyLoadedConfig,
    persistConfig,
    scheduleSettingsSave,
    cancelScheduledSettingsSave,
    waitForQueuedSaves,
    updateSettingsDraft,
    updateThemePreference,
  } = configStore;
  const {
    aboutInfo,
    setAboutInfo,
    updateStatus,
    isCheckingUpdate,
    isInstallingUpdate,
    refreshAbout,
    updateApp,
  } = useAppUpdates({ activeTab, isLoading, setStatus });
  const {
    registrationStatus,
    isRegistering,
    startWithWindowsEnabled,
    setStartWithWindowsEnabled,
    isUpdatingStartWithWindows,
    refreshRegistrationStatus,
    refreshStartWithWindowsStatus,
    updateStartWithWindows,
    openDefaultAppsSettings,
    registerBrowserIntegration,
    unregisterBrowserIntegration,
  } = useSystemIntegration({ setStatus });
  const { runningBrowserIds, refreshRunningState } = useRunningBrowsers({
    activeTab,
    isLoading,
    configRef,
  });
  const {
    isRefreshing,
    pendingBrowserIds,
    savingBrowserIds,
    failedBrowserIds,
    browserDraft,
    setBrowserDraft,
    manualBrowserConfirmation,
    setManualBrowserConfirmation,
    isConfirmingManualBrowser,
    handleRefreshBrowsers,
    updateBrowser,
    flushBrowserSave,
    toggleBrowserHidden,
    requestDeleteManualBrowser,
    deleteManualBrowser,
    addManualBrowser,
    confirmManualBrowser,
    cancelPendingBrowserSaves,
    resetBrowserEditState,
  } = useBrowserManager({
    store: configStore,
    setStatus,
    setFormModal,
    showFeedbackModal,
    onRequestDelete: setDeleteConfirmation,
    refreshRunningState,
  });

  const toggleSidebar = useCallback(() => {
    setIsSidebarCollapsed((current) => !current);
  }, []);

  useDocumentTheme(theme);
  useSidebarShortcut(toggleSidebar);

  useEffect(() => {
    const bootstrap = async () => {
      setIsLoading(true);
      try {
        const loaded = await loadConfig();
        applyLoadedConfig(loaded);
        setStatus({ kind: "success", text: "Configuration loaded." });

        const [loadedConfigPath, nextAboutInfo] = await Promise.all([
          getConfigFilePath(),
          getAppAboutInfo(),
          refreshRegistrationStatus(),
          refreshStartWithWindowsStatus(),
        ]);
        setConfigPath(loadedConfigPath);
        setAboutInfo(nextAboutInfo);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setStatus({ kind: "error", text: `Failed to load config: ${message}` });
      } finally {
        setIsLoading(false);
      }
    };

    void bootstrap();
  }, []);

  const visibleBrowsers = useMemo(
    () => config?.browsers.filter((browser) => !browser.isHidden) ?? [],
    [config],
  );
  const {
    dirtyRuleIds,
    savingRuleIds,
    ruleDraft,
    setRuleDraft,
    regexErrors,
    updateRule,
    saveRule,
    deleteRule,
    requestDeleteRule,
    moveRule,
    addRule,
    resetRuleEditState,
  } = useRuleManager({
    store: configStore,
    visibleBrowsers,
    setStatus,
    setFormModal,
    onRequestDelete: setDeleteConfirmation,
  });

  useEffect(() => {
    if (!config || config.onboardingCompleted) {
      return;
    }

    window.scrollTo({ top: 0, left: 0, behavior: "auto" });
  }, [config, onboardingStep]);

  useEffect(() => {
    setFormModal(null);
  }, [activeTab]);

  useEffect(() => {
    let disposed = false;
    let unlistenClose: (() => void) | null = null;

    void getCurrentWindow()
      .onCloseRequested((event) => {
        event.preventDefault();
        void closeSettingsWindow();
      })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlistenClose = unlisten;
      })
      .catch((error) => {
        console.warn("Could not listen for Hops close requests.", error);
      });

    return () => {
      disposed = true;
      unlistenClose?.();
    };
  }, []);

  function showFeedbackModal(options: FeedbackMessage) {
    setFeedbackModal(options);
  }

  async function closeSettingsWindow() {
    if (isClosingRef.current) {
      return;
    }

    isClosingRef.current = true;
    const hasPendingSettingsSave = cancelScheduledSettingsSave();
    const hasPendingBrowserSaves = cancelPendingBrowserSaves();

    try {
      const latestConfig = configRef.current;
      if (
        latestConfig &&
        (hasPendingSettingsSave || hasPendingBrowserSaves)
      ) {
        await persistConfig(latestConfig, {
          errorPrefix: "Could not save changes before closing",
          applySavedConfig: false,
        });
      }

      await waitForQueuedSaves();
      await getCurrentWindow().destroy();
    } catch {
      isClosingRef.current = false;
      if (configRef.current) {
        scheduleSettingsSave(configRef.current);
      }
    }
  }

  function confirmDelete() {
    const confirmation = deleteConfirmation;
    if (!confirmation) {
      return;
    }

    setDeleteConfirmation(null);
    if (confirmation.kind === "manual-browser") {
      deleteManualBrowser(confirmation.browserId);
      return;
    }

    deleteRule(confirmation.ruleId);
  }

  async function openConfigurationFolder() {
    try {
      await openConfigFolder();
      setStatus({
        kind: "success",
        text: "Opened config folder.",
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not open config folder: ${message}`,
      });
    }
  }

  async function finishOnboarding() {
    if (!config) {
      return;
    }

    const nextConfig: AppConfig = {
      ...config,
      onboardingCompleted: true,
    };

    setIsFinishingOnboarding(true);
    try {
      const enabled = await saveStartWithWindowsEnabled(
        onboardingStartWithWindows,
      );
      setStartWithWindowsEnabled(enabled);

      const saved = await saveConfig(nextConfig);
      applyLoadedConfig(saved);
      setActiveTab("settings");
      await showSettingsWindow();
      setStatus({
        kind: "success",
        text: onboardingStartWithWindows
          ? "Onboarding completed. Hops will start with Windows. Review your settings before closing this window."
          : "Onboarding completed. Review your settings before closing this window.",
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not finish onboarding: ${message}`,
      });
    } finally {
      setIsFinishingOnboarding(false);
    }
  }

  async function continueToOnboardingFinish() {
    setIsCheckingOnboardingDefaults(true);
    try {
      const next = await refreshRegistrationStatus();

      if (!next) {
        setStatus({
          kind: "warning",
          text: "Could not check Windows HTTP/HTTPS defaults. You can refresh status or finish setup later in Settings.",
        });
      }

      setOnboardingStep(3);
    } finally {
      setIsCheckingOnboardingDefaults(false);
    }
  }

  async function handleResetConfig() {
    setIsResettingConfig(true);
    try {
      const reset = await resetConfig();
      setSettingsActionPanel("none");
      applyLoadedConfig(reset);
      resetRuleEditState();
      resetBrowserEditState();
      setRouteDecision(null);
      setStatus({
        kind: "success",
        text: "Configuration reset. Rules and manual browsers were cleared, and detected browsers were restored.",
      });
      await refreshRunningState({ force: true });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not reset configuration: ${message}`,
      });
    } finally {
      setIsResettingConfig(false);
    }
  }

  async function handleRerunOnboarding(shouldResetFirst: boolean) {
    if (!config) {
      return;
    }

    setIsStartingOnboarding(true);
    try {
      const nextConfig = shouldResetFirst
        ? await saveConfig({
            ...(await resetConfig()),
            onboardingCompleted: false,
          })
        : await saveConfig({
            ...applySettingsDraft(config, settingsDraft),
            onboardingCompleted: false,
          });

      setOnboardingStep(0);
      setSettingsActionPanel("none");
      applyLoadedConfig(nextConfig);
      setStatus({
        kind: "success",
        text: shouldResetFirst
          ? "Configuration reset. Onboarding restarted from step 1."
          : "Onboarding restarted with your current configuration intact.",
      });
      await refreshRunningState({ force: true });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not restart onboarding: ${message}`,
      });
    } finally {
      setIsStartingOnboarding(false);
    }
  }

  async function runRoutePreview(openImmediately: boolean) {
    if (!config) {
      setStatus({ kind: "error", text: "Configuration is not loaded yet." });
      return;
    }

    if (!routeInput.trim()) {
      setStatus({ kind: "error", text: "Enter a URL to test routing." });
      return;
    }

    setIsRouting(true);
    try {
      const trimmedRouteInput = routeInput.trim();
      const decision = openImmediately
        ? await rejectAfter(
            routeAndOpenWithConfig(config, trimmedRouteInput),
            ROUTE_OPEN_TIMEOUT_MS,
            "Opening the routed browser timed out after 10 seconds.",
          )
        : await previewRouteWithConfig(config, trimmedRouteInput);

      setRouteDecision(decision);
      if (openImmediately && decision.action === "open_browser") {
        setStatus({
          kind: "success",
          text: `Opened in ${decision.browserName ?? "selected browser"}.`,
        });
      } else if (openImmediately && decision.action === "show_picker") {
        await rejectAfter(
          showPickerForUrl(
            trimmedRouteInput,
            decision.browserId,
            decision.privateMode,
          ),
          ROUTE_OPEN_TIMEOUT_MS,
          "Opening the picker timed out after 10 seconds.",
        );
        setStatus({
          kind: "success",
          text: "Routing requires the picker. The picker window was opened.",
        });
      } else {
        setStatus({ kind: "success", text: "Routing preview updated." });
      }
      await refreshRunningState({ force: true });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({ kind: "error", text: `Routing failed: ${message}` });
      if (
        openImmediately &&
        error instanceof Error &&
        error.name === "TimeoutError"
      ) {
        showFeedbackModal({
          title: "Route opening timed out",
          message: `Hops could not open the route within 10 seconds.\n\n${message}`,
          kind: "warning",
        });
      }
    } finally {
      setIsRouting(false);
    }
  }

  function scrollCurrentPageToTop() {
    contentAreaRef.current?.scrollTo({ top: 0, behavior: "smooth" });
    window.scrollTo({ top: 0, left: 0, behavior: "smooth" });
  }

  const shellClassName = "app-shell";
  const panelClassName = `app-panel ${
    isSidebarCollapsed ? "sidebar-collapsed" : ""
  }`;
  const onboardingPanelClassName = "onboarding-panel";
  const sidebarClassName = "app-sidebar";
  const contentClassName = "content-area";
  const topbarClassName = "topbar";

  const sidebarNav = (
    <SidebarNav
      activeTab={activeTab}
      className={sidebarClassName}
      isCollapsed={isSidebarCollapsed}
      theme={theme}
      onCollapseToggle={toggleSidebar}
      onTabChange={setActiveTab}
      onThemeToggle={() =>
        updateThemePreference(theme === "light" ? "dark" : "light")
      }
    />
  );

  const topButton = <ScrollTopButton onClick={scrollCurrentPageToTop} />;

  const bottomStatusBar = (
    <BottomStatusBar
      configPath={configPath}
      updateStatus={updateStatus}
      isCheckingUpdate={isCheckingUpdate}
      appVersion={aboutInfo?.version ?? null}
    />
  );

  const statusBanner = (
    <StatusBanner status={status} onDismiss={() => setStatus(EMPTY_STATUS)} />
  );

  const activeFormModal =
    formModal === "manual-browser" ? (
      <AddBrowserModal
        browserDraft={browserDraft}
        isSaving={isSaving}
        onSetBrowserDraft={setBrowserDraft}
        onAdd={() => void addManualBrowser()}
        onClose={() => setFormModal(null)}
      />
    ) : formModal === "rule" ? (
      <AddRuleModal
        ruleDraft={ruleDraft}
        visibleBrowsers={visibleBrowsers}
        isSaving={isSaving}
        onSetRuleDraft={setRuleDraft}
        onAdd={() => void addRule()}
        onClose={() => setFormModal(null)}
      />
    ) : null;

  const manualBrowserConfirmationModal = manualBrowserConfirmation ? (
    <ManualBrowserConfirmModal
      confirmation={manualBrowserConfirmation}
      isConfirming={isConfirmingManualBrowser}
      onConfirm={() => void confirmManualBrowser()}
      onClose={() => setManualBrowserConfirmation(null)}
    />
  ) : null;

  const deleteConfirmationModal = deleteConfirmation ? (
    <DeleteConfirmModal
      confirmation={deleteConfirmation}
      onConfirm={confirmDelete}
      onClose={() => setDeleteConfirmation(null)}
    />
  ) : null;

  const feedbackModalElement = feedbackModal ? (
    <FeedbackModal
      feedback={feedbackModal}
      onClose={() => setFeedbackModal(null)}
    />
  ) : null;

  if (isLoading) {
    return (
      <LoadingState
        shellClassName={shellClassName}
        message="Loading configuration..."
      />
    );
  }

  if (!config) {
    return (
      <LoadingState
        shellClassName={shellClassName}
        message="Could not load configuration. Check the status banner and try again."
      />
    );
  }

  if (!config.onboardingCompleted) {
    return (
      <main className={shellClassName}>
        <section className={onboardingPanelClassName}>
          <div className={contentClassName} ref={contentAreaRef}>
            <header className={topbarClassName}>
              <div>
                <p className="section-label">Hops Setup</p>
                <h1>Welcome to Hops</h1>
                <p>Quick onboarding to make external links route correctly.</p>
              </div>
              <p className="badge">Step {onboardingStep + 1} of 4</p>
            </header>

            <OnboardingFlow
              statusBanner={statusBanner}
              config={config}
              onboardingStep={onboardingStep}
              browserDraft={browserDraft}
              isRefreshing={isRefreshing}
              isRegistering={isRegistering}
              registrationStatus={registrationStatus}
              isCheckingOnboardingDefaults={isCheckingOnboardingDefaults}
              onboardingStartWithWindows={onboardingStartWithWindows}
              isFinishingOnboarding={isFinishingOnboarding}
              onRefreshBrowsers={handleRefreshBrowsers}
              onSetBrowserDraft={setBrowserDraft}
              onAddManualBrowser={() => void addManualBrowser()}
              onSetOnboardingStep={setOnboardingStep}
              onRegisterBrowserIntegration={() =>
                void registerBrowserIntegration()
              }
              onRefreshRegistrationStatus={() =>
                void refreshRegistrationStatus()
              }
              onOpenDefaultAppsSettings={openDefaultAppsSettings}
              onContinueToFinish={() => void continueToOnboardingFinish()}
              onSetOnboardingStartWithWindows={setOnboardingStartWithWindows}
              onFinishOnboarding={() => void finishOnboarding()}
            />
          </div>
        </section>
        {bottomStatusBar}
      </main>
    );
  }

  return (
    <main className={shellClassName}>
      <section className={panelClassName}>
        {sidebarNav}

        <div className={contentClassName} ref={contentAreaRef}>
          <Topbar
            activeTab={activeTab}
            className={topbarClassName}
            actions={
              <>
                {activeTab === "browsers" ? (
                  <>
                    <button
                      type="button"
                      className="secondary"
                      onClick={handleRefreshBrowsers}
                      disabled={isRefreshing}
                    >
                      {isRefreshing ? "Refreshing..." : "Refresh browsers"}
                    </button>
                    <button
                      type="button"
                      onClick={() => setFormModal("manual-browser")}
                    >
                      <FiPlus aria-hidden="true" />
                      Add browser
                    </button>
                  </>
                ) : null}
                {activeTab === "rules" ? (
                  <button
                    type="button"
                    onClick={() => setFormModal("rule")}
                    disabled={!visibleBrowsers.length}
                  >
                    <FiPlus aria-hidden="true" />
                    Add rule
                  </button>
                ) : null}
              </>
            }
          />

          {activeFormModal}
          {manualBrowserConfirmationModal}
          {deleteConfirmationModal}
          {feedbackModalElement}

          {statusBanner}

          {activeTab === "settings" ? (
            <SettingsTab
              startWithWindowsEnabled={startWithWindowsEnabled}
              isUpdatingStartWithWindows={isUpdatingStartWithWindows}
              settingsDraft={settingsDraft}
              visibleBrowsers={visibleBrowsers}
              theme={theme}
              settingsActionPanel={settingsActionPanel}
              isResettingConfig={isResettingConfig}
              isStartingOnboarding={isStartingOnboarding}
              registrationStatus={registrationStatus}
              isRegistering={isRegistering}
              onUpdateStartWithWindows={(enabled) =>
                void updateStartWithWindows(enabled)
              }
              onUpdateSettingsDraft={updateSettingsDraft}
              onUpdateThemePreference={updateThemePreference}
              onSetSettingsActionPanel={setSettingsActionPanel}
              onResetConfig={() => void handleResetConfig()}
              onRerunOnboarding={(resetFirst) =>
                void handleRerunOnboarding(resetFirst)
              }
              onOpenConfigFolder={() => void openConfigurationFolder()}
              onOpenDefaultAppsSettings={openDefaultAppsSettings}
              onRegisterBrowserIntegration={() =>
                void registerBrowserIntegration()
              }
              onUnregisterBrowserIntegration={() =>
                void unregisterBrowserIntegration()
              }
              onRefreshRegistrationStatus={() =>
                void refreshRegistrationStatus()
              }
            />
          ) : null}

          {activeTab === "browsers" ? (
            <BrowsersTab
              browsers={config.browsers}
              runningBrowserIds={runningBrowserIds}
              savingBrowserIds={savingBrowserIds}
              pendingBrowserIds={pendingBrowserIds}
              failedBrowserIds={failedBrowserIds}
              onUpdateBrowser={updateBrowser}
              onFlushBrowserSave={flushBrowserSave}
              onToggleBrowserHidden={toggleBrowserHidden}
              onDeleteManualBrowser={requestDeleteManualBrowser}
            />
          ) : null}

          {activeTab === "rules" ? (
            <RulesTab
              rules={config.rules}
              visibleBrowsers={visibleBrowsers}
              regexErrors={regexErrors}
              dirtyRuleIds={dirtyRuleIds}
              savingRuleIds={savingRuleIds}
              onMoveRule={moveRule}
              onDeleteRule={requestDeleteRule}
              onUpdateRule={updateRule}
              onSaveRule={saveRule}
            />
          ) : null}

          {activeTab === "router" ? (
            <RouterTester
              routeDecision={routeDecision}
              routeInput={routeInput}
              isRouting={isRouting}
              onRouteInputChange={setRouteInput}
              onRunRoutePreview={(shouldOpen) =>
                void runRoutePreview(shouldOpen)
              }
            />
          ) : null}

          {activeTab === "about" ? (
            <AboutPanel
              aboutInfo={aboutInfo}
              updateStatus={updateStatus}
              isCheckingUpdate={isCheckingUpdate}
              isInstallingUpdate={isInstallingUpdate}
              onRefreshAbout={() => void refreshAbout()}
              onUpdateApp={() => void updateApp()}
            />
          ) : null}
          {topButton}
        </div>
      </section>
      {bottomStatusBar}
    </main>
  );
}

export default App;
