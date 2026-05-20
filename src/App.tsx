import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { FiPlus } from "react-icons/fi";
import {
  checkForAppUpdate,
  getBrowserRegistrationStatus,
  getAppAboutInfo,
  getStartWithWindowsEnabled,
  installAvailableUpdate,
  listRunningBrowserIds,
  loadConfig,
  openWindowsDefaultApps,
  previewRouteWithConfig,
  registerHopsAsBrowser,
  refreshBrowsers,
  resetConfig,
  routeAndOpenWithConfig,
  saveConfig,
  setStartWithWindowsEnabled as saveStartWithWindowsEnabled,
  showPickerForUrl,
  unregisterHopsAsBrowser,
  validateManualBrowser,
} from "./services/tauri";
import type { AppAboutInfo, AppUpdateStatus } from "./services/tauri";
import { LoadingState } from "./components/common/LoadingState";
import { ModalShell } from "./components/common/ModalShell";
import { PatternTypePicker } from "./components/common/PatternTypePicker";
import { StatusBanner } from "./components/common/StatusBanner";
import { ScrollTopButton } from "./components/layout/ScrollTopButton";
import { SidebarNav } from "./components/layout/SidebarNav";
import { Topbar } from "./components/layout/Topbar";
import { AboutPanel } from "./features/about/AboutPanel";
import { BrowsersTab } from "./features/browsers/BrowsersTab";
import { OnboardingFlow } from "./features/onboarding/OnboardingFlow";
import { RouterTester } from "./features/router/RouterTester";
import { RulesTab } from "./features/rules/RulesTab";
import { SettingsTab } from "./features/settings/SettingsTab";
import {
  applySettingsDraft,
  cloneSet,
  createBrowserId,
  createRuleId,
  EMPTY_STATUS,
  regexErrorsByRule,
  rejectAfter,
  ROUTE_OPEN_TIMEOUT_MS,
  settingsDraftFromConfig,
} from "./lib/appTypes";
import { useDocumentTheme } from "./hooks/useDocumentTheme";
import { useSidebarShortcut } from "./hooks/useSidebarShortcut";
import type {
  AppConfig,
  BrowserDraft,
  BrowserConfig,
  ManualBrowserValidationResult,
  BrowserRegistrationStatus,
  FormModal,
  RouteDecision,
  RuleDraft,
  RuleConfig,
  SettingsActionPanel,
  SettingsDraft,
  StatusState,
  TabId,
  ThemePreference,
} from "./types";
import "./styles/App.css";

