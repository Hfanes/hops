import type {
  AppConfig,
  RuleConfig,
  SettingsDraft,
} from "../types";

export const EMPTY_STATUS = { kind: "idle", text: "" } as const;
export const ROUTE_OPEN_TIMEOUT_MS = 10_000;

export function regexErrorsByRule(rules: RuleConfig[]): Record<string, string> {
  const errors: Record<string, string> = {};

  for (const rule of rules) {
    if (rule.patternType !== "regex") {
      continue;
    }

    try {
      new RegExp(rule.pattern);
    } catch (error) {
      errors[rule.id] =
        error instanceof Error
          ? error.message
          : "Invalid regular expression pattern.";
    }
  }

  return errors;
}

export function createRuleId() {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `rule-${crypto.randomUUID()}`;
  }
  return `rule-${Date.now()}`;
}

export function createBrowserId() {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `manual-${crypto.randomUUID()}`;
  }
  return `manual-${Date.now()}`;
}

export function cloneSet<T>(values: Set<T>) {
  return new Set(values);
}

export function settingsDraftFromConfig(config: AppConfig): SettingsDraft {
  return {
    alwaysShowPicker: config.alwaysShowPicker,
    useDefaultsWhenNotRunning: config.useDefaultsWhenNotRunning,
    disableTransparency: config.disableTransparency,
    themePreference: config.themePreference,
    defaultBrowserId: config.defaultBrowserId ?? "",
  };
}

export function applySettingsDraft(
  config: AppConfig,
  settingsDraft: SettingsDraft | null,
): AppConfig {
  if (!settingsDraft) {
    return config;
  }

  return {
    ...config,
    alwaysShowPicker: settingsDraft.alwaysShowPicker,
    useDefaultsWhenNotRunning: settingsDraft.useDefaultsWhenNotRunning,
    disableTransparency: settingsDraft.disableTransparency,
    themePreference: settingsDraft.themePreference,
    defaultBrowserId: settingsDraft.defaultBrowserId || null,
  };
}

export function rejectAfter<T>(
  promise: Promise<T>,
  timeoutMs: number,
  message: string,
): Promise<T> {
  let timeoutId: number | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timeoutId = window.setTimeout(() => {
      const error = new Error(message);
      error.name = "TimeoutError";
      reject(error);
    }, timeoutMs);
  });

  return Promise.race([promise, timeout]).finally(() => {
    if (timeoutId !== undefined) {
      window.clearTimeout(timeoutId);
    }
  });
}
