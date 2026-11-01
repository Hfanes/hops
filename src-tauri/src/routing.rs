use crate::browsers::{
    current_running_processes, validate_browser_for_launch, RunningProcessSnapshot,
};
use crate::models::{
    AppConfig, BrowserConfig, RouteAction, RouteDecision, RuleConfig, RulePatternType,
};
#[cfg(target_os = "windows")]
use crate::CREATE_NO_WINDOW;
use globset::GlobBuilder;
use regex::Regex;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use url::Url;

pub(crate) fn resolve_route(config: &AppConfig, url: &str) -> Result<RouteDecision, String> {
    let mut running = current_running_processes(&config.browsers);
    resolve_route_with_running_processes(config, url, &mut running)
}

pub(crate) fn resolve_route_with_running_processes<F>(
    config: &AppConfig,
    url: &str,
    running: &mut RunningProcessSnapshot<F>,
) -> Result<RouteDecision, String>
where
    F: FnMut() -> std::collections::HashSet<String>,
{
    if config.always_show_picker {
        return Ok(picker_decision("always_show_picker"));
    }

    // Normalize once: this is the exact URL the browser will receive.
    let normalized_url = normalized_match_url(url);

    for rule in config.rules.iter().filter(|rule| rule.enabled) {
        if !rule_matches_url(rule, url, normalized_url.as_deref()) {
            continue;
        }

        let Some(browser) = config
            .browsers
            .iter()
            .find(|browser| browser.id == rule.browser_id && !browser.is_hidden)
        else {
            continue;
        };

        let browser_is_running = running.is_browser_running(browser);
        if browser_is_running {
            return Ok(open_decision(
                browser,
                rule.private_mode,
                Some(rule.id.clone()),
                "rule_match_running",
            ));
        }

        if config.use_defaults_when_not_running {
            if let Some(default_browser_id) = config.default_browser_id.as_ref() {
                if let Some(default_browser) = config
                    .browsers
                    .iter()
                    .find(|browser| &browser.id == default_browser_id && !browser.is_hidden)
                {
                    return Ok(open_decision(
                        default_browser,
                        rule.private_mode,
                        Some(rule.id.clone()),
                        "rule_browser_not_running_use_default",
                    ));
                }
            }

            return Ok(picker_decision_for_browser(
                browser,
                rule.private_mode,
                Some(rule.id.clone()),
                "rule_browser_not_running_no_default",
            ));
        }

        return Ok(picker_decision_for_browser(
            browser,
            rule.private_mode,
            Some(rule.id.clone()),
            "rule_browser_not_running",
        ));
    }

    if let Some(default_browser_id) = config.default_browser_id.as_ref() {
        let Some(browser) = config
            .browsers
            .iter()
            .find(|browser| &browser.id == default_browser_id && !browser.is_hidden)
        else {
            return Ok(picker_decision("default_browser_missing"));
        };

        if config.use_defaults_when_not_running || running.is_browser_running(browser) {
            return Ok(open_decision(browser, false, None, "default_browser"));
        }

        return Ok(picker_decision("default_browser_not_running"));
    }

    Ok(picker_decision("no_match"))
}

/// The URL as `open_url_with_browser` will launch it, when it differs from the
/// trimmed input. `None` when the input is not a valid http(s) URL or is
/// already in normalized form.
fn normalized_match_url(url: &str) -> Option<String> {
    let normalized = normalize_http_url(url).ok()?.to_string();
    (normalized != url).then_some(normalized)
}

/// Matches a rule against the incoming URL and its normalized form.
///
/// The normalized form is what the browser receives (lowercase scheme and
/// host, `/` for an empty path), so rules written against it must match. The
/// original is still tried so no rule that matched before stops matching,
/// e.g. a regex written against non-ASCII characters that normalization
/// percent-encodes.
pub(crate) fn rule_matches_url(rule: &RuleConfig, url: &str, normalized_url: Option<&str>) -> bool {
    normalized_url.is_some_and(|normalized| rule_matches(rule, normalized))
        || rule_matches(rule, url)
}