function App() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [activeTab, setActiveTab] = useState<TabId>("settings");
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [theme, setTheme] = useState<ThemePreference>("light");
  const [status, setStatus] = useState<StatusState>(EMPTY_STATUS);
  const [isLoading, setIsLoading] = useState(true);
  const [saveActivityCount, setSaveActivityCount] = useState(0);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isRegistering, setIsRegistering] = useState(false);
  const [isResettingConfig, setIsResettingConfig] = useState(false);
  const [isStartingOnboarding, setIsStartingOnboarding] = useState(false);
  const [runningBrowserIds, setRunningBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [registrationStatus, setRegistrationStatus] =
    useState<BrowserRegistrationStatus | null>(null);
  const [startWithWindowsEnabled, setStartWithWindowsEnabled] = useState<
    boolean | null
  >(null);
  const [isUpdatingStartWithWindows, setIsUpdatingStartWithWindows] =
    useState(false);
  const [aboutInfo, setAboutInfo] = useState<AppAboutInfo | null>(null);
  const [updateStatus, setUpdateStatus] = useState<AppUpdateStatus | null>(
    null,
  );
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [isInstallingUpdate, setIsInstallingUpdate] = useState(false);
  const [onboardingStep, setOnboardingStep] = useState(0);
  const [onboardingStartWithWindows, setOnboardingStartWithWindows] =
    useState(true);
  const [isFinishingOnboarding, setIsFinishingOnboarding] = useState(false);
  const [isCheckingOnboardingDefaults, setIsCheckingOnboardingDefaults] =
    useState(false);
  const [settingsDraft, setSettingsDraft] = useState<SettingsDraft | null>(
    null,
  );
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
  const [manualBrowserConfirmation, setManualBrowserConfirmation] = useState<{
    mode: "add" | "edit";
    browserId: string | null;
    browser: BrowserConfig;
    validation: ManualBrowserValidationResult;
  } | null>(null);
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
  const isSaving = saveActivityCount > 0;
  const configRef = useRef<AppConfig | null>(null);
  const persistedConfigRef = useRef<AppConfig | null>(null);
  const settingsDraftRef = useRef<SettingsDraft | null>(null);
  const saveQueueRef = useRef<Promise<void>>(Promise.resolve());
  const browserSaveTimersRef = useRef<Record<string, number>>({});
  const settingsSaveTimerRef = useRef<number | null>(null);
  const contentAreaRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  useEffect(() => {
    settingsDraftRef.current = settingsDraft;
  }, [settingsDraft]);

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

        const runningIds = await listRunningBrowserIds();
        setRunningBrowserIds(new Set(runningIds));
        await refreshRegistrationStatus();
        await refreshStartWithWindowsStatus();
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

  const regexErrors = useMemo(
    () => (config ? regexErrorsByRule(config.rules) : {}),
    [config],
  );

  useEffect(() => {
    return () => {
      for (const timerId of Object.values(browserSaveTimersRef.current)) {
        window.clearTimeout(timerId);
      }
      if (settingsSaveTimerRef.current !== null) {
        window.clearTimeout(settingsSaveTimerRef.current);
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
    if (activeTab !== "about") {
      return;
    }

    void refreshAbout(false);
  }, [activeTab]);

  function applyConfigChange(transform: (current: AppConfig) => AppConfig) {
    let nextConfig: AppConfig | null = null;

    setConfig((current) => {
      if (!current) {
        return current;
      }

      nextConfig = transform(current);
      configRef.current = nextConfig;
      return nextConfig;
    });

    return nextConfig;
  }

  function applyLoadedConfig(nextConfig: AppConfig) {
    const nextSettingsDraft = settingsDraftFromConfig(nextConfig);

    configRef.current = nextConfig;
    persistedConfigRef.current = nextConfig;
    settingsDraftRef.current = nextSettingsDraft;
    setConfig(nextConfig);
    setTheme(nextConfig.themePreference);
    setSettingsDraft(nextSettingsDraft);
  }

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

  function showBrowserValidationAlert(message: string) {
    setStatus({
      kind: "error",
      text: message,
    });
    window.alert(message);
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
    configRef.current = nextConfig;
    setConfig(nextConfig);
  }

  function applyManualBrowserValidation(
    browser: BrowserConfig,
    validation: ManualBrowserValidationResult,
  ): BrowserConfig {
    return {
      ...browser,
      name: browser.name.trim() || validation.browserName,
      privateFlag: validation.privateFlag,
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

  function updateThemePreference(nextTheme: ThemePreference) {
    setTheme(nextTheme);
    setSettingsDraft((current) => {
      const nextDraft = current
        ? {
            ...current,
            themePreference: nextTheme,
          }
        : current;
      settingsDraftRef.current = nextDraft;
      return nextDraft;
    });

    const currentConfig = configRef.current;
    if (!currentConfig || currentConfig.themePreference === nextTheme) {
      return;
    }

    const nextConfig: AppConfig = {
      ...currentConfig,
      themePreference: nextTheme,
    };

    configRef.current = nextConfig;
    setConfig(nextConfig);
    void persistConfig(nextConfig, {
      errorPrefix: "Could not save theme preference",
      applySavedConfig: false,
    });
  }

  function persistConfig(
    nextConfig: AppConfig,
    options?: {
      successText?: string;
      errorPrefix?: string;
      onSuccess?: (saved: AppConfig) => void;
      onError?: () => void;
      onSettled?: () => void;
      applySavedConfig?: boolean;
    },
  ) {
    setSaveActivityCount((count) => count + 1);

    const runSave = async () => {
      try {
        const saved = await saveConfig(nextConfig);
        if (options?.applySavedConfig ?? true) {
          applyLoadedConfig(saved);
        }
        options?.onSuccess?.(saved);
        if (options?.successText) {
          setStatus({ kind: "success", text: options.successText });
        }
      } catch (error) {
        options?.onError?.();
        const message = error instanceof Error ? error.message : String(error);
        setStatus({
          kind: "error",
          text: `${options?.errorPrefix ?? "Save failed"}: ${message}`,
        });
        throw error;
      } finally {
        setSaveActivityCount((count) => Math.max(0, count - 1));
        options?.onSettled?.();
      }
    };

    const queued = saveQueueRef.current.then(runSave, runSave);
    saveQueueRef.current = queued.catch(() => undefined);
    return queued;
  }

  function scheduleSettingsSave(nextConfig: AppConfig) {
    if (settingsSaveTimerRef.current !== null) {
      window.clearTimeout(settingsSaveTimerRef.current);
    }

    settingsSaveTimerRef.current = window.setTimeout(() => {
      settingsSaveTimerRef.current = null;
      void persistConfig(nextConfig, {
        errorPrefix: "Could not save settings",
        applySavedConfig: false,
      });
    }, 350);
  }

  function updateSettingsDraft(
    transform: (current: SettingsDraft) => SettingsDraft,
  ) {
    const currentConfig = configRef.current;
    if (!currentConfig) {
      return;
    }

    const currentDraft =
      settingsDraftRef.current ?? settingsDraftFromConfig(currentConfig);
    const nextDraft = transform(currentDraft);
    const nextConfig = applySettingsDraft(currentConfig, nextDraft);

    configRef.current = nextConfig;
    settingsDraftRef.current = nextDraft;
    setConfig(nextConfig);
    setSettingsDraft(nextDraft);
    scheduleSettingsSave(nextConfig);
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

      void (async () => {
        try {
          let configToSave = nextConfig;
          const browser = configToSave.browsers.find((item) => item.id === browserId);
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
            configRef.current = configToSave;
            setConfig(configToSave);
          }

          await persistConfig(configToSave, {
            errorPrefix: "Could not save browser changes",
            onSuccess: () => {
              setSavingBrowserIds((current) => {
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
            },
            onError: () => {
              setSavingBrowserIds((current) => {
                const next = cloneSet(current);
                next.delete(browserId);
                return next;
              });
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
      })();
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

    void (async () => {
      try {
        let configToSave = latestConfig;
        const browser = configToSave.browsers.find((item) => item.id === browserId);
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
          configRef.current = configToSave;
          setConfig(configToSave);
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
    })();
  }

  async function refreshRegistrationStatus() {
    try {
      const next = await getBrowserRegistrationStatus();
      setRegistrationStatus(next);
      return next;
    } catch {
      setRegistrationStatus(null);
      return null;
    }
  }

  async function refreshStartWithWindowsStatus() {
    try {
      const enabled = await getStartWithWindowsEnabled();
      setStartWithWindowsEnabled(enabled);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStartWithWindowsEnabled(null);
      setStatus({
        kind: "warning",
        text: `Could not read Start with Windows status: ${message}`,
      });
    }
  }

  async function updateStartWithWindows(checked: boolean) {
    const previous = startWithWindowsEnabled;
    setStartWithWindowsEnabled(checked);
    setIsUpdatingStartWithWindows(true);

    try {
      const enabled = await saveStartWithWindowsEnabled(checked);
      setStartWithWindowsEnabled(enabled);
      setStatus({
        kind: "success",
        text: enabled
          ? "Hops will start when you sign in."
          : "Hops will not start when you sign in.",
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStartWithWindowsEnabled(previous);
      setStatus({
        kind: "error",
        text: `Could not update Start with Windows: ${message}`,
      });
    } finally {
      setIsUpdatingStartWithWindows(false);
    }
  }

  async function refreshRunningState() {
    try {
      const runningIds = await listRunningBrowserIds();
      setRunningBrowserIds(new Set(runningIds));
    } catch {
      // Running-state refresh should be non-blocking.
    }
  }

  async function handleRefreshBrowsers() {
    setIsRefreshing(true);
    try {
      const refreshed = await refreshBrowsers();
      applyLoadedConfig(refreshed);
      setStatus({ kind: "success", text: "Browser detection refreshed." });
      await refreshRunningState();
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
    const browser = currentConfig?.browsers.find((item) => item.id === browserId);
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
        current.defaultBrowserId === browserId ? null : current.defaultBrowserId,
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
      manualTrust: null,
      source: "manual",
      isHidden: false,
    };
    const prepared = await validateManualBrowserConfig(browser, false, {
      showAlert: true,
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
      configRef.current = nextConfig;
      setConfig(nextConfig);
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
        new RegExp(pattern);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setStatus({ kind: "error", text: `Regex is invalid: ${message}` });
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

    configRef.current = nextConfig;
    setConfig(nextConfig);
    try {
      await persistConfig(nextConfig, {
        successText: "Rule added and saved.",
        errorPrefix: "Could not save new rule",
      });
      setRuleDraft((current) => ({ ...current, pattern: "" }));
      setFormModal(null);
    } catch {
      setConfig(config);
      configRef.current = config;
    }
  }

  async function openDefaultAppsSettings() {
    try {
      await openWindowsDefaultApps();
      setStatus({
        kind: "success",
        text: "Opened Windows Default Apps settings.",
      });
      await refreshRegistrationStatus();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not open Windows settings: ${message}`,
      });
    }
  }

  async function registerBrowserIntegration() {
    setIsRegistering(true);
    try {
      const next = await registerHopsAsBrowser();
      setRegistrationStatus(next);
      setStatus({
        kind: "success",
        text: "Hops was registered in Windows Default Apps. Now pick Hops for HTTP and HTTPS in Windows settings.",
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({ kind: "error", text: `Could not register Hops: ${message}` });
    } finally {
      setIsRegistering(false);
    }
  }

  async function unregisterBrowserIntegration() {
    setIsRegistering(true);
    try {
      const next = await unregisterHopsAsBrowser();
      setRegistrationStatus(next);
      setStatus({
        kind: "success",
        text: "Hops registration keys were removed from your user profile.",
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not unregister Hops: ${message}`,
      });
    } finally {
      setIsRegistering(false);
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
      setStatus({
        kind: "success",
        text: onboardingStartWithWindows
          ? "Onboarding completed. Hops will start with Windows and stay in tray."
          : "Onboarding completed. Hops will stay in tray when opened.",
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
      await refreshRunningState();
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
      await refreshRunningState();
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
      await refreshRunningState();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({ kind: "error", text: `Routing failed: ${message}` });
      if (openImmediately && error instanceof Error && error.name === "TimeoutError") {
        window.alert(
          `Hops could not open the route within 10 seconds.\n\n${message}`,
        );
      }
    } finally {
      setIsRouting(false);
    }
  }

  async function refreshAbout(showResultStatus = true) {
    setIsCheckingUpdate(true);
    try {
      const [nextAboutInfo, nextUpdateStatus] = await Promise.all([
        getAppAboutInfo(),
        checkForAppUpdate(),
      ]);
      setAboutInfo(nextAboutInfo);
      setUpdateStatus(nextUpdateStatus);

      if (showResultStatus) {
        setStatus({
          kind: nextUpdateStatus.available ? "warning" : "success",
          text: nextUpdateStatus.available
            ? `Hops ${nextUpdateStatus.version} is available.`
            : "Hops is up to date.",
        });
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not check for updates: ${message}`,
      });
    } finally {
      setIsCheckingUpdate(false);
    }
  }

  async function updateApp() {
    setIsInstallingUpdate(true);
    try {
      const installed = await installAvailableUpdate();

      if (!installed) {
        setStatus({ kind: "success", text: "Hops is already up to date." });
        await refreshAbout(false);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not install update: ${message}`,
      });
    } finally {
      setIsInstallingUpdate(false);
    }
  }

  function scrollCurrentPageToTop() {
    contentAreaRef.current?.scrollTo({ top: 0, behavior: "smooth" });
    window.scrollTo({ top: 0, left: 0, behavior: "smooth" });
  }

  const shellClassName = "min-h-screen bg-[var(--h-bg)] p-0";
  const panelClassName = `grid min-h-screen bg-[var(--h-bg)] ${
    isSidebarCollapsed
      ? "grid-cols-[54px_minmax(0,1fr)]"
      : "grid-cols-[168px_minmax(0,1fr)]"
  }`;
  const onboardingPanelClassName =
    "grid min-h-screen grid-cols-1 bg-[var(--h-bg)]";
  const sidebarClassName =
    "sticky top-0 flex h-screen min-h-0 flex-col gap-3 overflow-hidden border-r border-[var(--h-border)] bg-[#075056] p-2.5 text-[#FDF6E3]";
  const contentClassName = "content-area min-h-screen min-w-0 p-4 md:p-6";
  const topbarClassName =
    "topbar mb-3.5 flex flex-wrap items-start justify-between gap-4 border-b border-[var(--h-border)] pb-3.5";

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

  const statusBanner = (
    <StatusBanner
      status={status}
      onDismiss={() => setStatus(EMPTY_STATUS)}
    />
  );

  const activeFormModal =
    formModal === "manual-browser" ? (
      <ModalShell
        title={<h3 id="manual-browser-title">Add manual browser</h3>}
        titleId="manual-browser-title"
        onClose={() => setFormModal(null)}
      >
        <label>
          Name
          <input
            value={browserDraft.name}
            onChange={(event) => {
              const value = event.currentTarget.value;
              setBrowserDraft((current) => ({
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
              setBrowserDraft((current) => ({
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
              setBrowserDraft((current) => ({
                ...current,
                privateFlag: value,
              }));
            }}
            placeholder="--incognito"
          />
        </label>
        <div className="inline-actions">
          <button
            type="button"
            onClick={() => void addManualBrowser()}
            disabled={isSaving}
          >
            Add browser
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() => setFormModal(null)}
          >
            Cancel
          </button>
        </div>
      </ModalShell>
    ) : formModal === "rule" ? (
      <ModalShell
        title={
          <div>
            <h3 id="rule-modal-title">Add rule</h3>
            <p className="setting-help">New rules are created as enabled.</p>
          </div>
        }
        titleId="rule-modal-title"
        onClose={() => setFormModal(null)}
      >
          <label>
            Pattern
            <input
              value={ruleDraft.pattern}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setRuleDraft((current) => ({
                  ...current,
                  pattern: value,
                }));
              }}
              placeholder="*.notion.so"
            />
          </label>
          <PatternTypePicker
            name="new-rule-pattern-type"
            value={ruleDraft.patternType}
            onChange={(value) =>
              setRuleDraft((current) => ({
                ...current,
                patternType: value,
              }))
            }
          />
          <label>
            Browser
            <select
              value={ruleDraft.browserId}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setRuleDraft((current) => ({
                  ...current,
                  browserId: value,
                }));
              }}
            >
              <option value="">Choose browser</option>
              {visibleBrowsers.map((browser) => (
                <option key={browser.id} value={browser.id}>
                  {browser.name}
                </option>
              ))}
            </select>
          </label>
          <div className="dual-toggle">
            <label className="toggle">
              <input
                type="checkbox"
                checked={ruleDraft.privateMode}
                onChange={(event) => {
                  const checked = event.currentTarget.checked;
                  setRuleDraft((current) => ({
                    ...current,
                    privateMode: checked,
                  }));
                }}
              />
              <span>Private mode</span>
            </label>
          </div>
          <div className="inline-actions">
            <button
              type="button"
              onClick={() => void addRule()}
              disabled={isSaving || !visibleBrowsers.length}
            >
              Add rule
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => setFormModal(null)}
            >
              Cancel
            </button>
          </div>
      </ModalShell>
    ) : null;

  const manualBrowserConfirmationModal = manualBrowserConfirmation ? (
    <ModalShell
      title={<h3 id="manual-browser-confirm-title">Confirm manual browser</h3>}
      titleId="manual-browser-confirm-title"
      onClose={() => setManualBrowserConfirmation(null)}
    >
      <p className="setting-help">{manualBrowserConfirmation.validation.message}</p>
      <label>
        Executable path
        <input value={manualBrowserConfirmation.browser.path} readOnly />
      </label>
      <label>
        Name
        <input value={manualBrowserConfirmation.browser.name} readOnly />
      </label>
      <div className="inline-actions">
        <button
          type="button"
          onClick={() => void confirmManualBrowser()}
          disabled={isConfirmingManualBrowser}
        >
          {isConfirmingManualBrowser ? "Confirming..." : "Confirm and trust"}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={isConfirmingManualBrowser}
          onClick={() => setManualBrowserConfirmation(null)}
        >
          Cancel
        </button>
      </div>
    </ModalShell>
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
              onDeleteManualBrowser={deleteManualBrowser}
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
              onDeleteRule={deleteRule}
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
    </main>
  );
}

export default App;
