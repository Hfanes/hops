import { useEffect, useState } from "react";
import type { DeleteConfirmation } from "../../components/common/DeleteConfirmModal";
import type { ConfigStore } from "../../hooks/useConfigStore";
import { cloneSet, createRuleId } from "../../lib/appTypes";
import { validateRegexPatterns } from "../../services/tauri";
import type {
  AppConfig,
  BrowserConfig,
  FormModal,
  RuleConfig,
  RuleDraft,
  StatusState,
} from "../../types";
import { useRegexErrors } from "./useRegexErrors";

/**
 * Everything the Rules tab does to rules: inline edits saved explicitly per
 * rule, reordering, deleting, and adding new rules from the draft form.
 *
 * Modals shared with other features stay with the caller, which passes in
 * callbacks to open them.
 */
export function useRuleManager({
  store,
  visibleBrowsers,
  setStatus,
  setFormModal,
  onRequestDelete,
}: {
  store: ConfigStore;
  visibleBrowsers: BrowserConfig[];
  setStatus: (status: StatusState) => void;
  setFormModal: (modal: FormModal) => void;
  onRequestDelete: (confirmation: DeleteConfirmation) => void;
}) {
  const { config, configRef, setCurrentConfig, applyConfigChange, persistConfig } =
    store;

  const [dirtyRuleIds, setDirtyRuleIds] = useState<Set<string>>(new Set());
  const [savingRuleIds, setSavingRuleIds] = useState<Set<string>>(new Set());
  const [ruleDraft, setRuleDraft] = useState<RuleDraft>({
    pattern: "",
    patternType: "hostname",
    browserId: "",
    privateMode: false,
  });
  const regexErrors = useRegexErrors(config);

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

  function requestDeleteRule(ruleId: string) {
    const currentConfig = configRef.current;
    const ruleIndex = currentConfig?.rules.findIndex(
      (item) => item.id === ruleId,
    );
    if (!currentConfig || ruleIndex === undefined || ruleIndex < 0) {
      return;
    }

    const rule = currentConfig.rules[ruleIndex];
    onRequestDelete({
      kind: "rule",
      ruleId,
      ruleLabel: `Rule ${ruleIndex + 1}`,
      pattern: rule.pattern,
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
        const [regexError] = await validateRegexPatterns([pattern]);
        if (regexError) {
          setStatus({ kind: "error", text: `Regex is invalid: ${regexError}` });
          return;
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        setStatus({ kind: "error", text: `Could not validate regex: ${message}` });
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

    setCurrentConfig(nextConfig);
    try {
      await persistConfig(nextConfig, {
        successText: "Rule added and saved.",
        errorPrefix: "Could not save new rule",
      });
      setRuleDraft((current) => ({ ...current, pattern: "" }));
      setFormModal(null);
    } catch {
      setCurrentConfig(config);
    }
  }

  /** Clears per-rule edit and save indicators, e.g. after a config reset. */
  function resetRuleEditState() {
    setDirtyRuleIds(new Set());
    setSavingRuleIds(new Set());
  }

  return {
    dirtyRuleIds,
    savingRuleIds,
    ruleDraft,
    setRuleDraft,
    regexErrors,
    updateRule,
    saveRule,
    deleteRule,
    requestDeleteRule,
    moveRule,
    addRule,
    resetRuleEditState,
  };
}