pub(crate) fn rule_matches(rule: &RuleConfig, url: &str) -> bool {
    let pattern = rule.pattern.trim();
    if pattern.is_empty() {
        return false;
    }

    match rule.pattern_type {
        RulePatternType::Hostname => extract_hostname(url)
            .is_some_and(|hostname| hostname.eq_ignore_ascii_case(&pattern.to_lowercase())),
        RulePatternType::HostnameSubdomains => {
            let domain = pattern.trim_start_matches("*.").to_lowercase();
            extract_hostname(url).is_some_and(|hostname| hostname.ends_with(&format!(".{domain}")))
        }
        // Scheme and host are case-insensitive on the web; the rest stays exact.
        RulePatternType::Prefix => {
            url.starts_with(pattern) || url.starts_with(&lowercase_scheme_and_host(pattern))
        }
        RulePatternType::Contains => url.to_lowercase().contains(&pattern.to_lowercase()),
        RulePatternType::FullUrl => {
            // Normalize the pattern the same way as the URL, so `https://Example.com`
            // and `https://example.com/` are the same address. Fall back to the
            // case-folded pattern when it is not a full URL on its own.
            url == pattern
                || url
                    == normalize_http_url(pattern)
                        .map(|parsed| parsed.to_string())
                        .unwrap_or_else(|_| lowercase_scheme_and_host(pattern))
        }
        RulePatternType::Glob => GlobBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .ok()
            .is_some_and(|glob| glob.compile_matcher().is_match(url)),
        RulePatternType::Regex => Regex::new(pattern).is_ok_and(|regex| regex.is_match(url)),
    }
}

/// Checks a regex rule pattern with the same engine and trimming that
/// `rule_matches` uses, returning a single-line error message when invalid.
pub(crate) fn regex_pattern_error(pattern: &str) -> Option<String> {
    let error = Regex::new(pattern.trim()).err()?;
    let message = error.to_string();

    // regex::Error renders a multi-line caret diagram; keep only the summary line.
    let summary = message
        .lines()
        .find_map(|line| line.trim().strip_prefix("error: "))
        .unwrap_or_else(|| message.trim());
    Some(summary.to_string())
}

/// Lowercases the `scheme://host[:port]` part of a URL-like pattern and keeps
/// the path, query, and fragment as written. Works on partial patterns such as
/// `https://linear.app/my` that would not survive a full URL parse unchanged.
fn lowercase_scheme_and_host(pattern: &str) -> String {
    let Some(scheme_end) = pattern.find("://") else {
        return pattern.to_string();
    };
    let authority_start = scheme_end + "://".len();
    let authority_end = pattern[authority_start..]
        .find(['/', '?', '#'])
        .map_or(pattern.len(), |offset| authority_start + offset);

    let mut result = pattern[..authority_end].to_lowercase();
    result.push_str(&pattern[authority_end..]);
    result
}

fn extract_hostname(url: &str) -> Option<String> {
    if let Ok(parsed) = Url::parse(url) {
        return parsed.host_str().map(|value| value.to_lowercase());
    }

    if !url.contains("://") {
        let prefixed = format!("https://{url}");
        if let Ok(parsed) = Url::parse(&prefixed) {
            return parsed.host_str().map(|value| value.to_lowercase());
        }
    }

    None
}

pub(crate) fn normalize_http_url(url: &str) -> Result<Url, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("URL cannot be empty.".to_string());
    }

    let parsed = Url::parse(trimmed).map_err(|error| format!("Invalid URL: {error}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(format!(
                "Unsupported URL scheme '{scheme}'. Only http and https are allowed."
            ));
        }
    }

    if parsed.host_str().is_none() {
        return Err("URL must include a hostname.".to_string());
    }

    Ok(parsed)
}

fn open_decision(
    browser: &BrowserConfig,
    private_mode: bool,
    matched_rule_id: Option<String>,
    reason: &str,
) -> RouteDecision {
    RouteDecision {
        action: RouteAction::OpenBrowser,
        reason: reason.to_string(),
        browser_id: Some(browser.id.clone()),
        browser_name: Some(browser.name.clone()),
        private_mode,
        matched_rule_id,
    }
}

