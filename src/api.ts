import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  BrowserRegistrationStatus,
  OpenUrlRequest,
  RouteDecision,
} from "./types";

export function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("load_config");
}

export function saveConfig(config: AppConfig): Promise<AppConfig> {
  return invoke<AppConfig>("save_config", { config });
}

export function refreshBrowsers(): Promise<AppConfig> {
  return invoke<AppConfig>("refresh_browsers");
}

export function listRunningBrowserIds(): Promise<string[]> {
  return invoke<string[]>("list_running_browser_ids");
}

export function previewRoute(url: string): Promise<RouteDecision> {
  return invoke<RouteDecision>("preview_route", { url });
}

export function previewRouteWithConfig(config: AppConfig, url: string): Promise<RouteDecision> {
  return invoke<RouteDecision>("preview_route_with_config", { config, url });
}

export function routeAndOpen(url: string): Promise<RouteDecision> {
  return invoke<RouteDecision>("route_and_open", { url });
}

export function routeAndOpenWithConfig(config: AppConfig, url: string): Promise<RouteDecision> {
  return invoke<RouteDecision>("route_and_open_with_config", { config, url });
}

export function openUrl(request: OpenUrlRequest): Promise<void> {
  return invoke<void>("open_url", { request });
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
