import { useEffect, useMemo, useState } from "react";
import { validateRegexPatterns } from "../../services/tauri";
import type { AppConfig } from "../../types";

/**
 * Per-rule regex errors, validated by the backend so the UI uses the same Rust
 * regex engine as routing. Re-validates only when a regex rule's id or pattern
 * changes, so unrelated edits skip the call.
 */
export function useRegexErrors(config: AppConfig | null) {
  const regexRules = useMemo(
    () => config?.rules.filter((rule) => rule.patternType === "regex") ?? [],
    [config],
  );
  const regexRulesKey = useMemo(
    () => JSON.stringify(regexRules.map((rule) => [rule.id, rule.pattern])),
    [regexRules],
  );
  const [regexErrors, setRegexErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    const rules = regexRules;
    if (!rules.length) {
      setRegexErrors((current) =>
        Object.keys(current).length ? {} : current,
      );
      return;
    }

    let cancelled = false;
    validateRegexPatterns(rules.map((rule) => rule.pattern))
      .then((results) => {
        if (cancelled) {
          return;
        }

        const next: Record<string, string> = {};
        results.forEach((error, index) => {
          if (error) {
            next[rules[index].id] = error;
          }
        });
        setRegexErrors(next);
      })
      .catch((error) => {
        console.warn("Could not validate regex rules.", error);
      });

    return () => {
      cancelled = true;
    };
    // regexRulesKey captures every input that affects validation.
  }, [regexRulesKey]);

  return regexErrors;
}
