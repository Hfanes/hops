import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type {
  AppConfig,
  BrowserRegistrationStatus,
  ManualBrowserValidationRequest,
  ManualBrowserValidationResult,
  OpenUrlRequest,
  PickerSession,
  RouteDecision,
} from "../types";

export interface AppAboutInfo {
  version: string;
  releaseDate: string;
}

export interface AppUpdateStatus {
  currentVersion: string;
  available: boolean;
  version: string | null;
  date: string | null;
  body: string | null;
}

const RELEASE_DATE =
  import.meta.env.VITE_HOPS_RELEASE_DATE?.trim() || "Development build";

export function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("load_config");
}

export function saveConfig(config: AppConfig): Promise<AppConfig> {
  return invoke<AppConfig>("save_config", { config });
}

export function validateManualBrowser(
  request: ManualBrowserValidationRequest,
): Promise<ManualBrowserValidationResult> {
  return invoke<ManualBrowserValidationResult>("validate_manual_browser", {
    request,
  });
}

export function refreshBrowsers(): Promise<AppConfig> {
  return invoke<AppConfig>("refresh_browsers");
}

export function resetConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("reset_config");
}

export function listRunningBrowserIds(): Promise<string[]> {
  return invoke<string[]>("list_running_browser_ids");
}

export function previewRoute(url: string): Promise<RouteDecision> {
  return invoke<RouteDecision>("preview_route", { url });
}

export function previewRouteWithConfig(
  config: AppConfig,
  url: string,
): Promise<RouteDecision> {
  return invoke<RouteDecision>("preview_route_with_config", { config, url });
}

export function routeAndOpen(url: string): Promise<RouteDecision> {
  return invoke<RouteDecision>("route_and_open", { url });
}

export function routeAndOpenWithConfig(
  config: AppConfig,
  url: string,
): Promise<RouteDecision> {
  return invoke<RouteDecision>("route_and_open_with_config", { config, url });
}

export function openUrl(request: OpenUrlRequest): Promise<void> {
  return invoke<void>("open_url", { request });
}

export function getPickerState(): Promise<PickerSession | null> {
  return invoke<PickerSession | null>("get_picker_state");
}

export function showPickerForUrl(
  url: string,
  preferredBrowserId: string | null = null,
  preferredPrivateMode = false,
): Promise<void> {
  return invoke<void>("show_picker_for_url", {
    url,
    preferredBrowserId,
    preferredPrivateMode,
  });
}

export function hidePickerWindow(): Promise<void> {
  return invoke<void>("hide_picker_window");
}

export function showSettingsWindow(): Promise<void> {
  return invoke<void>("show_settings_window_command");
}

export function openWindowsDefaultApps(): Promise<void> {
  return invoke<void>("open_windows_default_apps");
}

export function getBrowserRegistrationStatus(): Promise<BrowserRegistrationStatus> {
  return invoke<BrowserRegistrationStatus>("get_browser_registration_status");
}

export function registerHopsAsBrowser(): Promise<BrowserRegistrationStatus> {
  return invoke<BrowserRegistrationStatus>("register_hops_as_browser");
}

export function unregisterHopsAsBrowser(): Promise<BrowserRegistrationStatus> {
  return invoke<BrowserRegistrationStatus>("unregister_hops_as_browser");
}

export function getStartWithWindowsEnabled(): Promise<boolean> {
  return isAutostartEnabled();
}

export async function setStartWithWindowsEnabled(
  enabled: boolean,
): Promise<boolean> {
  if (enabled) {
    await enableAutostart();
  } else {
    if (!(await isAutostartEnabled())) {
      return false;
    }
    await disableAutostart();
  }

  return isAutostartEnabled();
}

export async function getAppAboutInfo(): Promise<AppAboutInfo> {
  return {
    version: await getVersion(),
    releaseDate: RELEASE_DATE,
  };
}

export async function checkForAppUpdate(): Promise<AppUpdateStatus> {
  const [currentVersion, update] = await Promise.all([getVersion(), check()]);

  return {
    currentVersion,
    available: Boolean(update),
    version: update?.version ?? null,
    date: update?.date ?? null,
    body: update?.body ?? null,
  };
}

export async function installAvailableUpdate(): Promise<boolean> {
  const update = await check();

  if (!update) {
    return false;
  }

  await update.downloadAndInstall();
  await relaunch();
  return true;
}
