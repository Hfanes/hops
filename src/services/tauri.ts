import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type {
  AppConfig,
  BrowserRegistrationStatus,
  ManualBrowserValidationRequest,
  ManualBrowserValidationResult,
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

export function getConfigFilePath(): Promise<string> {
  return invoke<string>("get_config_file_path");
}

export function openConfigFolder(): Promise<void> {
  return invoke<void>("open_config_folder");
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

/**
 * Validates regex patterns with the backend's Rust regex engine, which is what
 * routing uses. Returns one entry per pattern: null when valid, else the error.
 */
export function validateRegexPatterns(
  patterns: string[],
): Promise<Array<string | null>> {
  return invoke<Array<string | null>>("validate_regex_patterns", { patterns });
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

const UPDATE_CHECK_CACHE_KEY = "hops.updateCheck";
const NOTIFIED_UPDATE_VERSION_KEY = "hops.notifiedUpdateVersion";
const UPDATE_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

interface CachedUpdateCheck {
  checkedAt: number;
  status: AppUpdateStatus;
}

// localStorage can be unavailable or throw (cleared or blocked site data), so
// every access is guarded and a failure simply means "check the network".
function readStorage(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Best effort only.
  }
}

function readCachedUpdateCheck(currentVersion: string): AppUpdateStatus | null {
  const raw = readStorage(UPDATE_CHECK_CACHE_KEY);
  if (!raw) {
    return null;
  }

  try {
    const cached = JSON.parse(raw) as Partial<CachedUpdateCheck>;
    const age = Date.now() - Number(cached.checkedAt);
    const isFresh = age >= 0 && age < UPDATE_CHECK_INTERVAL_MS;
    // A different current version means Hops was updated since the check.
    if (!isFresh || cached.status?.currentVersion !== currentVersion) {
      return null;
    }
    return cached.status;
  } catch {
    return null;
  }
}

/**
 * Checks GitHub for a newer Hops release.
 *
 * Automatic checks reuse a result from the last 24 hours so opening settings
 * does not hit the network every time. Pass `force` for user-initiated checks.
 */
export async function checkForAppUpdate(
  options: { force?: boolean } = {},
): Promise<AppUpdateStatus> {
  const currentVersion = await getVersion();

  if (!options.force) {
    const cached = readCachedUpdateCheck(currentVersion);
    if (cached) {
      return cached;
    }
  }

  const update = await check();
  const status: AppUpdateStatus = {
    currentVersion,
    available: Boolean(update),
    version: update?.version ?? null,
    date: update?.date ?? null,
    body: update?.body ?? null,
  };

  const cached: CachedUpdateCheck = { checkedAt: Date.now(), status };
  writeStorage(UPDATE_CHECK_CACHE_KEY, JSON.stringify(cached));
  return status;
}

/** Whether the "update available" notification was already shown for `version`. */
export function wasUpdateNotified(version: string): boolean {
  return readStorage(NOTIFIED_UPDATE_VERSION_KEY) === version;
}

export function markUpdateNotified(version: string) {
  writeStorage(NOTIFIED_UPDATE_VERSION_KEY, version);
}

export async function notifyAppUpdateAvailable(
  updateStatus: AppUpdateStatus,
): Promise<void> {
  if (!updateStatus.available) {
    return;
  }

  let permissionGranted = await isPermissionGranted();
  if (!permissionGranted) {
    permissionGranted = (await requestPermission()) === "granted";
  }

  if (permissionGranted) {
    sendNotification({
      title: "Hops update available",
      body: updateStatus.version
        ? `Hops ${updateStatus.version} is ready to install.`
        : "A new Hops version is ready to install.",
    });
  }
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
