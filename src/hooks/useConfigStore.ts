import { useEffect, useRef, useState } from "react";
import { saveConfig } from "../services/tauri";
import {
  applySettingsDraft,
  settingsDraftFromConfig,
} from "../lib/appTypes";
import type {
  AppConfig,
  SettingsDraft,
  StatusState,
  ThemePreference,
} from "../types";

const SETTINGS_SAVE_DEBOUNCE_MS = 350;

export interface PersistConfigOptions {
  successText?: string;
  errorPrefix?: string;
  onSuccess?: (saved: AppConfig) => void;
  onError?: () => void;
  onSettled?: () => void;
  /** Replace local state with the backend's normalized result (default true). */
  applySavedConfig?: boolean;
}

/**
 * The settings window's copy of the config and everything that saves it.
 *
 * `configRef` always holds the latest config, including changes made earlier
 * in the same event, while `config` drives rendering. Saves run one at a time
 * through a queue so they reach the backend in order. Settings-panel edits are
 * debounced; `cancelScheduledSettingsSave` and `waitForQueuedSaves` let the
 * window flush everything before it closes.
 */
export function useConfigStore({
  setStatus,
}: {
  setStatus: (status: StatusState) => void;
}) {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [theme, setTheme] = useState<ThemePreference>("light");
  const [settingsDraft, setSettingsDraft] = useState<SettingsDraft | null>(
    null,
  );
  const [saveActivityCount, setSaveActivityCount] = useState(0);
  const isSaving = saveActivityCount > 0;

  const configRef = useRef<AppConfig | null>(null);
  const persistedConfigRef = useRef<AppConfig | null>(null);
  const settingsDraftRef = useRef<SettingsDraft | null>(null);
  const saveQueueRef = useRef<Promise<void>>(Promise.resolve());
  const settingsSaveTimerRef = useRef<number | null>(null);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  useEffect(() => {
    settingsDraftRef.current = settingsDraft;
  }, [settingsDraft]);

  useEffect(() => {
    return () => {
      if (settingsSaveTimerRef.current !== null) {
        window.clearTimeout(settingsSaveTimerRef.current);
      }
    };
  }, []);

  /** Updates the working config (ref and render state) without saving. */
  function setCurrentConfig(nextConfig: AppConfig) {
    configRef.current = nextConfig;
    setConfig(nextConfig);
  }

  function applyConfigChange(transform: (current: AppConfig) => AppConfig) {
    const currentConfig = configRef.current;
    if (!currentConfig) {
      return null;
    }

    const nextConfig = transform(currentConfig);
    setCurrentConfig(nextConfig);
    return nextConfig;
  }

  /** Adopts a config that the backend loaded or saved as the new baseline. */
  function applyLoadedConfig(nextConfig: AppConfig) {
    const nextSettingsDraft = settingsDraftFromConfig(nextConfig);

    configRef.current = nextConfig;
    persistedConfigRef.current = nextConfig;
    settingsDraftRef.current = nextSettingsDraft;
    setConfig(nextConfig);
    setTheme(nextConfig.themePreference);
    setSettingsDraft(nextSettingsDraft);
  }

  function persistConfig(nextConfig: AppConfig, options?: PersistConfigOptions) {
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
    }, SETTINGS_SAVE_DEBOUNCE_MS);
  }

  /** Cancels a debounced settings save. Returns whether one was pending. */
  function cancelScheduledSettingsSave() {
    if (settingsSaveTimerRef.current === null) {
      return false;
    }

    window.clearTimeout(settingsSaveTimerRef.current);
    settingsSaveTimerRef.current = null;
    return true;
  }

  /** Resolves once every save queued so far has finished. */
  function waitForQueuedSaves() {
    return saveQueueRef.current;
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

    setCurrentConfig(nextConfig);
    void persistConfig(nextConfig, {
      errorPrefix: "Could not save theme preference",
      applySavedConfig: false,
    });
  }

  return {
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
  };
}

export type ConfigStore = ReturnType<typeof useConfigStore>;
