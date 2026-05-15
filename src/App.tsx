import { useEffect, useMemo, useRef, useState } from "react";
import type { IconType } from "react-icons";
import {
  FiArrowUp,
  FiChevronLeft,
  FiChevronRight,
  FiGlobe,
  FiInfo,
  FiList,
  FiMoon,
  FiNavigation,
  FiPlus,
  FiSettings,
  FiSun,
  FiX,
} from "react-icons/fi";
import {
  getBrowserRegistrationStatus,
  listRunningBrowserIds,
  loadConfig,
  openWindowsDefaultApps,
  previewRouteWithConfig,
  registerHopsAsBrowser,
  refreshBrowsers,
  resetConfig,
  routeAndOpenWithConfig,
  saveConfig,
  showPickerForUrl,
  unregisterHopsAsBrowser,
} from "./api";
import type {
  AppConfig,
  BrowserConfig,
  BrowserRegistrationStatus,
  RouteDecision,
  RuleConfig,
  RulePatternType,
  ThemePreference,
} from "./types";
import "./App.css";

type TabId = "settings" | "browsers" | "rules" | "router";

interface RuleDraft {
  pattern: string;
  patternType: RulePatternType;
  browserId: string;
  privateMode: boolean;
}

interface BrowserDraft {
  name: string;
  path: string;
  privateFlag: string;
}

interface SettingsDraft {
  alwaysShowPicker: boolean;
  useDefaultsWhenNotRunning: boolean;
  disableTransparency: boolean;
  themePreference: ThemePreference;
  defaultBrowserId: string;
}

interface StatusState {
  kind: "idle" | "success" | "error" | "warning";
  text: string;
}

type SettingsActionPanel = "none" | "reset" | "rerun-onboarding";
type FormModal = "manual-browser" | "rule" | null;

const EMPTY_STATUS: StatusState = { kind: "idle", text: "" };

const NAV_ITEMS: Array<{ id: TabId; label: string; icon: IconType }> = [
  { id: "settings", label: "Settings", icon: FiSettings },
  { id: "browsers", label: "Browsers", icon: FiGlobe },
  { id: "rules", label: "Rules", icon: FiList },
  { id: "router", label: "Route tester", icon: FiNavigation },
];

const TAB_TITLES: Record<TabId, { title: string; subtitle: string }> = {
  settings: {
    title: "Settings",
    subtitle: "Routing defaults, picker behavior, and Windows registration.",
  },
  browsers: {
    title: "Browsers",
    subtitle: "Detected and manual browser entries used by Hops.",
  },
  rules: {
    title: "Rules",
    subtitle: "URL match rules evaluated from top to bottom.",
  },
  router: {
    title: "Route Tester",
    subtitle: "Preview routing decisions before opening a URL.",
  },
};

const PATTERN_OPTIONS: Array<{ value: RulePatternType; label: string }> = [
  { value: "hostname", label: "Hostname" },
  { value: "hostname_subdomains", label: "Hostname + subdomains" },
  { value: "prefix", label: "Prefix" },
  { value: "contains", label: "Contains" },
  { value: "full_url", label: "Full URL" },
  { value: "glob", label: "Glob" },
  { value: "regex", label: "Regex" },
];

const PATTERN_HELP: Record<
  RulePatternType,
  { title: string; description: string; examples: string[] }
> = {
  hostname: {
    title: "Hostname",
    description:
      "Matches only the domain. Ignores protocol, path, and query string. Best default choice. If you paste a full URL here, it will usually not match.",
    examples: [
      "Pattern: github.com -> matches https://github.com/org/repo",
      "Pattern: github.com -> does not match https://api.github.com",
    ],
  },
  hostname_subdomains: {
    title: "Hostname + subdomains",
    description:
      "Use *.<domain> to match subdomains only. It does not match the root domain itself.",
    examples: [
      "Pattern: *.notion.so -> matches https://workspace.notion.so/page",
    ],
  },
  prefix: {
    title: "Prefix",
    description:
      "Matches when the URL starts exactly with your pattern. Great for locking one path branch.",
    examples: [
      "Pattern: https://linear.app/myteam -> matches https://linear.app/myteam/issue/ENG-1",
      "Pattern: https://linear.app/myteam -> does not match https://linear.app/otherteam",
    ],
  },
  contains: {
    title: "Contains",
    description:
      "Case-insensitive substring anywhere in the URL. Fast, but can match more than expected.",
    examples: [
      "Pattern: figma -> matches https://www.figma.com/file/123",
      "Pattern: figma -> also matches https://example.com?redirect=figma.com",
    ],
  },
  full_url: {
    title: "Full URL",
    description: "Exact full-string match only.",
    examples: [
      "Pattern: https://app.example.com/a -> matches only that exact URL",
      "Pattern: https://app.example.com/a -> does not match https://app.example.com/a?tab=1",
    ],
  },
  glob: {
    title: "Glob",
    description: "Shell-like wildcards. * = any text, ? = single character.",
    examples: ["Pattern: https://jira.*/browse/ENG-*"],
  },
  regex: {
    title: "Regex",
    description:
      "Full regular expression matching. Most flexible, easiest to misuse.",
    examples: ["Pattern: ^https?://(www\\.)?youtube\\.com/watch"],
  },
};

function PatternTypePicker({
  value,
  onChange,
  name,
}: {
  value: RulePatternType;
  onChange: (value: RulePatternType) => void;
  name: string;
}) {
  const [openPatternType, setOpenPatternType] =
    useState<RulePatternType | null>(null);

  useEffect(() => {
    setOpenPatternType(null);
  }, [value]);

  const openHelp = openPatternType ? PATTERN_HELP[openPatternType] : null;

  return (
    <fieldset
      className="pattern-type-picker"
      onPointerLeave={() => setOpenPatternType(null)}
    >
      <legend>Pattern type</legend>
      <div className="pattern-type-options">
        {PATTERN_OPTIONS.map((option) => {
          const help = PATTERN_HELP[option.value];

          return (
            <label key={option.value} className="pattern-type-option">
              <input
                type="radio"
                name={name}
                value={option.value}
                checked={value === option.value}
                onChange={() => onChange(option.value)}
              />
              <span>{option.label}</span>
              <span
                className="pattern-info-icon"
                tabIndex={0}
                aria-label={`Show help for ${help.title}`}
                title={`How ${help.title} works`}
                onPointerEnter={() => setOpenPatternType(option.value)}
                onFocus={() => setOpenPatternType(option.value)}
                onBlur={() => setOpenPatternType(null)}
              >
                <FiInfo aria-hidden="true" />
              </span>
            </label>
          );
        })}
      </div>
      {openHelp ? (
        <div
          className="pattern-type-popover"
          onPointerDown={(event) => event.stopPropagation()}
        >
          <p>
            <strong>{openHelp.title}</strong>
          </p>
          <p>{openHelp.description}</p>
          {openHelp.examples.map((example) => (
            <p key={example} className="pattern-example">
              {example}
            </p>
          ))}
        </div>
      ) : null}
    </fieldset>
  );
}

function regexErrorsByRule(rules: RuleConfig[]): Record<string, string> {
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

function createRuleId() {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `rule-${crypto.randomUUID()}`;
  }
  return `rule-${Date.now()}`;
}

function createBrowserId() {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `manual-${crypto.randomUUID()}`;
  }
  return `manual-${Date.now()}`;
}

function cloneSet(values: Set<string>) {
  return new Set(values);
}

