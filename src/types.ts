export type BrowserSource = "detected" | "manual";

export type RulePatternType =
  | "hostname"
  | "hostname_subdomains"
  | "prefix"
  | "contains"
  | "full_url"
  | "glob"
  | "regex";

export type RouteAction = "open_browser" | "show_picker";
export type ThemePreference = "light" | "dark";

export interface BrowserConfig {
  id: string;
  name: string;
  path: string;
  privateFlag: string | null;
  source: BrowserSource;
  isHidden: boolean;
}

export interface RuleConfig {
  id: string;
  pattern: string;
  patternType: RulePatternType;
  browserId: string;
  privateMode: boolean;
  enabled: boolean;
}

export interface AppConfig {
  version: number;
  alwaysShowPicker: boolean;
  useDefaultsWhenNotRunning: boolean;
  disableTransparency: boolean;
  themePreference: ThemePreference;
  onboardingCompleted: boolean;
  defaultBrowserId: string | null;
  browsers: BrowserConfig[];
  rules: RuleConfig[];
}

export interface RouteDecision {
  action: RouteAction;
  reason: string;
  browserId: string | null;
  browserName: string | null;
  privateMode: boolean;
  matchedRuleId: string | null;
}

export interface OpenUrlRequest {
  browserId: string;
  url: string;
  privateMode: boolean;
}

export interface BrowserRegistrationStatus {
  registered: boolean;
  isDefaultHttp: boolean;
  isDefaultHttps: boolean;
  isFullyDefault: boolean;
  currentHttpProgId: string | null;
  currentHttpsProgId: string | null;
}

export type PickerLaunchSource = "route" | "manual";

export interface PickerBrowserEntry {
  id: string;
  name: string;
  privateFlag: string | null;
  isDefault: boolean;
  isRunning: boolean;
}

export interface PickerSession {
  url: string;
  reason: string;
  source: PickerLaunchSource;
  preferredBrowserId: string | null;
  preferredPrivateMode: boolean;
  disableTransparency: boolean;
  themePreference: ThemePreference;
  alwaysShowPicker: boolean;
  altPressed: boolean;
  browsers: PickerBrowserEntry[];
}
