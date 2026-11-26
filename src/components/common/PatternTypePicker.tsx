import { useState } from "react";
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
      "Matches when the URL starts with your pattern. Protocol and domain ignore case; the path must match exactly. Great for locking one path branch.",
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
    description:
      "Exact match of the whole URL. Protocol and domain ignore case, and a missing trailing slash doesn't matter; the path and query must match exactly.",
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

/**
 * The rule's pattern type, as a native select like the other rule fields.
 * The info icon next to the label explains the selected type.
 */
export function PatternTypePicker({
  value,
  onChange,
}: {
  value: RulePatternType;
  onChange: (value: RulePatternType) => void;
}) {
  const [isHelpOpen, setIsHelpOpen] = useState(false);
  const help = PATTERN_HELP[value];

  return (
    <div
      className="pattern-type-picker"
      onPointerLeave={() => setIsHelpOpen(false)}
    >
      <label>
        <span className="label-row">
          Pattern type
          <span
            className="pattern-info-icon"
            tabIndex={0}
            aria-label={`Show help for ${help.title}`}
            title={`How ${help.title} works`}
            onPointerEnter={() => setIsHelpOpen(true)}
            onFocus={() => setIsHelpOpen(true)}
            onBlur={() => setIsHelpOpen(false)}
          >
            <FiInfo aria-hidden="true" />
          </span>
        </span>
        <select
          value={value}
          onChange={(event) =>
            onChange(event.currentTarget.value as RulePatternType)
          }
        >
          {PATTERN_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </label>
      {isHelpOpen ? (
        <div
          className="pattern-type-popover"
          onPointerDown={(event) => event.stopPropagation()}
        >
          <p>
            <strong>{help.title}</strong>
          </p>
          <p>{help.description}</p>
          {help.examples.map((example) => (
            <p key={example} className="pattern-example">
              {example}
            </p>
          ))}
        </div>
      ) : null}
    </div>
  );
}