function settingsDraftFromConfig(config: AppConfig): SettingsDraft {
  return {
    alwaysShowPicker: config.alwaysShowPicker,
    useDefaultsWhenNotRunning: config.useDefaultsWhenNotRunning,
    disableTransparency: config.disableTransparency,
    themePreference: config.themePreference,
    defaultBrowserId: config.defaultBrowserId ?? "",
  };
}

function applySettingsDraft(
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

function App() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [activeTab, setActiveTab] = useState<TabId>("settings");
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [theme, setTheme] = useState<ThemePreference>("light");
  const [status, setStatus] = useState<StatusState>(EMPTY_STATUS);
  const [isLoading, setIsLoading] = useState(true);
  const [saveActivityCount, setSaveActivityCount] = useState(0);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isRegistering, setIsRegistering] = useState(false);
  const [isResettingConfig, setIsResettingConfig] = useState(false);
  const [isStartingOnboarding, setIsStartingOnboarding] = useState(false);
  const [runningBrowserIds, setRunningBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [registrationStatus, setRegistrationStatus] =
    useState<BrowserRegistrationStatus | null>(null);
  const [onboardingStep, setOnboardingStep] = useState(0);
  const [isFinishingOnboarding, setIsFinishingOnboarding] = useState(false);
  const [settingsDraft, setSettingsDraft] = useState<SettingsDraft | null>(
    null,
  );
  const [dirtyRuleIds, setDirtyRuleIds] = useState<Set<string>>(new Set());
  const [savingRuleIds, setSavingRuleIds] = useState<Set<string>>(new Set());
  const [pendingBrowserIds, setPendingBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [savingBrowserIds, setSavingBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [failedBrowserIds, setFailedBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [settingsActionPanel, setSettingsActionPanel] =
    useState<SettingsActionPanel>("none");
  const [formModal, setFormModal] = useState<FormModal>(null);

  const [browserDraft, setBrowserDraft] = useState<BrowserDraft>({
    name: "",
    path: "",
    privateFlag: "",
  });

  const [ruleDraft, setRuleDraft] = useState<RuleDraft>({
    pattern: "",
    patternType: "hostname",
    browserId: "",
    privateMode: false,
  });

  const [routeInput, setRouteInput] = useState(
    "https://github.com/openai/codex",
  );
  const [routeDecision, setRouteDecision] = useState<RouteDecision | null>(
    null,
  );
  const [isRouting, setIsRouting] = useState(false);
  const isSaving = saveActivityCount > 0;
  const configRef = useRef<AppConfig | null>(null);
  const saveQueueRef = useRef<Promise<void>>(Promise.resolve());
  const browserSaveTimersRef = useRef<Record<string, number>>({});
  const contentAreaRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
  }, [theme]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        event.ctrlKey &&
        (event.key.toLowerCase() === "b" || event.code === "KeyB")
      ) {
        event.preventDefault();
        event.stopPropagation();
        setIsSidebarCollapsed((current) => !current);
      }
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
    };
  }, []);

  useEffect(() => {
    const bootstrap = async () => {
      setIsLoading(true);
      try {
        const loaded = await loadConfig();
        applyLoadedConfig(loaded);
        setStatus({ kind: "success", text: "Configuration loaded." });

        const runningIds = await listRunningBrowserIds();
        setRunningBrowserIds(new Set(runningIds));
        await refreshRegistrationStatus();
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setStatus({ kind: "error", text: `Failed to load config: ${message}` });
      } finally {
        setIsLoading(false);
      }
    };

    void bootstrap();
  }, []);

  const visibleBrowsers = useMemo(
    () => config?.browsers.filter((browser) => !browser.isHidden) ?? [],
    [config],
  );

  const hasUnsavedSettings =
    !!config &&
    !!settingsDraft &&
    (settingsDraft.alwaysShowPicker !== config.alwaysShowPicker ||
      settingsDraft.useDefaultsWhenNotRunning !==
        config.useDefaultsWhenNotRunning ||
      settingsDraft.disableTransparency !== config.disableTransparency ||
      settingsDraft.themePreference !== config.themePreference ||
      settingsDraft.defaultBrowserId !== (config.defaultBrowserId ?? ""));

  useEffect(() => {
    if (!visibleBrowsers.length) {
      return;
    }

    setRuleDraft((current) => {
      if (
        !current.browserId ||
        !visibleBrowsers.some((browser) => browser.id === current.browserId)
      ) {
        return { ...current, browserId: visibleBrowsers[0].id };
      }
      return current;
    });
  }, [visibleBrowsers]);

  const regexErrors = useMemo(
    () => (config ? regexErrorsByRule(config.rules) : {}),
    [config],
  );

  useEffect(() => {
    return () => {
      for (const timerId of Object.values(browserSaveTimersRef.current)) {
        window.clearTimeout(timerId);
      }
    };
  }, []);

  useEffect(() => {
    if (!config || config.onboardingCompleted) {
      return;
    }

    window.scrollTo({ top: 0, left: 0, behavior: "auto" });
  }, [config, onboardingStep]);

  useEffect(() => {
    setFormModal(null);
  }, [activeTab]);

  function applyConfigChange(transform: (current: AppConfig) => AppConfig) {
    let nextConfig: AppConfig | null = null;

    setConfig((current) => {
      if (!current) {
        return current;
      }

      nextConfig = transform(current);
      configRef.current = nextConfig;
      return nextConfig;
    });

    return nextConfig;
  }

  function applyLoadedConfig(nextConfig: AppConfig) {
    configRef.current = nextConfig;
    setConfig(nextConfig);
    setTheme(nextConfig.themePreference);
    setSettingsDraft(settingsDraftFromConfig(nextConfig));
  }

  function updateThemePreference(nextTheme: ThemePreference) {
    setTheme(nextTheme);
    setSettingsDraft((current) =>
      current
        ? {
            ...current,
            themePreference: nextTheme,
          }
        : current,
    );

    const currentConfig = configRef.current;
    if (!currentConfig || currentConfig.themePreference === nextTheme) {
      return;
    }

    const nextConfig: AppConfig = {
      ...currentConfig,
      themePreference: nextTheme,
    };

    configRef.current = nextConfig;
    setConfig(nextConfig);
    void persistConfig(nextConfig, {
      errorPrefix: "Could not save theme preference",
    });
  }

  function persistConfig(
    nextConfig: AppConfig,
    options?: {
      successText?: string;
      errorPrefix?: string;
      onSuccess?: (saved: AppConfig) => void;
      onError?: () => void;
      onSettled?: () => void;
    },
  ) {
    setSaveActivityCount((count) => count + 1);

    const runSave = async () => {
      try {
        const saved = await saveConfig(nextConfig);
        applyLoadedConfig(saved);
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

  function scheduleBrowserSave(browserId: string, nextConfig: AppConfig) {
    const existingTimer = browserSaveTimersRef.current[browserId];
    if (existingTimer) {
      window.clearTimeout(existingTimer);
    }

    setPendingBrowserIds((current) => {
      const next = cloneSet(current);
      next.add(browserId);
      return next;
    });
    setFailedBrowserIds((current) => {
      if (!current.has(browserId)) {
        return current;
      }
      const next = cloneSet(current);
      next.delete(browserId);
      return next;
    });

    browserSaveTimersRef.current[browserId] = window.setTimeout(() => {
      delete browserSaveTimersRef.current[browserId];
      setPendingBrowserIds((current) => {
        const next = cloneSet(current);
        next.delete(browserId);
        return next;
      });
      setSavingBrowserIds((current) => {
        const next = cloneSet(current);
        next.add(browserId);
        return next;
      });

      void persistConfig(nextConfig, {
        errorPrefix: "Could not save browser changes",
        onSuccess: () => {
          setSavingBrowserIds((current) => {
            const next = cloneSet(current);
            next.delete(browserId);
            return next;
          });
          setFailedBrowserIds((current) => {
            if (!current.has(browserId)) {
              return current;
            }
            const next = cloneSet(current);
            next.delete(browserId);
            return next;
          });
        },
        onError: () => {
          setSavingBrowserIds((current) => {
            const next = cloneSet(current);
            next.delete(browserId);
            return next;
          });
          setFailedBrowserIds((current) => {
            const next = cloneSet(current);
            next.add(browserId);
            return next;
          });
        },
      });
    }, 500);
  }

  function flushBrowserSave(browserId: string) {
    const existingTimer = browserSaveTimersRef.current[browserId];
    const latestConfig = configRef.current;
    if (!existingTimer || !latestConfig) {
      return;
    }

    window.clearTimeout(existingTimer);
    delete browserSaveTimersRef.current[browserId];
    setPendingBrowserIds((current) => {
      const next = cloneSet(current);
      next.delete(browserId);
      return next;
    });
    setSavingBrowserIds((current) => {
      const next = cloneSet(current);
      next.add(browserId);
      return next;
    });

    void persistConfig(latestConfig, {
      errorPrefix: "Could not save browser changes",
      onSuccess: () => {
        setSavingBrowserIds((current) => {
          const next = cloneSet(current);
          next.delete(browserId);
          return next;
        });
        setFailedBrowserIds((current) => {
          if (!current.has(browserId)) {
            return current;
          }
          const next = cloneSet(current);
          next.delete(browserId);
          return next;
        });
      },
      onError: () => {
        setSavingBrowserIds((current) => {
          const next = cloneSet(current);
          next.delete(browserId);
          return next;
        });
        setFailedBrowserIds((current) => {
          const next = cloneSet(current);
          next.add(browserId);
          return next;
        });
      },
    });
  }

  async function handleSaveSettings() {
    if (!config || !settingsDraft || !hasUnsavedSettings) {
      return;
    }

    const nextConfig = applySettingsDraft(config, settingsDraft);

    await persistConfig(nextConfig, {
      successText: "Settings saved.",
      errorPrefix: "Could not save settings",
    });
  }

  async function refreshRegistrationStatus() {
    try {
      const next = await getBrowserRegistrationStatus();
      setRegistrationStatus(next);
    } catch {
      setRegistrationStatus(null);
    }
  }

  async function refreshRunningState() {
    try {
      const runningIds = await listRunningBrowserIds();
      setRunningBrowserIds(new Set(runningIds));
    } catch {
      // Running-state refresh should be non-blocking.
    }
  }

  async function handleRefreshBrowsers() {
    setIsRefreshing(true);
    try {
      const refreshed = await refreshBrowsers();
      applyLoadedConfig(refreshed);
      setStatus({ kind: "success", text: "Browser detection refreshed." });
      await refreshRunningState();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({ kind: "error", text: `Refresh failed: ${message}` });
    } finally {
      setIsRefreshing(false);
    }
  }

  function updateBrowser(browserId: string, patch: Partial<BrowserConfig>) {
    const nextConfig = applyConfigChange((current) => ({
      ...current,
      browsers: current.browsers.map((browser) =>
        browser.id === browserId ? { ...browser, ...patch } : browser,
      ),
    }));

    if (!nextConfig) {
      return;
    }

    scheduleBrowserSave(browserId, nextConfig);
  }

  function toggleBrowserHidden(browserId: string, isHidden: boolean) {
    const nextConfig = applyConfigChange((current) => ({
      ...current,
      defaultBrowserId:
        isHidden && current.defaultBrowserId === browserId
          ? null
          : current.defaultBrowserId,
      browsers: current.browsers.map((browser) =>
        browser.id === browserId ? { ...browser, isHidden } : browser,
      ),
    }));

    if (!nextConfig) {
      return;
    }

    void persistConfig(nextConfig, {
      successText: isHidden
        ? "Browser hidden from picker."
        : "Browser restored to picker.",
      errorPrefix: "Could not update browser visibility",
    });
  }

  async function addManualBrowser() {
    const name = browserDraft.name.trim();
    const path = browserDraft.path.trim();

    if (!name || !path) {
      setStatus({
        kind: "error",
        text: "Manual browser needs both a name and an executable path.",
      });
      return;
    }

    const browser: BrowserConfig = {
      id: createBrowserId(),
      name,
      path,
      privateFlag: browserDraft.privateFlag.trim() || null,
      source: "manual",
      isHidden: false,
    };

    const nextConfig = applyConfigChange((current) => ({
      ...current,
      browsers: [...current.browsers, browser],
    }));
    if (!nextConfig) {
      return;
    }

    setBrowserDraft({ name: "", path: "", privateFlag: "" });
    await persistConfig(nextConfig, {
      successText: `Added manual browser "${name}".`,
      errorPrefix: "Could not add manual browser",
    });
    setFormModal(null);
  }

  function updateRule(ruleId: string, patch: Partial<RuleConfig>) {
    const nextConfig = applyConfigChange((current) => ({
      ...current,
      rules: current.rules.map((rule) =>
        rule.id === ruleId ? { ...rule, ...patch } : rule,
      ),
    }));
    if (!nextConfig) {
      return;
    }

    setDirtyRuleIds((current) => {
      const next = cloneSet(current);
      next.add(ruleId);
      return next;
    });
  }

  function saveRule(ruleId: string) {
    const currentConfig = configRef.current;
    if (!currentConfig) {
      return;
    }

    const rule = currentConfig.rules.find((item) => item.id === ruleId);
    if (!rule) {
      return;
    }

    if (!rule.pattern.trim()) {
      setStatus({ kind: "error", text: "Rule pattern cannot be empty." });
      return;
    }

    if (!rule.browserId) {
      setStatus({ kind: "error", text: "Select a browser for this rule." });
      return;
    }

    if (rule.patternType === "regex" && regexErrors[rule.id]) {
      setStatus({
        kind: "error",
        text: "Fix this rule's regex before saving.",
      });
      return;
    }

    setSavingRuleIds((current) => {
      const next = cloneSet(current);
      next.add(ruleId);
      return next;
    });

    void persistConfig(currentConfig, {
      successText: "Rule saved.",
      errorPrefix: "Could not save rule",
      onSuccess: () => {
        setDirtyRuleIds((current) => {
          const next = cloneSet(current);
          next.delete(ruleId);
          return next;
        });
        setSavingRuleIds((current) => {
          const next = cloneSet(current);
          next.delete(ruleId);
          return next;
        });
      },
      onError: () => {
        setSavingRuleIds((current) => {
          const next = cloneSet(current);
          next.delete(ruleId);
          return next;
        });
      },
    });
  }

  function deleteRule(ruleId: string) {
    const nextConfig = applyConfigChange((current) => ({
      ...current,
      rules: current.rules.filter((rule) => rule.id !== ruleId),
    }));
    if (!nextConfig) {
      return;
    }

    setDirtyRuleIds((current) => {
      if (!current.has(ruleId)) {
        return current;
      }
      const next = cloneSet(current);
      next.delete(ruleId);
      return next;
    });

    void persistConfig(nextConfig, {
      successText: "Rule deleted.",
      errorPrefix: "Could not delete rule",
    });
  }

  function moveRule(ruleId: string, direction: -1 | 1) {
    let didMove = false;
    const nextConfig = applyConfigChange((current) => {
      const index = current.rules.findIndex((rule) => rule.id === ruleId);
      if (index < 0) {
        return current;
      }

      const destination = index + direction;
      if (destination < 0 || destination >= current.rules.length) {
        return current;
      }

      const nextRules = [...current.rules];
      [nextRules[index], nextRules[destination]] = [
        nextRules[destination],
        nextRules[index],
      ];
      didMove = true;

      return { ...current, rules: nextRules };
    });
    if (!nextConfig || !didMove) {
      return;
    }

    void persistConfig(nextConfig, {
      successText: "Rule order updated.",
      errorPrefix: "Could not reorder rules",
    });
  }

  async function addRule() {
    if (!config) {
      return;
    }

    const pattern = ruleDraft.pattern.trim();
    if (!pattern) {
      setStatus({ kind: "error", text: "Rule pattern cannot be empty." });
      return;
    }

    if (!ruleDraft.browserId) {
      setStatus({ kind: "error", text: "Select a browser for this rule." });
      return;
    }

    if (ruleDraft.patternType === "regex") {
      try {
        new RegExp(pattern);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setStatus({ kind: "error", text: `Regex is invalid: ${message}` });
        return;
      }
    }

    const rule: RuleConfig = {
      id: createRuleId(),
      pattern,
      patternType: ruleDraft.patternType,
      browserId: ruleDraft.browserId,
      privateMode: ruleDraft.privateMode,
      enabled: true,
    };

    const nextConfig: AppConfig = {
      ...config,
      rules: [...config.rules, rule],
    };

    configRef.current = nextConfig;
    setConfig(nextConfig);
    try {
      await persistConfig(nextConfig, {
        successText: "Rule added and saved.",
        errorPrefix: "Could not save new rule",
      });
      setRuleDraft((current) => ({ ...current, pattern: "" }));
      setFormModal(null);
    } catch {
      setConfig(config);
      configRef.current = config;
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

  async function finishOnboarding() {
    if (!config) {
      return;
    }

    const nextConfig: AppConfig = {
      ...config,
      onboardingCompleted: true,
    };

    setIsFinishingOnboarding(true);
    try {
      const saved = await saveConfig(nextConfig);
      applyLoadedConfig(saved);
      setStatus({
        kind: "success",
        text: "Onboarding completed. Hops will now start minimized to tray.",
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not finish onboarding: ${message}`,
      });
    } finally {
      setIsFinishingOnboarding(false);
    }
  }

  async function handleResetConfig() {
    setIsResettingConfig(true);
    try {
      const reset = await resetConfig();
      setSettingsActionPanel("none");
      applyLoadedConfig(reset);
      setDirtyRuleIds(new Set());
      setSavingRuleIds(new Set());
      setPendingBrowserIds(new Set());
      setSavingBrowserIds(new Set());
      setFailedBrowserIds(new Set());
      setRouteDecision(null);
      setBrowserDraft({ name: "", path: "", privateFlag: "" });
      setStatus({
        kind: "success",
        text: "Configuration reset. Rules and manual browsers were cleared, and detected browsers were restored.",
      });
      await refreshRunningState();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not reset configuration: ${message}`,
      });
    } finally {
      setIsResettingConfig(false);
    }
  }

  async function handleRerunOnboarding(shouldResetFirst: boolean) {
    if (!config) {
      return;
    }

    setIsStartingOnboarding(true);
    try {
      const nextConfig = shouldResetFirst
        ? await saveConfig({
            ...(await resetConfig()),
            onboardingCompleted: false,
          })
        : await saveConfig({
            ...applySettingsDraft(config, settingsDraft),
            onboardingCompleted: false,
          });

      setOnboardingStep(0);
      setSettingsActionPanel("none");
      applyLoadedConfig(nextConfig);
      setStatus({
        kind: "success",
        text: shouldResetFirst
          ? "Configuration reset. Onboarding restarted from step 1."
          : "Onboarding restarted with your current configuration intact.",
      });
      await refreshRunningState();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({
        kind: "error",
        text: `Could not restart onboarding: ${message}`,
      });
    } finally {
      setIsStartingOnboarding(false);
    }
  }

  async function runRoutePreview(openImmediately: boolean) {
    if (!config) {
      setStatus({ kind: "error", text: "Configuration is not loaded yet." });
      return;
    }

    if (!routeInput.trim()) {
      setStatus({ kind: "error", text: "Enter a URL to test routing." });
      return;
    }

    setIsRouting(true);
    try {
      const decision = openImmediately
        ? await routeAndOpenWithConfig(config, routeInput.trim())
        : await previewRouteWithConfig(config, routeInput.trim());

      setRouteDecision(decision);
      if (openImmediately && decision.action === "open_browser") {
        setStatus({
          kind: "success",
          text: `Opened in ${decision.browserName ?? "selected browser"}.`,
        });
      } else if (openImmediately && decision.action === "show_picker") {
        await showPickerForUrl(routeInput.trim());
        setStatus({
          kind: "success",
          text: "Routing requires the picker. The picker window was opened.",
        });
      } else {
        setStatus({ kind: "success", text: "Routing preview updated." });
      }
      await refreshRunningState();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus({ kind: "error", text: `Routing failed: ${message}` });
    } finally {
      setIsRouting(false);
    }
  }

  function scrollCurrentPageToTop() {
    contentAreaRef.current?.scrollTo({ top: 0, behavior: "smooth" });
    window.scrollTo({ top: 0, left: 0, behavior: "smooth" });
  }

  const shellClassName = "h-screen overflow-hidden bg-[var(--h-bg)] p-0 md:p-3.5";
  const panelClassName = `grid h-screen overflow-hidden border border-[var(--h-border)] bg-[var(--h-bg)] shadow-none md:h-[calc(100vh-28px)] md:shadow-[4px_4px_0_var(--h-shadow)] ${
    isSidebarCollapsed
      ? "grid-cols-[54px_minmax(0,1fr)]"
      : "grid-cols-[168px_minmax(0,1fr)]"
  }`;
  const sidebarClassName =
    "flex h-full min-h-0 flex-col gap-3 overflow-hidden border-r border-[var(--h-border)] bg-[#075056] p-2.5 text-[#FDF6E3]";
  const contentClassName = "h-full min-h-0 min-w-0 overflow-auto p-3.5 md:p-[18px]";
  const topbarClassName =
    "topbar mb-3.5 flex flex-wrap items-start justify-between gap-4 border-b border-[var(--h-border)] pb-3.5";

  const sidebarNav = (
    <aside className={sidebarClassName} aria-label="Primary">
      <div className="flex min-h-10 items-center gap-2.5 overflow-hidden px-1">
        <span className="grid size-8 shrink-0 place-items-center border border-[#FDF6E3] bg-[var(--h-accent)] text-base font-black text-white shadow-[4px_4px_0_#032f33]">
          H
        </span>
        <span
          className={`whitespace-nowrap text-[13px] font-black uppercase tracking-[0.12em] ${
            isSidebarCollapsed ? "sr-only" : ""
          }`}
        >
          Hops
        </span>
      </div>
      <nav className="grid gap-2">
        {NAV_ITEMS.map((item) => {
          const Icon = item.icon;
          return (
            <button
              key={item.id}
              type="button"
              className={`sidebar-nav-item ${activeTab === item.id ? "active" : ""} ${
                isSidebarCollapsed ? "icon-only" : ""
              }`}
              onClick={() => setActiveTab(item.id)}
              title={item.label}
            >
              <Icon aria-hidden="true" />
              <span className={isSidebarCollapsed ? "sr-only" : ""}>
                {item.label}
              </span>
            </button>
          );
        })}
      </nav>
      <div className="mt-auto grid gap-2">
        <button
          type="button"
          className={`sidebar-toggle secondary ${isSidebarCollapsed ? "icon-only" : ""}`}
          onClick={() =>
            updateThemePreference(theme === "light" ? "dark" : "light")
          }
          aria-label={
            theme === "light" ? "Switch to dark theme" : "Switch to light theme"
          }
          title={
            theme === "light" ? "Switch to dark theme" : "Switch to light theme"
          }
        >
          {theme === "light" ? (
            <FiMoon aria-hidden="true" />
          ) : (
            <FiSun aria-hidden="true" />
          )}
          <span className={isSidebarCollapsed ? "sr-only" : ""}>
            {theme === "light" ? "Dark" : "Light"}
          </span>
        </button>
        <button
          type="button"
          className={`sidebar-toggle secondary ${isSidebarCollapsed ? "icon-only" : ""}`}
          onClick={() => setIsSidebarCollapsed((current) => !current)}
          aria-label={
            isSidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"
          }
          title="Toggle sidebar (Ctrl+B)"
        >
          {isSidebarCollapsed ? (
            <FiChevronRight aria-hidden="true" />
          ) : (
            <FiChevronLeft aria-hidden="true" />
          )}
          <span className={isSidebarCollapsed ? "sr-only" : ""}>Collapse</span>
        </button>
      </div>
    </aside>
  );

  const topButton = (
    <button
      type="button"
      className="floating-top-button"
      onClick={scrollCurrentPageToTop}
      aria-label="Scroll to top"
      title="Top"
    >
      <FiArrowUp aria-hidden="true" />
    </button>
  );

  const statusBanner = status.text ? (
    <div className={`status ${status.kind}`} role="status" aria-live="polite">
      <span>{status.text}</span>
      <button
        type="button"
        className="status-close"
        onClick={() => setStatus(EMPTY_STATUS)}
        aria-label="Dismiss status message"
        title="Dismiss"
      >
        <FiX aria-hidden="true" />
      </button>
    </div>
  ) : null;

  const activeFormModal =
    formModal === "manual-browser" ? (
      <div
        className="modal-backdrop"
        role="presentation"
        onMouseDown={() => setFormModal(null)}
      >
        <section
          className="modal-panel"
          role="dialog"
          aria-modal="true"
          aria-labelledby="manual-browser-title"
          onMouseDown={(event) => event.stopPropagation()}
        >
          <div className="modal-title">
            <h3 id="manual-browser-title">Add manual browser</h3>
            <button
              type="button"
              className="icon-button secondary"
              onClick={() => setFormModal(null)}
              aria-label="Close add manual browser"
              title="Close"
            >
              <FiX aria-hidden="true" />
            </button>
          </div>
          <label>
            Name
            <input
              value={browserDraft.name}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setBrowserDraft((current) => ({
                  ...current,
                  name: value,
                }));
              }}
              placeholder="Portable Chrome"
            />
          </label>
          <label>
            Executable path
            <input
              value={browserDraft.path}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setBrowserDraft((current) => ({
                  ...current,
                  path: value,
                }));
              }}
              placeholder="C:\\Tools\\Chrome\\chrome.exe"
            />
          </label>
          <label>
            Private mode flag
            <input
              value={browserDraft.privateFlag}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setBrowserDraft((current) => ({
                  ...current,
                  privateFlag: value,
                }));
              }}
              placeholder="--incognito"
            />
          </label>
          <div className="inline-actions">
            <button
              type="button"
              onClick={() => void addManualBrowser()}
              disabled={isSaving}
            >
              Add browser
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => setFormModal(null)}
            >
              Cancel
            </button>
          </div>
        </section>
      </div>
    ) : formModal === "rule" ? (
      <div
        className="modal-backdrop"
        role="presentation"
        onMouseDown={() => setFormModal(null)}
      >
        <section
          className="modal-panel"
          role="dialog"
          aria-modal="true"
          aria-labelledby="rule-modal-title"
          onMouseDown={(event) => event.stopPropagation()}
        >
          <div className="modal-title">
            <div>
              <h3 id="rule-modal-title">Add rule</h3>
              <p className="setting-help">New rules are created as enabled.</p>
            </div>
            <button
              type="button"
              className="icon-button secondary"
              onClick={() => setFormModal(null)}
              aria-label="Close add rule"
              title="Close"
            >
              <FiX aria-hidden="true" />
            </button>
          </div>
          <label>
            Pattern
            <input
              value={ruleDraft.pattern}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setRuleDraft((current) => ({
                  ...current,
                  pattern: value,
                }));
              }}
              placeholder="*.notion.so"
            />
          </label>
          <PatternTypePicker
            name="new-rule-pattern-type"
            value={ruleDraft.patternType}
            onChange={(value) =>
              setRuleDraft((current) => ({
                ...current,
                patternType: value,
              }))
            }
          />
          <label>
            Browser
            <select
              value={ruleDraft.browserId}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setRuleDraft((current) => ({
                  ...current,
                  browserId: value,
                }));
              }}
            >
              <option value="">Choose browser</option>
              {visibleBrowsers.map((browser) => (
                <option key={browser.id} value={browser.id}>
                  {browser.name}
                </option>
              ))}
            </select>
          </label>
          <div className="dual-toggle">
            <label className="toggle">
              <input
                type="checkbox"
                checked={ruleDraft.privateMode}
                onChange={(event) => {
                  const checked = event.currentTarget.checked;
                  setRuleDraft((current) => ({
                    ...current,
                    privateMode: checked,
                  }));
                }}
              />
              <span>Private mode</span>
            </label>
          </div>
          <div className="inline-actions">
            <button
              type="button"
              onClick={() => void addRule()}
              disabled={isSaving || !visibleBrowsers.length}
            >
              Add rule
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => setFormModal(null)}
            >
              Cancel
            </button>
          </div>
        </section>
      </div>
    ) : null;

  if (isLoading) {
    return (
      <main className={shellClassName}>
        <section className="h-full overflow-auto border border-[var(--h-border)] bg-[var(--h-bg)] p-[18px] md:shadow-[4px_4px_0_var(--h-shadow)]">
          <h1>Hops</h1>
          <p>Loading configuration...</p>
        </section>
      </main>
    );
  }

  if (!config) {
    return (
      <main className={shellClassName}>
        <section className="h-full overflow-auto border border-[var(--h-border)] bg-[var(--h-bg)] p-[18px] md:shadow-[4px_4px_0_var(--h-shadow)]">
          <h1>Hops</h1>
          <p>
            Could not load configuration. Check the status banner and try again.
          </p>
        </section>
      </main>
    );
  }

  if (!config.onboardingCompleted) {
    return (
      <main className={shellClassName}>
        <section className={panelClassName}>
          {sidebarNav}
          <div className={contentClassName} ref={contentAreaRef}>
            <header className={topbarClassName}>
              <div>
                <p className="section-label">Hops Setup</p>
                <h1>Welcome to Hops</h1>
                <p>Quick onboarding to make external links route correctly.</p>
              </div>
              <p className="badge">Step {onboardingStep + 1} of 4</p>
            </header>

            {statusBanner}

            {onboardingStep === 0 ? (
              <section className="tab-body">
                <h3>1. Detect your browsers</h3>
                <p className="setting-help">
                  We scanned common install locations. Add any missing browser
                  manually.
                </p>

                <div className="inline-actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={handleRefreshBrowsers}
                    disabled={isRefreshing}
                  >
                    {isRefreshing ? "Refreshing..." : "Refresh browsers"}
                  </button>
                </div>

                <div className="browser-list">
                  {config.browsers.length === 0 ? (
                    <article className="card">
                      <p>
                        No browsers detected yet. Refresh scan or add one
                        manually below.
                      </p>
                    </article>
                  ) : (
                    config.browsers.map((browser) => (
                      <article key={browser.id} className="card">
                        <div className="card-title">
                          <strong>{browser.name}</strong>
                          <div className="badges">
                            <span className="badge">{browser.source}</span>
                          </div>
                        </div>
                        <p>{browser.path}</p>
                      </article>
                    ))
                  )}
                </div>

                <article className="card">
                  <h3>Add manual browser</h3>
                  <label>
                    Name
                    <input
                      value={browserDraft.name}
                      onChange={(event) => {
                        const value = event.currentTarget.value;
                        setBrowserDraft((current) => ({
                          ...current,
                          name: value,
                        }));
                      }}
                      placeholder="Portable Browser"
                    />
                  </label>
                  <label>
                    Executable path
                    <input
                      value={browserDraft.path}
                      onChange={(event) => {
                        const value = event.currentTarget.value;
                        setBrowserDraft((current) => ({
                          ...current,
                          path: value,
                        }));
                      }}
                      placeholder="C:\\Tools\\Browser\\browser.exe"
                    />
                  </label>
                  <label>
                    Private mode flag
                    <input
                      value={browserDraft.privateFlag}
                      onChange={(event) => {
                        const value = event.currentTarget.value;
                        setBrowserDraft((current) => ({
                          ...current,
                          privateFlag: value,
                        }));
                      }}
                      placeholder="--incognito"
                    />
                  </label>
                  <div className="inline-actions">
                    <button type="button" onClick={addManualBrowser}>
                      Add browser
                    </button>
                  </div>
                </article>

                <div className="inline-actions">
                  <button type="button" onClick={() => setOnboardingStep(1)}>
                    Continue
                  </button>
                </div>
              </section>
            ) : null}

            {onboardingStep === 1 ? (
              <section className="tab-body">
                <h3>2. Register Hops in Windows Default Apps</h3>
                <p className="setting-help">
                  This writes per-user keys in <code>HKCU</code> so Windows can
                  list Hops as a browser candidate.
                </p>

                <div className="inline-actions">
                  <button
                    type="button"
                    onClick={() => void registerBrowserIntegration()}
                    disabled={isRegistering}
                  >
                    Register Hops
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void refreshRegistrationStatus()}
                    disabled={isRegistering}
                  >
                    Refresh status
                  </button>
                </div>

                <article className="card">
                  <p>
                    Registered in Default Apps list:{" "}
                    <strong>
                      {registrationStatus?.registered ? "Yes" : "No"}
                    </strong>
                  </p>
                </article>

                <div className="inline-actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => setOnboardingStep(0)}
                  >
                    Back
                  </button>
                  <button
                    type="button"
                    onClick={() => setOnboardingStep(2)}
                    disabled={!registrationStatus?.registered}
                  >
                    Continue
                  </button>
                </div>
              </section>
            ) : null}

            {onboardingStep === 2 ? (
              <section className="tab-body">
                <h3>3. Set Hops as default for HTTP and HTTPS</h3>
                <p className="setting-help">
                  Windows requires user confirmation. Open Default Apps and set
                  Hops for both <code>http</code> and <code>https</code>.
                </p>

                <div className="inline-actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={openDefaultAppsSettings}
                  >
                    Open Windows Default Apps
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void refreshRegistrationStatus()}
                  >
                    Refresh status
                  </button>
                </div>

                <article className="card">
                  <p>
                    HTTP default:{" "}
                    <strong>
                      {registrationStatus?.isDefaultHttp ? "Hops" : "Not Hops"}
                    </strong>
                  </p>
                  <p>
                    HTTPS default:{" "}
                    <strong>
                      {registrationStatus?.isDefaultHttps ? "Hops" : "Not Hops"}
                    </strong>
                  </p>
                </article>

                <div className="inline-actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => setOnboardingStep(1)}
                  >
                    Back
                  </button>
                  <button type="button" onClick={() => setOnboardingStep(3)}>
                    Continue
                  </button>
                </div>
              </section>
            ) : null}

            {onboardingStep === 3 ? (
              <section className="tab-body">
                <h3>4. Finish onboarding</h3>
                <p className="setting-help">
                  Hops will keep running in tray and process external links in
                  the background.
                </p>
                {!registrationStatus?.isFullyDefault ? (
                  <p className="status warning">
                    Hops is not yet default for both HTTP and HTTPS. You can
                    finish now and complete this later in Settings.
                  </p>
                ) : null}

                <div className="inline-actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => setOnboardingStep(2)}
                  >
                    Back
                  </button>
                  <button
                    type="button"
                    onClick={() => void finishOnboarding()}
                    disabled={isFinishingOnboarding}
                  >
                    {isFinishingOnboarding
                      ? "Finishing..."
                      : "Finish and open Hops"}
                  </button>
                </div>
              </section>
            ) : null}
            {topButton}
          </div>
        </section>
      </main>
    );
  }

  return (
    <main className={shellClassName}>
      <section className={panelClassName}>
        {sidebarNav}

        <div className={contentClassName} ref={contentAreaRef}>
          <header className={topbarClassName}>
            <div>
              <p className="section-label">Hops Control Center</p>
              <h1>{TAB_TITLES[activeTab].title}</h1>
              <p>{TAB_TITLES[activeTab].subtitle}</p>
            </div>
            <div className="actions">
              {activeTab === "browsers" ? (
                <>
                  <button
                    type="button"
                    className="secondary"
                    onClick={handleRefreshBrowsers}
                    disabled={isRefreshing}
                  >
                    {isRefreshing ? "Refreshing..." : "Refresh browsers"}
                  </button>
                  <button
                    type="button"
                    onClick={() => setFormModal("manual-browser")}
                  >
                    <FiPlus aria-hidden="true" />
                    Add browser
                  </button>
                </>
              ) : null}
              {activeTab === "rules" ? (
                <button
                  type="button"
                  onClick={() => setFormModal("rule")}
                  disabled={!visibleBrowsers.length}
                >
                  <FiPlus aria-hidden="true" />
                  Add rule
                </button>
              ) : null}
            </div>
          </header>

          {activeFormModal}

          {statusBanner}

          {activeTab === "settings" ? (
            <section className="tab-body">
              <label className="toggle">
                <input
                  type="checkbox"
                  checked={settingsDraft?.alwaysShowPicker ?? false}
                  onChange={(event) => {
                    const checked = event.currentTarget.checked;
                    setSettingsDraft((current) =>
                      current
                        ? {
                            ...current,
                            alwaysShowPicker: checked,
                          }
                        : current,
                    );
                  }}
                />
                <span>Always show picker</span>
              </label>
              <p className="setting-help">
                If enabled, Hops skips rules and default browser and always asks
                you where to open.
              </p>

              <label className="toggle">
                <input
                  type="checkbox"
                  checked={settingsDraft?.useDefaultsWhenNotRunning ?? false}
                  onChange={(event) => {
                    const checked = event.currentTarget.checked;
                    setSettingsDraft((current) =>
                      current
                        ? {
                            ...current,
                            useDefaultsWhenNotRunning: checked,
                          }
                        : current,
                    );
                  }}
                />
                <span>Use defaults when browser is not already running</span>
              </label>
              <p className="setting-help">
                If disabled and a matched rule browser is closed, Hops goes to
                picker. If enabled, Hops falls back to your configured default
                browser.
              </p>

              <label className="toggle">
                <input
                  type="checkbox"
                  checked={settingsDraft?.disableTransparency ?? false}
                  onChange={(event) => {
                    const checked = event.currentTarget.checked;
                    setSettingsDraft((current) =>
                      current
                        ? {
                            ...current,
                            disableTransparency: checked,
                          }
                        : current,
                    );
                  }}
                />
                <span>Turn off transparency in picker</span>
              </label>
              <p className="setting-help">
                Stored now for future picker styling. When picker is built, this
                will force a solid background.
              </p>

              <label>
                Theme
                <select
                  value={settingsDraft?.themePreference ?? theme}
                  onChange={(event) => {
                    updateThemePreference(
                      event.currentTarget.value as ThemePreference,
                    );
                  }}
                >
                  <option value="light">Light</option>
                  <option value="dark">Dark</option>
                </select>
              </label>
              <p className="setting-help">
                Theme changes are saved immediately.
              </p>

              <label>
                Default browser
                <select
                  value={settingsDraft?.defaultBrowserId ?? ""}
                  onChange={(event) => {
                    const value = event.currentTarget.value;
                    setSettingsDraft((current) =>
                      current
                        ? {
                            ...current,
                            defaultBrowserId: value,
                          }
                        : current,
                    );
                  }}
                >
                  <option value="">None</option>
                  {visibleBrowsers.map((browser) => (
                    <option key={browser.id} value={browser.id}>
                      {browser.name}
                    </option>
                  ))}
                </select>
              </label>

              <div className="settings-actions">
                <button
                  type="button"
                  onClick={() => void handleSaveSettings()}
                  disabled={!hasUnsavedSettings || isSaving}
                >
                  {isSaving ? "Saving..." : "Save settings"}
                </button>
                {hasUnsavedSettings ? (
                  <p className="setting-help">
                    You have unsaved settings changes.
                  </p>
                ) : null}
              </div>

              <article className="card">
                <h3>Configuration Recovery</h3>
                <p className="setting-help">
                  Reset clears your rules, fallback browser choice, toggles, and
                  manual browser entries. Detected browsers are scanned again
                  immediately. It does not reopen onboarding.
                </p>

                <div className="inline-actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={() =>
                      setSettingsActionPanel((current) =>
                        current === "reset" ? "none" : "reset",
                      )
                    }
                    disabled={isResettingConfig || isStartingOnboarding}
                  >
                    Reset config
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() =>
                      setSettingsActionPanel((current) =>
                        current === "rerun-onboarding"
                          ? "none"
                          : "rerun-onboarding",
                      )
                    }
                    disabled={isResettingConfig || isStartingOnboarding}
                  >
                    Rerun onboarding
                  </button>
                </div>

                {settingsActionPanel === "reset" ? (
                  <div className="action-panel">
                    <p className="setting-help">
                      This removes your current routing rules and manual
                      browsers and restores defaults without reopening
                      onboarding.
                    </p>
                    <div className="inline-actions">
                      <button
                        type="button"
                        onClick={() => void handleResetConfig()}
                        disabled={isResettingConfig}
                      >
                        {isResettingConfig ? "Resetting..." : "Confirm reset"}
                      </button>
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => setSettingsActionPanel("none")}
                        disabled={isResettingConfig}
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : null}

                {settingsActionPanel === "rerun-onboarding" ? (
                  <div className="action-panel">
                    <p className="setting-help">
                      Choose whether onboarding should reuse your current
                      configuration or start from a fresh reset.
                    </p>
                    <div className="inline-actions">
                      <button
                        type="button"
                        onClick={() => void handleRerunOnboarding(false)}
                        disabled={isStartingOnboarding}
                      >
                        {isStartingOnboarding
                          ? "Starting..."
                          : "Keep current config"}
                      </button>
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => void handleRerunOnboarding(true)}
                        disabled={isStartingOnboarding || isResettingConfig}
                      >
                        Reset first
                      </button>
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => setSettingsActionPanel("none")}
                        disabled={isStartingOnboarding}
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : null}
              </article>

              <article className="card">
                <h3>Windows Default App Registration</h3>
                {registrationStatus ? (
                  <>
                    <p className="setting-help">
                      Registered in Default Apps list:{" "}
                      <strong>
                        {registrationStatus.registered ? "Yes" : "No"}
                      </strong>
                    </p>
                    <p className="setting-help">
                      Default for `http`:{" "}
                      <strong>
                        {registrationStatus.isDefaultHttp ? "Yes" : "No"}
                      </strong>
                      {registrationStatus.currentHttpProgId
                        ? ` (current: ${registrationStatus.currentHttpProgId})`
                        : ""}
                    </p>
                    <p className="setting-help">
                      Default for `https`:{" "}
                      <strong>
                        {registrationStatus.isDefaultHttps ? "Yes" : "No"}
                      </strong>
                      {registrationStatus.currentHttpsProgId
                        ? ` (current: ${registrationStatus.currentHttpsProgId})`
                        : ""}
                    </p>
                  </>
                ) : (
                  <p className="setting-help">
                    Registration status is unavailable (usually because this is
                    not Windows).
                  </p>
                )}

                <div className="inline-actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={openDefaultAppsSettings}
                  >
                    Open Windows Default Apps
                  </button>
                  <button
                    type="button"
                    onClick={() => void registerBrowserIntegration()}
                    disabled={isRegistering}
                  >
                    Register Hops
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void unregisterBrowserIntegration()}
                    disabled={isRegistering}
                  >
                    Unregister Hops
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void refreshRegistrationStatus()}
                    disabled={isRegistering}
                  >
                    Refresh status
                  </button>
                </div>

                <p className="setting-help">
                  Register writes only to <code>HKCU</code> (current user) so no
                  admin rights are needed. Unregister removes those same keys.
                </p>
                <p className="setting-help">
                  Before unregistering, switch HTTP/HTTPS defaults away from
                  Hops in Windows to avoid stale associations.
                </p>
                <p className="setting-help">
                  Keys touched: <code>HKCU\Software\Classes\HopsURL</code>,{" "}
                  <code>HKCU\Software\Classes\HopsHTML</code>,{" "}
                  <code>HKCU\Software\Classes\Hops</code>,{" "}
                  <code>HKCU\Software\Hops\Capabilities</code>,{" "}
                  <code>HKCU\Software\RegisteredApplications\Hops</code>.
                </p>
              </article>
            </section>
          ) : null}

          {activeTab === "browsers" ? (
            <section className="tab-body">
              <div className="browser-list">
                {config.browsers.map((browser) => (
                  <article
                    key={browser.id}
                    className={`card ${browser.isHidden ? "muted" : ""}`}
                  >
                    <div className="card-title">
                      <strong>{browser.name}</strong>
                      <div className="badges">
                        <span className="badge">{browser.source}</span>
                        {runningBrowserIds.has(browser.id) ? (
                          <span className="badge running">running</span>
                        ) : null}
                        {browser.isHidden ? (
                          <span className="badge warning">hidden</span>
                        ) : null}
                      </div>
                    </div>

                    <p className="setting-help inline-save-state">
                      {savingBrowserIds.has(browser.id)
                        ? "Saving browser..."
                        : pendingBrowserIds.has(browser.id)
                          ? "Unsaved browser changes..."
                          : failedBrowserIds.has(browser.id)
                            ? "Browser save failed. Keep editing to retry."
                            : "Browser changes save automatically."}
                    </p>

                    <label>
                      Display name
                      <input
                        value={browser.name}
                        onChange={(event) =>
                          updateBrowser(browser.id, {
                            name: event.currentTarget.value,
                          })
                        }
                        onBlur={() => flushBrowserSave(browser.id)}
                      />
                    </label>

                    <label>
                      Executable path
                      <input
                        value={browser.path}
                        onChange={(event) =>
                          updateBrowser(browser.id, {
                            path: event.currentTarget.value,
                          })
                        }
                        onBlur={() => flushBrowserSave(browser.id)}
                      />
                    </label>

                    <label>
                      Private mode flag
                      <input
                        value={browser.privateFlag ?? ""}
                        placeholder="--incognito"
                        onChange={(event) =>
                          updateBrowser(browser.id, {
                            privateFlag:
                              event.currentTarget.value.trim() || null,
                          })
                        }
                        onBlur={() => flushBrowserSave(browser.id)}
                      />
                    </label>

                    <div className="inline-actions">
                      {browser.isHidden ? (
                        <button
                          type="button"
                          className="secondary"
                          onClick={() => toggleBrowserHidden(browser.id, false)}
                        >
                          Restore
                        </button>
                      ) : (
                        <button
                          type="button"
                          className="secondary"
                          onClick={() => toggleBrowserHidden(browser.id, true)}
                        >
                          Hide from picker
                        </button>
                      )}
                    </div>
                  </article>
                ))}
              </div>
            </section>
          ) : null}

          {activeTab === "rules" ? (
            <section className="tab-body">
              {!visibleBrowsers.length ? (
                <p>Add or detect at least one browser before creating rules.</p>
              ) : null}
              <section className="rules-section">
                <h3>Existing rules</h3>
                <p className="setting-help">
                  Rules are evaluated top to bottom. First enabled match wins.
                  Field edits stay local until you save that rule.
                </p>
                <div className="rule-list">
                  {config.rules.map((rule, index) => (
                    <article key={rule.id} className="card">
                      <div className="card-title">
                        <strong>Rule {index + 1}</strong>
                        <div className="inline-actions">
                          <button
                            type="button"
                            className="secondary"
                            onClick={() => moveRule(rule.id, -1)}
                          >
                            Up
                          </button>
                          <button
                            type="button"
                            className="secondary"
                            onClick={() => moveRule(rule.id, 1)}
                          >
                            Down
                          </button>
                          <button
                            type="button"
                            className="secondary danger"
                            onClick={() => deleteRule(rule.id)}
                          >
                            Delete
                          </button>
                        </div>
                      </div>

                      <label>
                        Pattern
                        <input
                          value={rule.pattern}
                          onChange={(event) =>
                            updateRule(rule.id, {
                              pattern: event.currentTarget.value,
                            })
                          }
                          placeholder="github.com"
                        />
                      </label>

                      {regexErrors[rule.id] ? (
                        <p className="field-error">
                          Regex error: {regexErrors[rule.id]}
                        </p>
                      ) : null}
                      {!regexErrors[rule.id] && dirtyRuleIds.has(rule.id) ? (
                        <p className="setting-help inline-save-state">
                          Unsaved rule changes.
                        </p>
                      ) : null}

                      <PatternTypePicker
                        name={`rule-${rule.id}-pattern-type`}
                        value={rule.patternType}
                        onChange={(value) =>
                          updateRule(rule.id, { patternType: value })
                        }
                      />

                      <label>
                        Target browser
                        <select
                          value={rule.browserId}
                          onChange={(event) =>
                            updateRule(rule.id, {
                              browserId: event.currentTarget.value,
                            })
                          }
                        >
                          {visibleBrowsers.map((browser) => (
                            <option key={browser.id} value={browser.id}>
                              {browser.name}
                            </option>
                          ))}
                        </select>
                      </label>

                      <div className="dual-toggle">
                        <label className="toggle">
                          <input
                            type="checkbox"
                            checked={rule.privateMode}
                            onChange={(event) =>
                              updateRule(rule.id, {
                                privateMode: event.currentTarget.checked,
                              })
                            }
                          />
                          <span>Private mode</span>
                        </label>
                        <label className="toggle">
                          <input
                            type="checkbox"
                            checked={rule.enabled}
                            onChange={(event) =>
                              updateRule(rule.id, {
                                enabled: event.currentTarget.checked,
                              })
                            }
                          />
                          <span>Enabled</span>
                        </label>
                      </div>

                      <div className="inline-actions">
                        <button
                          type="button"
                          onClick={() => saveRule(rule.id)}
                          disabled={
                            !dirtyRuleIds.has(rule.id) ||
                            !!regexErrors[rule.id] ||
                            savingRuleIds.has(rule.id)
                          }
                        >
                          {savingRuleIds.has(rule.id)
                            ? "Saving..."
                            : "Save rule"}
                        </button>
                      </div>
                    </article>
                  ))}
                </div>
              </section>
            </section>
          ) : null}

          {activeTab === "router" ? (
            <section className="tab-body">
              <p className="setting-help">
                Route tester simulates what Hops would do for this URL with your
                current in-app config. <strong>Preview route</strong> only shows
                the decision. <strong>Route and open</strong> also launches the
                chosen browser when action is <code>open_browser</code>.
              </p>
              <label>
                URL
                <input
                  value={routeInput}
                  onChange={(event) => setRouteInput(event.currentTarget.value)}
                />
              </label>
              <div className="inline-actions">
                <button
                  type="button"
                  className="secondary"
                  disabled={isRouting}
                  onClick={() => void runRoutePreview(false)}
                >
                  {isRouting ? "Checking..." : "Preview route"}
                </button>
                <button
                  type="button"
                  disabled={isRouting}
                  onClick={() => void runRoutePreview(true)}
                >
                  {isRouting ? "Opening..." : "Route and open"}
                </button>
              </div>

              {routeDecision ? (
                <article className="card">
                  <h3>Routing result</h3>
                  <p>
                    <strong>Action:</strong> {routeDecision.action}
                  </p>
                  <p>
                    <strong>Reason:</strong> {routeDecision.reason}
                  </p>
                  <p>
                    <strong>Browser:</strong>{" "}
                    {routeDecision.browserName ?? "Picker required"}
                  </p>
                  <p>
                    <strong>Private:</strong>{" "}
                    {routeDecision.privateMode ? "Yes" : "No"}
                  </p>
                  <p>
                    <strong>Matched rule:</strong>{" "}
                    {routeDecision.matchedRuleId ?? "None"}
                  </p>
                </article>
              ) : null}
            </section>
          ) : null}
          {topButton}
        </div>
      </section>
    </main>
  );
}

export default App;
