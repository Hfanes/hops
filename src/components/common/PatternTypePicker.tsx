import { useEffect, useState } from "react";
import { FiInfo } from "react-icons/fi";
import type { RulePatternType } from "../../types";

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

export function PatternTypePicker({
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
