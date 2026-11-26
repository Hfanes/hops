import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AppModals } from "./AppModals";
import type { DeleteConfirmation } from "./components/common/DeleteConfirmModal";
import type { FeedbackMessage } from "./components/common/FeedbackModal";
import { LoadingState } from "./components/common/LoadingState";
import { StatusBanner } from "./components/common/StatusBanner";
import { BottomStatusBar } from "./components/layout/BottomStatusBar";
import { MainScreen } from "./components/layout/MainScreen";
import { OnboardingScreen } from "./components/layout/OnboardingScreen";
import { ScrollTopButton } from "./components/layout/ScrollTopButton";
import { SidebarNav } from "./components/layout/SidebarNav";
import { Topbar } from "./components/layout/Topbar";
import { TopbarActions } from "./components/layout/TopbarActions";
import { AboutPanel } from "./features/about/AboutPanel";
import { useAppUpdates } from "./features/about/useAppUpdates";
import { BrowsersTab } from "./features/browsers/BrowsersTab";
import { useBrowserManager } from "./features/browsers/useBrowserManager";
import { useRunningBrowsers } from "./features/browsers/useRunningBrowsers";
import { OnboardingFlow } from "./features/onboarding/OnboardingFlow";
import { useOnboarding } from "./features/onboarding/useOnboarding";
import { RouterTester } from "./features/router/RouterTester";
import { useRouteTester } from "./features/router/useRouteTester";
import { RulesTab } from "./features/rules/RulesTab";
import { useRuleManager } from "./features/rules/useRuleManager";
import { SettingsTab } from "./features/settings/SettingsTab";
import { useConfigActions } from "./features/settings/useConfigActions";
import { useSystemIntegration } from "./features/settings/useSystemIntegration";
import { useAppBootstrap } from "./hooks/useAppBootstrap";
import { useCloseGuard } from "./hooks/useCloseGuard";
import { useConfigStore } from "./hooks/useConfigStore";
import { useDocumentTheme } from "./hooks/useDocumentTheme";
import { useSidebarShortcut } from "./hooks/useSidebarShortcut";
import { EMPTY_STATUS } from "./lib/appTypes";
import { getAppAboutInfo } from "./services/tauri";
import type { FormModal, StatusState, TabId } from "./types";
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
  const { config, configRef, theme, applyLoadedConfig } = configStore;
  const { isLoading, configPath } = useAppBootstrap({
    applyLoadedConfig,
    setStatus,
    loadStartupDetails,
  });
  const updates = useAppUpdates({ activeTab, isLoading, setStatus });
  const integration = useSystemIntegration({ setStatus });
  const onboarding = useOnboarding({
    config,
    applyLoadedConfig,
    setStatus,
    setStartWithWindowsEnabled: integration.setStartWithWindowsEnabled,
    refreshRegistrationStatus: integration.refreshRegistrationStatus,
    onFinished: () => setActiveTab("settings"),
  });
  const running = useRunningBrowsers({ activeTab, isLoading, configRef });
  const { refreshRunningState } = running;
  const browsers = useBrowserManager({
    store: configStore,
    setStatus,
    setFormModal,
    showFeedbackModal: setFeedbackModal,
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
    showFeedbackModal: setFeedbackModal,
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
    return () => updates.setAboutInfo(nextAboutInfo);
  }

  function scrollCurrentPageToTop() {
    contentAreaRef.current?.scrollTo({ top: 0, behavior: "smooth" });
    window.scrollTo({ top: 0, left: 0, behavior: "smooth" });
  }

  const bottomStatusBar = (
    <BottomStatusBar
      configPath={configPath}
      updateStatus={updates.updateStatus}
      isCheckingUpdate={updates.isCheckingUpdate}
      appVersion={updates.aboutInfo?.version ?? null}
    />
  );

  const statusBanner = (
    <StatusBanner status={status} onDismiss={() => setStatus(EMPTY_STATUS)} />
  );

  if (isLoading) {
    return <LoadingState message="Loading configuration..." />;
  }

  if (!config) {
    return (
      <LoadingState message="Could not load configuration. Check the status banner and try again." />
    );
  }

  if (!config.onboardingCompleted) {
    return (
      <OnboardingScreen
        step={onboarding.step}
        contentAreaRef={contentAreaRef}
        footer={bottomStatusBar}
      >
        <OnboardingFlow
          statusBanner={statusBanner}
          config={config}
          onboarding={onboarding}
          integration={integration}
          browsers={browsers}
        />
      </OnboardingScreen>
    );
  }

  return (
    <MainScreen
      isSidebarCollapsed={isSidebarCollapsed}
      contentAreaRef={contentAreaRef}
      footer={bottomStatusBar}
      sidebar={
        <SidebarNav
          activeTab={activeTab}
          isCollapsed={isSidebarCollapsed}
          theme={theme}
          onCollapseToggle={toggleSidebar}
          onTabChange={setActiveTab}
          onThemeToggle={() =>
            configStore.updateThemePreference(
              theme === "light" ? "dark" : "light",
            )
          }
        />
      }
    >
      <Topbar
        activeTab={activeTab}
        actions={
          <TopbarActions
            activeTab={activeTab}
            isRefreshing={browsers.isRefreshing}
            canAddRule={visibleBrowsers.length > 0}
            onRefreshBrowsers={() => void browsers.handleRefreshBrowsers()}
            onAddBrowser={() => setFormModal("manual-browser")}
            onAddRule={() => setFormModal("rule")}
          />
        }
      />

      <AppModals
        formModal={formModal}
        onCloseFormModal={() => setFormModal(null)}
        browsers={browsers}
        rules={rules}
        visibleBrowsers={visibleBrowsers}
        isSaving={configStore.isSaving}
        deleteConfirmation={deleteConfirmation}
        onCloseDeleteConfirmation={() => setDeleteConfirmation(null)}
        feedback={feedbackModal}
        onCloseFeedback={() => setFeedbackModal(null)}
      />

      {statusBanner}

      {activeTab === "settings" ? (
        <SettingsTab
          settingsDraft={configStore.settingsDraft}
          visibleBrowsers={visibleBrowsers}
          theme={theme}
          integration={integration}
          configActions={configActions}
          onUpdateSettingsDraft={configStore.updateSettingsDraft}
          onUpdateThemePreference={configStore.updateThemePreference}
        />
      ) : null}

      {activeTab === "browsers" ? (
        <BrowsersTab
          browsers={config.browsers}
          runningBrowserIds={running.runningBrowserIds}
          missingBrowserIds={running.missingBrowserIds}
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
          aboutInfo={updates.aboutInfo}
          updateStatus={updates.updateStatus}
          isCheckingUpdate={updates.isCheckingUpdate}
          isInstallingUpdate={updates.isInstallingUpdate}
          onRefreshAbout={() => void updates.refreshAbout()}
          onUpdateApp={() => void updates.updateApp()}
        />
      ) : null}
      <ScrollTopButton onClick={scrollCurrentPageToTop} />
    </MainScreen>
  );
}

export default App;
