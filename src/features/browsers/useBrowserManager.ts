import { useEffect, useRef, useState } from "react";
import type { DeleteConfirmation } from "../../components/common/DeleteConfirmModal";
import type { FeedbackMessage } from "../../components/common/FeedbackModal";
import type { ConfigStore } from "../../hooks/useConfigStore";
import { cloneSet, createBrowserId } from "../../lib/appTypes";
import { refreshBrowsers, validateManualBrowser } from "../../services/tauri";
import type {
  AppConfig,
  BrowserConfig,
  BrowserDraft,
  FormModal,
  ManualBrowserValidationResult,
  StatusState,
} from "../../types";
import type { ManualBrowserConfirmation } from "./ManualBrowserConfirmModal";

const EMPTY_BROWSER_DRAFT: BrowserDraft = { name: "", path: "", privateFlag: "" };

/**
 * Everything the Browsers tab (and onboarding) does to browsers: inline edits
 * with debounced, validated saves; refreshing detection; hiding; and adding,
 * confirming, and deleting manual browsers.
 *
 * Modals shared with other features stay with the caller, which passes in
 * callbacks to open them.
 */
export function useBrowserManager({
  store,
  setStatus,
  setFormModal,
  showFeedbackModal,
  onRequestDelete,
  refreshRunningState,
}: {
  store: ConfigStore;
  setStatus: (status: StatusState) => void;
  setFormModal: (modal: FormModal) => void;
  showFeedbackModal: (feedback: FeedbackMessage) => void;
  onRequestDelete: (confirmation: DeleteConfirmation) => void;
  refreshRunningState: (options?: { force?: boolean }) => Promise<void>;
}) {
  const {
    configRef,
    persistedConfigRef,
    setCurrentConfig,
    applyConfigChange,
    applyLoadedConfig,
    persistConfig,
  } = store;

  const [isRefreshing, setIsRefreshing] = useState(false);
  const [pendingBrowserIds, setPendingBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [savingBrowserIds, setSavingBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [failedBrowserIds, setFailedBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [browserDraft, setBrowserDraft] =
    useState<BrowserDraft>(EMPTY_BROWSER_DRAFT);
  const [manualBrowserConfirmation, setManualBrowserConfirmation] =
    useState<ManualBrowserConfirmation | null>(null);
  const [isConfirmingManualBrowser, setIsConfirmingManualBrowser] =
    useState(false);
  const browserSaveTimersRef = useRef<Record<string, number>>({});

  useEffect(() => {
    return () => {
      for (const timerId of Object.values(browserSaveTimersRef.current)) {
        window.clearTimeout(timerId);
      }
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

    onRequestDelete({
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

  /** Cancels all debounced browser saves. Returns whether any were pending. */
  function cancelPendingBrowserSaves() {
    const timers = Object.values(browserSaveTimersRef.current);
    for (const timerId of timers) {
      window.clearTimeout(timerId);
    }
    browserSaveTimersRef.current = {};
    return timers.length > 0;
  }

  /** Clears per-browser save indicators and the add form, e.g. after a reset. */
  function resetBrowserEditState() {
    setPendingBrowserIds(new Set());
    setSavingBrowserIds(new Set());
    setFailedBrowserIds(new Set());
    setBrowserDraft(EMPTY_BROWSER_DRAFT);
  }

  return {
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
  };
}
