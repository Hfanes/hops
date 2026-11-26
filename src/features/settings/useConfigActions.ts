import { useState } from "react";
import type { ConfigStore } from "../../hooks/useConfigStore";
import { applySettingsDraft } from "../../lib/appTypes";
import { errorMessage } from "../../lib/errors";
import {
  openConfigFolder as openConfigFolderInExplorer,
  resetConfig as resetConfigOnDisk,
  saveConfig,
} from "../../services/tauri";
import type { SettingsActionPanel, StatusState } from "../../types";

/**
 * The config-wide actions in Settings: reset the configuration, restart
 * onboarding (with or without a reset first) and open the config folder,
 * plus which confirmation panel is open.
 *
 * Other features keep state that a reset or restart makes stale; the caller
 * clears it in `onConfigReset` and `onOnboardingRestarted`.
 */
export function useConfigActions({
  store,
  setStatus,
  cancelPendingBrowserSaves,
  refreshRunningState,
  onConfigReset,
  onOnboardingRestarted,
}: {
  store: ConfigStore;
  setStatus: (status: StatusState) => void;
  cancelPendingBrowserSaves: () => boolean;
  refreshRunningState: (options?: { force?: boolean }) => Promise<void>;
  onConfigReset: () => void;
  onOnboardingRestarted: () => void;
}) {
  const {
    config,
    settingsDraft,
    applyLoadedConfig,
    enqueueConfigTask,
    cancelScheduledSettingsSave,
  } = store;
  const [settingsActionPanel, setSettingsActionPanel] =
    useState<SettingsActionPanel>("none");
  const [isResettingConfig, setIsResettingConfig] = useState(false);
  const [isStartingOnboarding, setIsStartingOnboarding] = useState(false);

  async function resetConfig() {
    setIsResettingConfig(true);
    try {
      // Pending edits are discarded by the reset, so drop them instead of
      // saving them, and queue the reset after saves already in flight.
      cancelScheduledSettingsSave();
      cancelPendingBrowserSaves();
      const reset = await enqueueConfigTask(() => resetConfigOnDisk());
      setSettingsActionPanel("none");
      applyLoadedConfig(reset);
      onConfigReset();
      setStatus({
        kind: "success",
        text: "Configuration reset. Rules and manual browsers were cleared, and detected browsers were restored.",
      });
      await refreshRunningState({ force: true });
    } catch (error) {
      const message = errorMessage(error);
      setStatus({
        kind: "error",
        text: `Could not reset configuration: ${message}`,
      });
    } finally {
      setIsResettingConfig(false);
    }
  }

  async function rerunOnboarding(shouldResetFirst: boolean) {
    if (!config) {
      return;
    }

    setIsStartingOnboarding(true);
    try {
      const nextConfig = shouldResetFirst
        ? await saveConfig({
            ...(await resetConfigOnDisk()),
            onboardingCompleted: false,
          })
        : await saveConfig({
            ...applySettingsDraft(config, settingsDraft),
            onboardingCompleted: false,
          });

      onOnboardingRestarted();
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
      const message = errorMessage(error);
      setStatus({
        kind: "error",
        text: `Could not restart onboarding: ${message}`,
      });
    } finally {
      setIsStartingOnboarding(false);
    }
  }

  async function openConfigFolder() {
    try {
      await openConfigFolderInExplorer();
      setStatus({
        kind: "success",
        text: "Opened config folder.",
      });
    } catch (error) {
      const message = errorMessage(error);
      setStatus({
        kind: "error",
        text: `Could not open config folder: ${message}`,
      });
    }
  }

  return {
    settingsActionPanel,
    setSettingsActionPanel,
    isResettingConfig,
    isStartingOnboarding,
    resetConfig,
    rerunOnboarding,
    openConfigFolder,
  };
}

export type ConfigActions = ReturnType<typeof useConfigActions>;
