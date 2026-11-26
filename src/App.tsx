import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { FiPlus } from "react-icons/fi";
import {
  getAppAboutInfo,
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
import { useOnboarding } from "./features/onboarding/useOnboarding";
import { RouterTester } from "./features/router/RouterTester";
import { useRouteTester } from "./features/router/useRouteTester";
import { AddRuleModal } from "./features/rules/AddRuleModal";
import { RulesTab } from "./features/rules/RulesTab";
import { useRuleManager } from "./features/rules/useRuleManager";
import { SettingsTab } from "./features/settings/SettingsTab";
import { useConfigActions } from "./features/settings/useConfigActions";
import { useSystemIntegration } from "./features/settings/useSystemIntegration";
import {
  EMPTY_STATUS,
} from "./lib/appTypes";
import { useAppBootstrap } from "./hooks/useAppBootstrap";
import { useCloseGuard } from "./hooks/useCloseGuard";
import { useConfigStore } from "./hooks/useConfigStore";
import { useDocumentTheme } from "./hooks/useDocumentTheme";
import { useSidebarShortcut } from "./hooks/useSidebarShortcut";
import type {
  FormModal,
  StatusState,
  TabId,
} from "./types";
import "./styles/App.css";

function App() {
  const [activeTab, setActiveTab] = useState<TabId>("settings");
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [status, setStatus] = useState<StatusState>(EMPTY_STATUS);
  const [formModal, setFormModal] = useState<FormModal>(null);

  const [feedbackModal, setFeedbackModal] = useState<FeedbackMessage | null>(
    null,
  );
  const [deleteConfirmation, setDeleteConfirmation] =
    useState<DeleteConfirmation | null>(null);

  const contentAreaRef = useRef<HTMLDivElement | null>(null);

  const configStore = useConfigStore({ setStatus });
  const {
    config,
    configRef,
    theme,
    settingsDraft,
    isSaving,
    applyLoadedConfig,
    updateSettingsDraft,
    updateThemePreference,
  } = configStore;
  const { isLoading, configPath } = useAppBootstrap({
    applyLoadedConfig,
    setStatus,
    loadStartupDetails,
  });
  const {
    aboutInfo,
    setAboutInfo,
    updateStatus,
    isCheckingUpdate,
    isInstallingUpdate,
    refreshAbout,
    updateApp,
  } = useAppUpdates({ activeTab, isLoading, setStatus });
  const integration = useSystemIntegration({ setStatus });
  const onboarding = useOnboarding({
    config,
    applyLoadedConfig,
    setStatus,
    setStartWithWindowsEnabled: integration.setStartWithWindowsEnabled,
    refreshRegistrationStatus: integration.refreshRegistrationStatus,
    onFinished: () => setActiveTab("settings"),
  });
  const { runningBrowserIds, missingBrowserIds, refreshRunningState } =
    useRunningBrowsers({
    activeTab,
    isLoading,
    configRef,
  });
  const browsers = useBrowserManager({
    store: configStore,
    setStatus,
    setFormModal,
    showFeedbackModal,
    onRequestDelete: setDeleteConfirmation,
    refreshRunningState,
  });
  useCloseGuard({
    store: configStore,
    cancelPendingBrowserSaves: browsers.cancelPendingBrowserSaves,
  });
  const routeTester = useRouteTester({
    config,
    setStatus,
    showFeedbackModal,
    refreshRunningState,
  });

  const toggleSidebar = useCallback(() => {
    setIsSidebarCollapsed((current) => !current);
  }, []);

  useDocumentTheme(theme);
  useSidebarShortcut(toggleSidebar);

  const visibleBrowsers = useMemo(
    () => config?.browsers.filter((browser) => !browser.isHidden) ?? [],
    [config],
  );
  const rules = useRuleManager({
    store: configStore,
    visibleBrowsers,
    setStatus,
    setFormModal,
    onRequestDelete: setDeleteConfirmation,
  });
  const configActions = useConfigActions({
    store: configStore,
    setStatus,
    cancelPendingBrowserSaves: browsers.cancelPendingBrowserSaves,
    refreshRunningState,
    onConfigReset: () => {
      rules.resetRuleEditState();
      browsers.resetBrowserEditState();
      routeTester.clearRouteDecision();
    },
    onOnboardingRestarted: () => onboarding.setStep(0),
  });

  useEffect(() => {
    setFormModal(null);
  }, [activeTab]);

  // Declared as a function so useAppBootstrap, which runs before the hooks it
  // uses, can take it. It is only called from the bootstrap effect.
  async function loadStartupDetails() {
    const [nextAboutInfo] = await Promise.all([
      getAppAboutInfo(),
      integration.refreshRegistrationStatus(),
      integration.refreshStartWithWindowsStatus(),
    ]);
    return () => setAboutInfo(nextAboutInfo);
  }

  function showFeedbackModal(options: FeedbackMessage) {
    setFeedbackModal(options);
  }

  function confirmDelete() {
    const confirmation = deleteConfirmation;
    if (!confirmation) {
      return;
    }

    setDeleteConfirmation(null);
    if (confirmation.kind === "manual-browser") {
      browsers.deleteManualBrowser(confirmation.browserId);
      return;
    }

    rules.deleteRule(confirmation.ruleId);
  }

  function scrollCurrentPageToTop() {
    contentAreaRef.current?.scrollTo({ top: 0, behavior: "smooth" });
    window.scrollTo({ top: 0, left: 0, behavior: "smooth" });
  }

  const sidebarNav = (
    <SidebarNav
      activeTab={activeTab}
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
        browserDraft={browsers.browserDraft}
        isSaving={isSaving}
        onSetBrowserDraft={browsers.setBrowserDraft}
        onAdd={() => void browsers.addManualBrowser()}
        onClose={() => setFormModal(null)}
      />
    ) : formModal === "rule" ? (
      <AddRuleModal
        ruleDraft={rules.ruleDraft}
        visibleBrowsers={visibleBrowsers}
        isSaving={isSaving}
        onSetRuleDraft={rules.setRuleDraft}
        onAdd={() => void rules.addRule()}
        onClose={() => setFormModal(null)}
      />
    ) : null;

  const manualBrowserConfirmationModal = browsers.manualBrowserConfirmation ? (
    <ManualBrowserConfirmModal
      confirmation={browsers.manualBrowserConfirmation}
      isConfirming={browsers.isConfirmingManualBrowser}
      onConfirm={() => void browsers.confirmManualBrowser()}
      onClose={() => browsers.setManualBrowserConfirmation(null)}
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
        message="Loading configuration..."
      />
    );
  }

  if (!config) {
    return (
      <LoadingState
        message="Could not load configuration. Check the status banner and try again."
      />
    );
  }

  if (!config.onboardingCompleted) {
    return (
      <main className="app-shell">
        <section className="onboarding-panel">
          <div className="content-area" ref={contentAreaRef}>
            <header className="topbar">
              <div>
                <p className="section-label">Hops Setup</p>
                <h1>Welcome to Hops</h1>
                <p>Quick onboarding to make external links route correctly.</p>
              </div>
              <p className="badge">Step {onboarding.step + 1} of 4</p>
            </header>

            <OnboardingFlow
              statusBanner={statusBanner}
              config={config}
              onboarding={onboarding}
              integration={integration}
              browsers={browsers}
            />
          </div>
        </section>
        {bottomStatusBar}
      </main>
    );
  }

  return (
    <main className="app-shell">
      <section
        className={`app-panel ${isSidebarCollapsed ? "sidebar-collapsed" : ""}`}
      >
        {sidebarNav}

        <div className="content-area" ref={contentAreaRef}>
          <Topbar
            activeTab={activeTab}
            actions={
              <>
                {activeTab === "browsers" ? (
                  <>
                    <button
                      type="button"
                      className="secondary"
                      onClick={browsers.handleRefreshBrowsers}
                      disabled={browsers.isRefreshing}
                    >
                      {browsers.isRefreshing ? "Refreshing..." : "Refresh browsers"}
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
              settingsDraft={settingsDraft}
              visibleBrowsers={visibleBrowsers}
              theme={theme}
              integration={integration}
              configActions={configActions}
              onUpdateSettingsDraft={updateSettingsDraft}
              onUpdateThemePreference={updateThemePreference}
            />
          ) : null}

          {activeTab === "browsers" ? (
            <BrowsersTab
              browsers={config.browsers}
              runningBrowserIds={runningBrowserIds}
              missingBrowserIds={missingBrowserIds}
              savingBrowserIds={browsers.savingBrowserIds}
              pendingBrowserIds={browsers.pendingBrowserIds}
              failedBrowserIds={browsers.failedBrowserIds}
              onUpdateBrowser={browsers.updateBrowser}
              onFlushBrowserSave={browsers.flushBrowserSave}
              onToggleBrowserHidden={browsers.toggleBrowserHidden}
              onDeleteManualBrowser={browsers.requestDeleteManualBrowser}
            />
          ) : null}

          {activeTab === "rules" ? (
            <RulesTab
              rules={config.rules}
              visibleBrowsers={visibleBrowsers}
              regexErrors={rules.regexErrors}
              dirtyRuleIds={rules.dirtyRuleIds}
              savingRuleIds={rules.savingRuleIds}
              onMoveRule={rules.moveRule}
              onDeleteRule={rules.requestDeleteRule}
              onUpdateRule={rules.updateRule}
              onSaveRule={rules.saveRule}
            />
          ) : null}

          {activeTab === "router" ? (
            <RouterTester
              routeDecision={routeTester.routeDecision}
              routeInput={routeTester.routeInput}
              isRouting={routeTester.isRouting}
              onRouteInputChange={routeTester.setRouteInput}
              onRunRoutePreview={(shouldOpen) =>
                void routeTester.runRoutePreview(shouldOpen)
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
