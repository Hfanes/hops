import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FiPlus } from "react-icons/fi";
import {
  getAppAboutInfo,
  getConfigFilePath,
  loadConfig,
  openConfigFolder,
  previewRouteWithConfig,
  refreshBrowsers,
  resetConfig,
  routeAndOpenWithConfig,
  saveConfig,
  setStartWithWindowsEnabled as saveStartWithWindowsEnabled,
  showSettingsWindow,
  showPickerForUrl,
  validateManualBrowser,
  validateRegexPatterns,
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
import {
  ManualBrowserConfirmModal,
  type ManualBrowserConfirmation,
} from "./features/browsers/ManualBrowserConfirmModal";
import { useRunningBrowsers } from "./features/browsers/useRunningBrowsers";
import { OnboardingFlow } from "./features/onboarding/OnboardingFlow";
import { RouterTester } from "./features/router/RouterTester";
import { AddRuleModal } from "./features/rules/AddRuleModal";
import { RulesTab } from "./features/rules/RulesTab";
import { useRegexErrors } from "./features/rules/useRegexErrors";
import { SettingsTab } from "./features/settings/SettingsTab";
import { useSystemIntegration } from "./features/settings/useSystemIntegration";
import {
  applySettingsDraft,
  cloneSet,
  createBrowserId,
  createRuleId,
  EMPTY_STATUS,
  rejectAfter,
  ROUTE_OPEN_TIMEOUT_MS,
} from "./lib/appTypes";
import { useConfigStore } from "./hooks/useConfigStore";
import { useDocumentTheme } from "./hooks/useDocumentTheme";
import { useSidebarShortcut } from "./hooks/useSidebarShortcut";
import type {
  AppConfig,
  BrowserDraft,
  BrowserConfig,
  ManualBrowserValidationResult,
  FormModal,
  RouteDecision,
  RuleDraft,
  RuleConfig,
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
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isResettingConfig, setIsResettingConfig] = useState(false);
  const [isStartingOnboarding, setIsStartingOnboarding] = useState(false);
  const [configPath, setConfigPath] = useState<string | null>(null);
  const [onboardingStep, setOnboardingStep] = useState(0);
  const [onboardingStartWithWindows, setOnboardingStartWithWindows] =
    useState(true);
  const [isFinishingOnboarding, setIsFinishingOnboarding] = useState(false);
  const [isCheckingOnboardingDefaults, setIsCheckingOnboardingDefaults] =
    useState(false);
  const [dirtyRuleIds, setDirtyRuleIds] = useState<Set<string>>(new Set());
  const [savingRuleIds, setSavingRuleIds] = useState<Set<string>>(new Set());
  const [pendingBrowserIds, setPendingBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [savingBrowserIds, setSavingBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [failedBrowserIds, setFailedBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [settingsActionPanel, setSettingsActionPanel] =
    useState<SettingsActionPanel>("none");
  const [formModal, setFormModal] = useState<FormModal>(null);

  const [browserDraft, setBrowserDraft] = useState<BrowserDraft>({
    name: "",
    path: "",
    privateFlag: "",
  });
  const [manualBrowserConfirmation, setManualBrowserConfirmation] =
    useState<ManualBrowserConfirmation | null>(null);
  const [feedbackModal, setFeedbackModal] = useState<FeedbackMessage | null>(
    null,
  );
  const [deleteConfirmation, setDeleteConfirmation] =
    useState<DeleteConfirmation | null>(null);
  const [isConfirmingManualBrowser, setIsConfirmingManualBrowser] =
    useState(false);

  const [ruleDraft, setRuleDraft] = useState<RuleDraft>({
    pattern: "",
    patternType: "hostname",
    browserId: "",
    privateMode: false,
  });

  const [routeInput, setRouteInput] = useState(
    "https://github.com/openai/codex",
  );
  const [routeDecision, setRouteDecision] = useState<RouteDecision | null>(
    null,
  );
  const [isRouting, setIsRouting] = useState(false);
  const browserSaveTimersRef = useRef<Record<string, number>>({});
  const isClosingRef = useRef(false);
  const contentAreaRef = useRef<HTMLDivElement | null>(null);

  const {
    config,
    configRef,
    persistedConfigRef,
    theme,
    settingsDraft,
    isSaving,
    setCurrentConfig,
    applyConfigChange,
    applyLoadedConfig,
    persistConfig,
    scheduleSettingsSave,
    cancelScheduledSettingsSave,
    waitForQueuedSaves,
    updateSettingsDraft,
    updateThemePreference,
  } = useConfigStore({ setStatus });
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
  const regexErrors = useRegexErrors(config);

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

  useEffect(() => {
    if (!visibleBrowsers.length) {
      return;
    }

    setRuleDraft((current) => {
      if (
        !current.browserId ||
        !visibleBrowsers.some((browser) => browser.id === current.browserId)
      ) {
        return { ...current, browserId: visibleBrowsers[0].id };
      }
      return current;
    });
  }, [visibleBrowsers]);

  useEffect(() => {
    return () => {
      for (const timerId of Object.values(browserSaveTimersRef.current)) {
        window.clearTimeout(timerId);
      }
    };
  }, []);

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

  function normalizeBrowserPath(path: string) {
    return path.trim().replace(/\//g, "\\").toLowerCase();
  }

  function findBrowserPathConflict(browser: BrowserConfig) {
    const currentConfig = configRef.current;
    if (!currentConfig) {
      return null;
    }

    const normalizedPath = normalizeBrowserPath(browser.path);
    if (!normalizedPath) {
      return null;
    }

    return (
      currentConfig.browsers.find(
        (candidate) =>
          candidate.id !== browser.id &&
          normalizeBrowserPath(candidate.path) === normalizedPath,
      ) ?? null
    );
  }

  function showFeedbackModal(options: FeedbackMessage) {
    setFeedbackModal(options);
  }

  function showBrowserValidationAlert(message: string) {
    setStatus({
      kind: "error",
      text: message,
    });
    showFeedbackModal({
      title: "Invalid browser configuration",
      message,
      kind: "error",
    });
  }

  function revertBrowserToPersisted(browserId: string) {
    const persistedConfig = persistedConfigRef.current;
    const currentConfig = configRef.current;
    if (!persistedConfig || !currentConfig) {
      return;
    }

    const persistedBrowser = persistedConfig.browsers.find(
      (browser) => browser.id === browserId,
    );
    if (!persistedBrowser) {
      return;
    }

    const nextConfig: AppConfig = {
      ...currentConfig,
      browsers: currentConfig.browsers.map((browser) =>
        browser.id === browserId ? persistedBrowser : browser,
      ),
    };
    setCurrentConfig(nextConfig);
  }

  function applyManualBrowserValidation(
    browser: BrowserConfig,
    validation: ManualBrowserValidationResult,
  ): BrowserConfig {
    return {
      ...browser,
      name: browser.name.trim() || validation.browserName,
      privateFlag: validation.privateFlag,
      iconKey: validation.iconKey ?? browser.iconKey ?? "generic",
      manualTrust: validation.manualTrust,
    };
  }

  async function validateManualBrowserConfig(
    browser: BrowserConfig,
    allowUserConfirmed = false,
    options?: {
      showAlert?: boolean;
      revertBrowserId?: string;
      allowConfirmationFlow?: boolean;
    },
  ): Promise<{
    browser: BrowserConfig;
    validation: ManualBrowserValidationResult;
  } | null> {
    const conflict = findBrowserPathConflict(browser);
    if (conflict) {
      const message = `A browser with this executable path already exists: "${conflict.name}".`;
      if (options?.revertBrowserId) {
        revertBrowserToPersisted(options.revertBrowserId);
      }
      if (options?.showAlert) {
        showBrowserValidationAlert(message);
      } else {
        setStatus({
          kind: "error",
          text: message,
        });
      }
      return null;
    }

    let validation: ManualBrowserValidationResult;
    try {
      validation = await validateManualBrowser({
        name: browser.name,
        path: browser.path,
        privateFlag: browser.privateFlag,
        allowUserConfirmed,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (options?.revertBrowserId) {
        revertBrowserToPersisted(options.revertBrowserId);
      }
      if (options?.showAlert) {
        showBrowserValidationAlert(message);
      } else {
        setStatus({
          kind: "error",
          text: message,
        });
      }
      return null;
    }

    if (validation.requiresConfirmation) {
      if (!options?.allowConfirmationFlow) {
        const message = validation.message;
        if (options?.revertBrowserId) {
          revertBrowserToPersisted(options.revertBrowserId);
        }
        if (options?.showAlert) {
          showBrowserValidationAlert(message);
        } else {
          setStatus({
            kind: "error",
            text: message,
          });
        }
        return null;
      }

      return {
        browser,
        validation,
      };
    }

    return {
      browser: applyManualBrowserValidation(browser, validation),
      validation,
    };
  }

  // Validates a browser edit and saves it. Shared by the debounced save and
  // the immediate flush (e.g. when an input loses focus).
  async function saveBrowserChanges(browserId: string, baseConfig: AppConfig) {
    try {
      let configToSave = baseConfig;
      const browser = configToSave.browsers.find(
        (item) => item.id === browserId,
      );
      if (browser) {
        const prepared = await validateManualBrowserConfig(browser, false, {
          showAlert: true,
          revertBrowserId: browserId,
          allowConfirmationFlow: browser.source === "manual",
        });
        if (!prepared) {
          return;
        }

        if (
          browser.source === "manual" &&
          prepared.validation.requiresConfirmation
        ) {
          setManualBrowserConfirmation({
            mode: "edit",
            browserId,
            browser,
            validation: prepared.validation,
          });
          setStatus({
            kind: "warning",
            text: prepared.validation.message,
          });
          return;
        }

        configToSave = {
          ...configToSave,
          browsers: configToSave.browsers.map((item) =>
            item.id === browserId ? prepared.browser : item,
          ),
        };
        setCurrentConfig(configToSave);
      }

      await persistConfig(configToSave, {
        errorPrefix: "Could not save browser changes",
        onSuccess: () => {
          setFailedBrowserIds((current) => {
            if (!current.has(browserId)) {
              return current;
            }
            const next = cloneSet(current);
            next.delete(browserId);
            return next;
          });
        },
        onError: () => {
          setFailedBrowserIds((current) => {
            const next = cloneSet(current);
            next.add(browserId);
            return next;
          });
        },
      });
    } finally {
      setSavingBrowserIds((current) => {
        const next = cloneSet(current);
        next.delete(browserId);
        return next;
      });
    }
  }

  function scheduleBrowserSave(browserId: string, nextConfig: AppConfig) {
    const existingTimer = browserSaveTimersRef.current[browserId];
    if (existingTimer) {
      window.clearTimeout(existingTimer);
    }

    setPendingBrowserIds((current) => {
      const next = cloneSet(current);
      next.add(browserId);
      return next;
    });
    setFailedBrowserIds((current) => {
      if (!current.has(browserId)) {
        return current;
      }
      const next = cloneSet(current);
      next.delete(browserId);
      return next;
    });

    browserSaveTimersRef.current[browserId] = window.setTimeout(() => {
      delete browserSaveTimersRef.current[browserId];
      setPendingBrowserIds((current) => {
        const next = cloneSet(current);
        next.delete(browserId);
        return next;
      });
      setSavingBrowserIds((current) => {
        const next = cloneSet(current);
        next.add(browserId);
        return next;
      });

      void saveBrowserChanges(browserId, nextConfig);
    }, 500);
  }

  function flushBrowserSave(browserId: string) {
    const existingTimer = browserSaveTimersRef.current[browserId];
    const latestConfig = configRef.current;
    if (!existingTimer || !latestConfig) {
      return;
    }

    window.clearTimeout(existingTimer);
    delete browserSaveTimersRef.current[browserId];
    setPendingBrowserIds((current) => {
      const next = cloneSet(current);
      next.delete(browserId);
      return next;
    });
    setSavingBrowserIds((current) => {
      const next = cloneSet(current);
      next.add(browserId);
      return next;
    });

    void saveBrowserChanges(browserId, latestConfig);
  }

  async function closeSettingsWindow() {
    if (isClosingRef.current) {
      return;
    }

    isClosingRef.current = true;
    const hasPendingSettingsSave = cancelScheduledSettingsSave();
    const hasPendingBrowserSaves =
      Object.keys(browserSaveTimersRef.current).length > 0;

    for (const timerId of Object.values(browserSaveTimersRef.current)) {
      window.clearTimeout(timerId);
    }
    browserSaveTimersRef.current = {};

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

  async function handleRefreshBrowsers() {
    setIsRefreshing(true);
    try {
      const refreshed = await refreshBrowsers();
      applyLoadedConfig(refreshed);
      setStatus({ kind: "success", text: "Browser detection refreshed." });
      await refreshRunningState({ force: true });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({ kind: "error", text: `Refresh failed: ${message}` });
    } finally {
      setIsRefreshing(false);
    }
  }

  function updateBrowser(browserId: string, patch: Partial<BrowserConfig>) {
    const nextConfig = applyConfigChange((current) => ({
      ...current,
      browsers: current.browsers.map((browser) => {
        if (browser.id !== browserId) {
          return browser;
        }

        const nextBrowser = { ...browser, ...patch };
        if (browser.source === "manual" && patch.path !== undefined) {
          nextBrowser.manualTrust = null;
        }
        return nextBrowser;
      }),
    }));

    if (!nextConfig) {
      return;
    }

    scheduleBrowserSave(browserId, nextConfig);
  }

  function toggleBrowserHidden(browserId: string, isHidden: boolean) {
    const nextConfig = applyConfigChange((current) => ({
      ...current,
      defaultBrowserId:
        isHidden && current.defaultBrowserId === browserId
          ? null
          : current.defaultBrowserId,
      browsers: current.browsers.map((browser) =>
        browser.id === browserId ? { ...browser, isHidden } : browser,
      ),
    }));

    if (!nextConfig) {
      return;
    }

    void persistConfig(nextConfig, {
      successText: isHidden
        ? "Browser hidden from picker."
        : "Browser restored to picker.",
      errorPrefix: "Could not update browser visibility",
    });
  }

  function deleteManualBrowser(browserId: string) {
    const currentConfig = configRef.current;
    const browser = currentConfig?.browsers.find(
      (item) => item.id === browserId,
    );
    if (!currentConfig || !browser || browser.source !== "manual") {
      return;
    }

    const existingTimer = browserSaveTimersRef.current[browserId];
    if (existingTimer) {
      window.clearTimeout(existingTimer);
      delete browserSaveTimersRef.current[browserId];
    }

    const nextConfig = applyConfigChange((current) => ({
      ...current,
      defaultBrowserId:
        current.defaultBrowserId === browserId
          ? null
          : current.defaultBrowserId,
      browsers: current.browsers.filter((item) => item.id !== browserId),
      rules: current.rules.filter((rule) => rule.browserId !== browserId),
    }));

    if (!nextConfig) {
      return;
    }

    setPendingBrowserIds((current) => {
      if (!current.has(browserId)) {
        return current;
      }
      const next = cloneSet(current);
      next.delete(browserId);
      return next;
    });
    setSavingBrowserIds((current) => {
      if (!current.has(browserId)) {
        return current;
      }
      const next = cloneSet(current);
      next.delete(browserId);
      return next;
    });
    setFailedBrowserIds((current) => {
      if (!current.has(browserId)) {
        return current;
      }
      const next = cloneSet(current);
      next.delete(browserId);
      return next;
    });

    void persistConfig(nextConfig, {
      successText: `Deleted manual browser "${browser.name}".`,
      errorPrefix: "Could not delete manual browser",
    });
  }

  function requestDeleteManualBrowser(browserId: string) {
    const currentConfig = configRef.current;
    const browser = currentConfig?.browsers.find(
      (item) => item.id === browserId,
    );
    if (!currentConfig || !browser || browser.source !== "manual") {
      return;
    }

    setDeleteConfirmation({
      kind: "manual-browser",
      browserId,
      browserName: browser.name,
      affectedRuleCount: currentConfig.rules.filter(
        (rule) => rule.browserId === browserId,
      ).length,
    });
  }

  async function addManualBrowser() {
    const currentConfig = configRef.current;
    const name = browserDraft.name.trim();
    const path = browserDraft.path.trim();

    if (!currentConfig || !name || !path) {
      setStatus({
        kind: "error",
        text: "Manual browser needs both a name and an executable path.",
      });
      return;
    }

    const browser: BrowserConfig = {
      id: createBrowserId(),
      name,
      path,
      privateFlag: browserDraft.privateFlag.trim() || null,
      iconKey: null,
      manualTrust: null,
      source: "manual",
      isHidden: false,
    };
    const prepared = await validateManualBrowserConfig(browser, false, {
      showAlert: true,
      allowConfirmationFlow: true,
    });
    if (!prepared) {
      return;
    }

    if (prepared.validation.requiresConfirmation) {
      setManualBrowserConfirmation({
        mode: "add",
        browserId: null,
        browser,
        validation: prepared.validation,
      });
      setStatus({
        kind: "warning",
        text: prepared.validation.message,
      });
      return;
    }

    setBrowserDraft({ name: "", path: "", privateFlag: "" });
    setFormModal(null);
    const nextConfig: AppConfig = {
      ...currentConfig,
      browsers: [...currentConfig.browsers, prepared.browser],
    };
    await persistConfig(nextConfig, {
      successText: `Added manual browser "${prepared.browser.name}".`,
      errorPrefix: "Could not add manual browser",
    });
  }

  async function confirmManualBrowser() {
    const confirmation = manualBrowserConfirmation;
    const currentConfig = configRef.current;
    if (!confirmation || !currentConfig || isConfirmingManualBrowser) {
      return;
    }

    setIsConfirmingManualBrowser(true);
    setManualBrowserConfirmation(null);
    if (confirmation.mode === "add") {
      setFormModal(null);
      setBrowserDraft({ name: "", path: "", privateFlag: "" });
    }

    try {
      if (confirmation.mode === "add") {
        const prepared = await validateManualBrowserConfig(
          confirmation.browser,
          true,
          {
            showAlert: true,
          },
        );
        if (!prepared) {
          return;
        }

        const latestConfig = configRef.current;
        if (!latestConfig) {
          return;
        }

        const nextConfig: AppConfig = {
          ...latestConfig,
          browsers: [...latestConfig.browsers, prepared.browser],
        };
        await persistConfig(nextConfig, {
          successText: `Added manual browser "${prepared.browser.name}".`,
          errorPrefix: "Could not add manual browser",
        });
        return;
      }

      const browserId = confirmation.browserId;
      if (!browserId) {
        return;
      }

      const currentBrowser = currentConfig.browsers.find(
        (browser) => browser.id === browserId,
      );
      if (!currentBrowser) {
        return;
      }

      const prepared = await validateManualBrowserConfig(currentBrowser, true, {
        showAlert: true,
        revertBrowserId: browserId,
      });
      if (!prepared) {
        return;
      }

      const nextConfig: AppConfig = {
        ...currentConfig,
        browsers: currentConfig.browsers.map((browser) =>
          browser.id === browserId ? prepared.browser : browser,
        ),
      };
      setCurrentConfig(nextConfig);
      await persistConfig(nextConfig, {
        successText: `Updated manual browser "${prepared.browser.name}".`,
        errorPrefix: "Could not save browser changes",
      });
    } finally {
      setIsConfirmingManualBrowser(false);
    }
  }

  function updateRule(ruleId: string, patch: Partial<RuleConfig>) {
    const nextConfig = applyConfigChange((current) => ({
      ...current,
      rules: current.rules.map((rule) =>
        rule.id === ruleId ? { ...rule, ...patch } : rule,
      ),
    }));
    if (!nextConfig) {
      return;
    }

    setDirtyRuleIds((current) => {
      const next = cloneSet(current);
      next.add(ruleId);
      return next;
    });
  }

  function saveRule(ruleId: string) {
    const currentConfig = configRef.current;
    if (!currentConfig) {
      return;
    }

    const rule = currentConfig.rules.find((item) => item.id === ruleId);
    if (!rule) {
      return;
    }

    if (!rule.pattern.trim()) {
      setStatus({ kind: "error", text: "Rule pattern cannot be empty." });
      return;
    }

    if (!rule.browserId) {
      setStatus({ kind: "error", text: "Select a browser for this rule." });
      return;
    }

    if (rule.patternType === "regex" && regexErrors[rule.id]) {
      setStatus({
        kind: "error",
        text: "Fix this rule's regex before saving.",
      });
      return;
    }

    setSavingRuleIds((current) => {
      const next = cloneSet(current);
      next.add(ruleId);
      return next;
    });

    void persistConfig(currentConfig, {
      successText: "Rule saved.",
      errorPrefix: "Could not save rule",
      onSuccess: () => {
        setDirtyRuleIds((current) => {
          const next = cloneSet(current);
          next.delete(ruleId);
          return next;
        });
        setSavingRuleIds((current) => {
          const next = cloneSet(current);
          next.delete(ruleId);
          return next;
        });
      },
      onError: () => {
        setSavingRuleIds((current) => {
          const next = cloneSet(current);
          next.delete(ruleId);
          return next;
        });
      },
    });
  }

  function deleteRule(ruleId: string) {
    const nextConfig = applyConfigChange((current) => ({
      ...current,
      rules: current.rules.filter((rule) => rule.id !== ruleId),
    }));
    if (!nextConfig) {
      return;
    }

    setDirtyRuleIds((current) => {
      if (!current.has(ruleId)) {
        return current;
      }
      const next = cloneSet(current);
      next.delete(ruleId);
      return next;
    });

    void persistConfig(nextConfig, {
      successText: "Rule deleted.",
      errorPrefix: "Could not delete rule",
    });
  }

  function requestDeleteRule(ruleId: string) {
    const currentConfig = configRef.current;
    const ruleIndex = currentConfig?.rules.findIndex(
      (item) => item.id === ruleId,
    );
    if (!currentConfig || ruleIndex === undefined || ruleIndex < 0) {
      return;
    }

    const rule = currentConfig.rules[ruleIndex];
    setDeleteConfirmation({
      kind: "rule",
      ruleId,
      ruleLabel: `Rule ${ruleIndex + 1}`,
      pattern: rule.pattern,
    });
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

  function moveRule(ruleId: string, direction: -1 | 1) {
    let didMove = false;
    const nextConfig = applyConfigChange((current) => {
      const index = current.rules.findIndex((rule) => rule.id === ruleId);
      if (index < 0) {
        return current;
      }

      const destination = index + direction;
      if (destination < 0 || destination >= current.rules.length) {
        return current;
      }

      const nextRules = [...current.rules];
      [nextRules[index], nextRules[destination]] = [
        nextRules[destination],
        nextRules[index],
      ];
      didMove = true;

      return { ...current, rules: nextRules };
    });
    if (!nextConfig || !didMove) {
      return;
    }

    void persistConfig(nextConfig, {
      successText: "Rule order updated.",
      errorPrefix: "Could not reorder rules",
    });
  }

  async function addRule() {
    if (!config) {
      return;
    }

    const pattern = ruleDraft.pattern.trim();
    if (!pattern) {
      setStatus({ kind: "error", text: "Rule pattern cannot be empty." });
      return;
    }

    if (!ruleDraft.browserId) {
      setStatus({ kind: "error", text: "Select a browser for this rule." });
      return;
    }

    if (ruleDraft.patternType === "regex") {
      try {
        const [regexError] = await validateRegexPatterns([pattern]);
        if (regexError) {
          setStatus({ kind: "error", text: `Regex is invalid: ${regexError}` });
          return;
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setStatus({ kind: "error", text: `Could not validate regex: ${message}` });
        return;
      }
    }

    const rule: RuleConfig = {
      id: createRuleId(),
      pattern,
      patternType: ruleDraft.patternType,
      browserId: ruleDraft.browserId,
      privateMode: ruleDraft.privateMode,
      enabled: true,
    };

    const nextConfig: AppConfig = {
      ...config,
      rules: [...config.rules, rule],
    };

    setCurrentConfig(nextConfig);
    try {
      await persistConfig(nextConfig, {
        successText: "Rule added and saved.",
        errorPrefix: "Could not save new rule",
      });
      setRuleDraft((current) => ({ ...current, pattern: "" }));
      setFormModal(null);
    } catch {
      setCurrentConfig(config);
    }
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
      setDirtyRuleIds(new Set());
      setSavingRuleIds(new Set());
      setPendingBrowserIds(new Set());
      setSavingBrowserIds(new Set());
      setFailedBrowserIds(new Set());
      setRouteDecision(null);
      setBrowserDraft({ name: "", path: "", privateFlag: "" });
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
