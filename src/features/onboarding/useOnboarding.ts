import { useEffect, useState } from "react";
import { errorMessage } from "../../lib/errors";
import {
  saveConfig,
  setStartWithWindowsEnabled as saveStartWithWindowsEnabled,
  showSettingsWindow,
} from "../../services/tauri";
import type {
  AppConfig,
  BrowserRegistrationStatus,
  StatusState,
} from "../../types";

/**
 * The four-step onboarding shown until the config is marked complete: the
 * current step, the Start with Windows choice, and the checks and saves that
 * move to the last step and finish.
 */
export function useOnboarding({
  config,
  applyLoadedConfig,
  setStatus,
  setStartWithWindowsEnabled,
  refreshRegistrationStatus,
  onFinished,
}: {
  config: AppConfig | null;
  applyLoadedConfig: (config: AppConfig) => void;
  setStatus: (status: StatusState) => void;
  setStartWithWindowsEnabled: (enabled: boolean | null) => void;
  refreshRegistrationStatus: () => Promise<BrowserRegistrationStatus | null>;
  onFinished: () => void;
}) {
  const [step, setStep] = useState(0);
  const [startWithWindows, setStartWithWindows] = useState(true);
  const [isFinishing, setIsFinishing] = useState(false);
  const [isCheckingDefaults, setIsCheckingDefaults] = useState(false);

  useEffect(() => {
    if (!config || config.onboardingCompleted) {
      return;
    }

    window.scrollTo({ top: 0, left: 0, behavior: "auto" });
  }, [config, step]);

  async function finish() {
    if (!config) {
      return;
    }

    const nextConfig: AppConfig = {
      ...config,
      onboardingCompleted: true,
    };

    setIsFinishing(true);
    try {
      const enabled = await saveStartWithWindowsEnabled(startWithWindows);
      setStartWithWindowsEnabled(enabled);

      const saved = await saveConfig(nextConfig);
      applyLoadedConfig(saved);
      onFinished();
      await showSettingsWindow();
      setStatus({
        kind: "success",
        text: startWithWindows
          ? "Onboarding completed. Hops will start with Windows. Review your settings before closing this window."
          : "Onboarding completed. Review your settings before closing this window.",
      });
    } catch (error) {
      const message = errorMessage(error);
      setStatus({
        kind: "error",
        text: `Could not finish onboarding: ${message}`,
      });
    } finally {
      setIsFinishing(false);
    }
  }

  async function continueToFinish() {
    setIsCheckingDefaults(true);
    try {
      const next = await refreshRegistrationStatus();

      if (!next) {
        setStatus({
          kind: "warning",
          text: "Could not check Windows HTTP/HTTPS defaults. You can refresh status or finish setup later in Settings.",
        });
      }

      setStep(3);
    } finally {
      setIsCheckingDefaults(false);
    }
  }

  return {
    step,
    setStep,
    startWithWindows,
    setStartWithWindows,
    isFinishing,
    isCheckingDefaults,
    finish,
    continueToFinish,
  };
}

export type Onboarding = ReturnType<typeof useOnboarding>;