pub(crate) fn picker_decision(reason: &str) -> RouteDecision {
    RouteDecision {
        action: RouteAction::ShowPicker,
        reason: reason.to_string(),
        browser_id: None,
        browser_name: None,
        private_mode: false,
        matched_rule_id: None,
    }
}

fn picker_decision_for_browser(
    browser: &BrowserConfig,
    private_mode: bool,
    matched_rule_id: Option<String>,
    reason: &str,
) -> RouteDecision {
    RouteDecision {
        action: RouteAction::ShowPicker,
        reason: reason.to_string(),
        browser_id: Some(browser.id.clone()),
        browser_name: Some(browser.name.clone()),
        private_mode,
        matched_rule_id,
    }
}

pub(crate) fn open_url_with_browser(
    config: &AppConfig,
    browser_id: &str,
    url: &str,
    private_mode: bool,
) -> Result<(), String> {
    let normalized_url = normalize_http_url(url)?;
    let browser = config
        .browsers
        .iter()
        .find(|browser| browser.id == browser_id && !browser.is_hidden)
        .ok_or_else(|| format!("Browser '{browser_id}' was not found in config."))?;
    let validated = validate_browser_for_launch(browser)?;

    let mut command = Command::new(&validated.executable_path);
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if private_mode {
        if let Some(flag) = validated.private_flag.as_deref() {
            if !flag.is_empty() {
                command.arg(flag);
            }
        }
    }
    command.arg(normalized_url.as_str());
    command
        .spawn()
        .map_err(|error| format!("Could not launch {}: {error}", browser.name))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BrowserConfig, BrowserSource, RuleConfig, ThemePreference};
    use std::collections::HashSet;

    fn test_rule(pattern_type: RulePatternType, pattern: &str) -> RuleConfig {
        RuleConfig {
            id: "test".to_string(),
            pattern: pattern.to_string(),
            pattern_type,
            browser_id: "chrome".to_string(),
            private_mode: false,
            enabled: true,
        }
    }

    fn test_app_config(browsers: Vec<BrowserConfig>) -> AppConfig {
        AppConfig {
            version: 1,
            always_show_picker: false,
            use_defaults_when_not_running: false,
            disable_transparency: false,
            theme_preference: ThemePreference::Light,
            onboarding_completed: true,
            default_browser_id: None,
            browsers,
            rules: Vec::new(),
        }
    }

    fn manual_browser(id: &str, name: &str) -> BrowserConfig {
        BrowserConfig {
            id: id.to_string(),
            name: name.to_string(),
            path: format!("C:\\Tools\\{name}\\browser.exe"),
            private_flag: Some("--incognito".to_string()),
            icon_key: Some("generic".to_string()),
            manual_trust: None,
            source: BrowserSource::Manual,
            is_hidden: false,
        }
    }

    fn resolve_route_with_empty_running_set(
        config: &AppConfig,
        url: &str,
    ) -> (RouteDecision, usize) {
        let mut scan_count = 0;
        let decision = {
            let mut running = RunningProcessSnapshot::new(|| {
                scan_count += 1;
                HashSet::new()
            });

            resolve_route_with_running_processes(config, url, &mut running)
                .expect("route should resolve")
        };

        (decision, scan_count)
    }

    #[test]
    fn always_show_picker_returns_without_running_process_scan() {
        let mut config = test_app_config(vec![manual_browser("chrome", "Chrome")]);
        config.always_show_picker = true;
        config.default_browser_id = Some("chrome".to_string());

        let (decision, scan_count) =
            resolve_route_with_empty_running_set(&config, "https://github.com/openai/codex");

        assert_eq!(decision.action, RouteAction::ShowPicker);
        assert_eq!(decision.reason, "always_show_picker");
        assert_eq!(scan_count, 0);
    }

    #[test]
    fn hostname_match_is_exact() {
        let rule = test_rule(RulePatternType::Hostname, "github.com");
        assert!(rule_matches(&rule, "https://github.com/openai"));
        assert!(!rule_matches(&rule, "https://api.github.com"));
    }

    #[test]
    fn hostname_subdomain_match_works() {
        let rule = test_rule(RulePatternType::HostnameSubdomains, "*.notion.so");
        assert!(rule_matches(&rule, "https://workspace.notion.so/page"));
        assert!(!rule_matches(&rule, "https://notion.so/page"));
    }

    #[test]
    fn glob_match_works() {
        let rule = test_rule(RulePatternType::Glob, "https://jira.*/browse/ENG-*");
        assert!(rule_matches(
            &rule,
            "https://jira.mycompany.com/browse/ENG-11"
        ));
        assert!(!rule_matches(
            &rule,
            "https://jira.mycompany.com/browse/OPS-11"
        ));
    }

    #[test]
    fn regex_match_works() {
        let rule = test_rule(
            RulePatternType::Regex,
            r"^https?://(www\.)?youtube\.com/watch",
        );
        assert!(rule_matches(&rule, "https://youtube.com/watch?v=abc"));
        assert!(!rule_matches(&rule, "https://youtube.com/shorts/abc"));
    }

    fn matches_incoming(rule: &RuleConfig, url: &str) -> bool {
        rule_matches_url(rule, url, normalized_match_url(url).as_deref())
    }

    #[test]
    fn full_url_matches_the_url_the_browser_receives() {
        let rule = test_rule(RulePatternType::FullUrl, "https://example.com/");
        assert!(matches_incoming(&rule, "https://example.com"));
        assert!(matches_incoming(&rule, "HTTPS://Example.COM/"));

        let rule = test_rule(RulePatternType::FullUrl, "https://Example.com");
        assert!(matches_incoming(&rule, "https://example.com/"));
    }

    #[test]
    fn full_url_keeps_path_and_query_exact() {
        let rule = test_rule(RulePatternType::FullUrl, "https://app.example.com/a");
        assert!(matches_incoming(&rule, "https://app.example.com/a"));
        assert!(!matches_incoming(&rule, "https://app.example.com/A"));
        assert!(!matches_incoming(&rule, "https://app.example.com/a?tab=1"));
    }

    #[test]
    fn prefix_ignores_scheme_and_host_case_only() {
        let rule = test_rule(RulePatternType::Prefix, "https://GitHub.com/openai");
        assert!(matches_incoming(&rule, "https://github.com/openai/codex"));
        assert!(matches_incoming(&rule, "HTTPS://GITHUB.COM/openai/codex"));
        assert!(!matches_incoming(&rule, "https://github.com/OpenAI/codex"));
        assert!(!matches_incoming(&rule, "https://github.com/other"));
    }

    #[test]
    fn prefix_partial_pattern_still_matches() {
        let rule = test_rule(RulePatternType::Prefix, "https://linear.app/my");
        assert!(matches_incoming(
            &rule,
            "https://Linear.app/myteam/issue/ENG-1"
        ));
    }

    #[test]
    fn rules_written_against_the_raw_url_keep_matching() {
        // Normalization percent-encodes the path; the raw form is still tried.
        let rule = test_rule(RulePatternType::Regex, "/café$");
        assert!(matches_incoming(&rule, "https://example.com/café"));

        let rule = test_rule(RulePatternType::Contains, "café");
        assert!(matches_incoming(&rule, "https://example.com/café"));
    }

    #[test]
    fn lowercase_scheme_and_host_leaves_the_rest_untouched() {
        assert_eq!(
            lowercase_scheme_and_host("HTTPS://GitHub.com:8443/OpenAI?Q=1#Top"),
            "https://github.com:8443/OpenAI?Q=1#Top"
        );
        assert_eq!(
            lowercase_scheme_and_host("https://Linear.App"),
            "https://linear.app"
        );
        assert_eq!(lowercase_scheme_and_host("Figma"), "Figma");
    }

    #[test]
    fn resolve_route_matches_rule_against_normalized_url() {
        let browser = manual_browser("chrome", "Chrome");
        let mut rule = test_rule(RulePatternType::FullUrl, "https://example.com/");
        rule.browser_id = browser.id.clone();
        let mut config = test_app_config(vec![browser]);
        config.rules.push(rule);

        let (decision, _) = resolve_route_with_empty_running_set(&config, "https://EXAMPLE.com");

        assert_eq!(decision.matched_rule_id.as_deref(), Some("test"));
        assert_eq!(decision.reason, "rule_browser_not_running");
    }

    #[test]
    fn regex_pattern_error_uses_rust_regex_syntax() {
        // Valid in Rust's regex crate (rejected by JavaScript's RegExp).
        assert_eq!(regex_pattern_error(r"(?i)^https://github\.com/"), None);
        // Valid in JavaScript's RegExp but unsupported by Rust's regex crate.
        let look_ahead = regex_pattern_error(r"^https://(?=www\.)").expect("look-around rejected");
        assert!(look_ahead.contains("look-around"), "{look_ahead}");
        assert!(!look_ahead.contains('\n'), "{look_ahead}");
        assert!(regex_pattern_error(r"(\w+)\1").is_some());
        assert!(regex_pattern_error("(").is_some());
    }

    #[test]
    fn private_rule_picker_decision_preserves_rule_browser_and_private_mode() {
        let browser = manual_browser("brave", "Brave");
        let mut rule = test_rule(RulePatternType::Hostname, "github.com");
        rule.browser_id = browser.id.clone();
        rule.private_mode = true;

        let mut config = test_app_config(vec![browser.clone()]);
        config.rules.push(rule);

        let (decision, scan_count) =
            resolve_route_with_empty_running_set(&config, "https://github.com/openai/codex");

        assert_eq!(decision.action, RouteAction::ShowPicker);
        assert_eq!(decision.browser_id.as_deref(), Some(browser.id.as_str()));
        assert!(decision.private_mode);
        assert_eq!(decision.reason, "rule_browser_not_running");
        assert_eq!(scan_count, 1);
    }

    #[test]
    fn private_rule_default_fallback_preserves_private_mode() {
        let rule_browser = manual_browser("brave", "Brave");
        let default_browser = manual_browser("chrome", "Chrome");
        let mut rule = test_rule(RulePatternType::Hostname, "github.com");
        rule.browser_id = rule_browser.id.clone();
        rule.private_mode = true;

        let mut config = test_app_config(vec![rule_browser, default_browser.clone()]);
        config.use_defaults_when_not_running = true;
        config.default_browser_id = Some(default_browser.id.clone());
        config.rules.push(rule);

        let (decision, scan_count) =
            resolve_route_with_empty_running_set(&config, "https://github.com/openai/codex");

        assert_eq!(decision.action, RouteAction::OpenBrowser);
        assert_eq!(
            decision.browser_id.as_deref(),
            Some(default_browser.id.as_str())
        );
        assert!(decision.private_mode);
        assert_eq!(decision.reason, "rule_browser_not_running_use_default");
        assert_eq!(scan_count, 1);
    }

    #[test]
    fn default_browser_with_default_fallback_opens_without_running_process_scan() {
        let default_browser = manual_browser("chrome", "Chrome");
        let mut config = test_app_config(vec![default_browser.clone()]);
        config.use_defaults_when_not_running = true;
        config.default_browser_id = Some(default_browser.id.clone());

        let (decision, scan_count) =
            resolve_route_with_empty_running_set(&config, "https://example.com");

        assert_eq!(decision.action, RouteAction::OpenBrowser);
        assert_eq!(decision.browser_id.as_deref(), Some("chrome"));
        assert_eq!(decision.reason, "default_browser");
        assert_eq!(scan_count, 0);
    }

    #[test]
    fn no_match_and_no_default_returns_picker_without_running_process_scan() {
        let browser = manual_browser("chrome", "Chrome");
        let mut rule = test_rule(RulePatternType::Hostname, "github.com");
        rule.browser_id = browser.id.clone();

        let mut config = test_app_config(vec![browser]);
        config.rules.push(rule);

        let (decision, scan_count) =
            resolve_route_with_empty_running_set(&config, "https://example.com");

        assert_eq!(decision.action, RouteAction::ShowPicker);
        assert_eq!(decision.reason, "no_match");
        assert_eq!(scan_count, 0);
    }
}
