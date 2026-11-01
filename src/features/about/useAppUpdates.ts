import { useEffect, useState } from "react";
import {
  checkForAppUpdate,
  getAppAboutInfo,
  installAvailableUpdate,
  markUpdateNotified,
  notifyAppUpdateAvailable,
  wasUpdateNotified,
} from "../../services/tauri";
import type { AppAboutInfo, AppUpdateStatus } from "../../services/tauri";
import type { StatusState, TabId } from "../../types";

/**
 * About-panel info and update checks: a silent check after startup (with a
 * one-time notification per version), a refresh when the About tab opens, and
 * the explicit check and install actions.
 */
export function useAppUpdates({
  activeTab,
  isLoading,
  setStatus,
}: {
  activeTab: TabId;
  isLoading: boolean;
  setStatus: (status: StatusState) => void;
}) {
  const [aboutInfo, setAboutInfo] = useState<AppAboutInfo | null>(null);
  const [updateStatus, setUpdateStatus] = useState<AppUpdateStatus | null>(
    null,
  );
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [isInstallingUpdate, setIsInstallingUpdate] = useState(false);

  // A user-visible check (the button) always hits the network; silent
  // refreshes (opening the About tab) reuse the daily cached result.
  async function refreshAbout(
    showResultStatus = true,
    force = showResultStatus,
  ) {
    setIsCheckingUpdate(true);
    try {
      const [nextAboutInfo, nextUpdateStatus] = await Promise.all([
        getAppAboutInfo(),
        checkForAppUpdate({ force }),
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

  async function refreshStatusBarUpdate() {
    setIsCheckingUpdate(true);
    try {
      const nextUpdateStatus = await checkForAppUpdate();
      setUpdateStatus(nextUpdateStatus);

      // Remembered across settings windows, which are destroyed on close, so
      // the same update is announced only once.
      const updateVersionKey = nextUpdateStatus.version ?? "unknown";
      if (nextUpdateStatus.available && !wasUpdateNotified(updateVersionKey)) {
        markUpdateNotified(updateVersionKey);
        try {
          await notifyAppUpdateAvailable(nextUpdateStatus);
        } catch (error) {
          console.warn("Could not show update notification.", error);
        }
      }
    } catch (error) {
      console.warn("Could not check for updates.", error);
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
        await refreshAbout(false, true);
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

  useEffect(() => {
    if (activeTab !== "about") {
      return;
    }

    void refreshAbout(false);
  }, [activeTab]);

  useEffect(() => {
    if (isLoading) {
      return;
    }

    const idleId = window.requestIdleCallback(
      () => void refreshStatusBarUpdate(),
      { timeout: 5000 },
    );
    return () => window.cancelIdleCallback(idleId);
  }, [isLoading]);

  return {
    aboutInfo,
    setAboutInfo,
    updateStatus,
    isCheckingUpdate,
    isInstallingUpdate,
    refreshAbout,
    updateApp,
  };
}
