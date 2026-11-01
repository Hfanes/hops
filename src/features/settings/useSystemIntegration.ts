import { useState } from "react";
import {
  getBrowserRegistrationStatus,
  getStartWithWindowsEnabled,
  openWindowsDefaultApps,
  registerHopsAsBrowser,
  setStartWithWindowsEnabled as saveStartWithWindowsEnabled,
  unregisterHopsAsBrowser,
} from "../../services/tauri";
import type { BrowserRegistrationStatus, StatusState } from "../../types";

/**
 * Windows integration state shared by Settings and onboarding: Hops' browser
 * registration (Default Apps) and the Start with Windows toggle.
 */
export function useSystemIntegration({
  setStatus,
}: {
  setStatus: (status: StatusState) => void;
}) {
  const [registrationStatus, setRegistrationStatus] =
    useState<BrowserRegistrationStatus | null>(null);
  const [isRegistering, setIsRegistering] = useState(false);
  const [startWithWindowsEnabled, setStartWithWindowsEnabled] = useState<
    boolean | null
  >(null);
  const [isUpdatingStartWithWindows, setIsUpdatingStartWithWindows] =
    useState(false);

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

  return {
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
  };
}
