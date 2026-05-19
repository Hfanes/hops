use crate::browsers::{is_browser_running, running_processes};
use crate::models::{
    AppConfig, BrowserConfig, RouteAction, RouteDecision, RuleConfig, RulePatternType,
};
#[cfg(target_os = "windows")]
use crate::CREATE_NO_WINDOW;
use globset::GlobBuilder;
use regex::Regex;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use url::Url;

pub(crate) fn resolve_route(config: &AppConfig, url: &str) -> Result<RouteDecision, String> {
    let running = running_processes().unwrap_or_default();

    if config.always_show_picker {
        return Ok(picker_decision("always_show_picker"));
    }

    for rule in config.rules.iter().filter(|rule| rule.enabled) {
        if !rule_matches(rule, url) {
            continue;
        }

        let Some(browser) = config
            .browsers
            .iter()
            .find(|browser| browser.id == rule.browser_id && !browser.is_hidden)
        else {
            continue;
        };

        if is_browser_running(browser, &running) || config.use_defaults_when_not_running {
            if is_browser_running(browser, &running) {
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

        if is_browser_running(browser, &running) || config.use_defaults_when_not_running {
            return Ok(open_decision(browser, false, None, "default_browser"));
        }

        return Ok(picker_decision("default_browser_not_running"));
    }

    Ok(picker_decision("no_match"))
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
        RulePatternType::Prefix => url.starts_with(pattern),
        RulePatternType::Contains => url.to_lowercase().contains(&pattern.to_lowercase()),
        RulePatternType::FullUrl => url == pattern,
        RulePatternType::Glob => GlobBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .ok()
            .is_some_and(|glob| glob.compile_matcher().is_match(url)),
        RulePatternType::Regex => Regex::new(pattern).is_ok_and(|regex| regex.is_match(url)),
    }
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

    if !Path::new(&browser.path).exists() {
        return Err(format!(
            "Browser executable does not exist: {}",
            browser.path
        ));
    }

    let mut command = Command::new(&browser.path);
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if private_mode {
        if let Some(flag) = browser.private_flag.as_deref() {
            if !flag.trim().is_empty() {
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
            source: BrowserSource::Manual,
            is_hidden: false,
        }
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

    #[test]
    fn private_rule_picker_decision_preserves_rule_browser_and_private_mode() {
        let browser = manual_browser("brave", "Brave");
        let mut rule = test_rule(RulePatternType::Hostname, "github.com");
        rule.browser_id = browser.id.clone();
        rule.private_mode = true;

        let mut config = test_app_config(vec![browser.clone()]);
        config.rules.push(rule);

        let decision = resolve_route(&config, "https://github.com/openai/codex")
            .expect("route should resolve");

        assert_eq!(decision.action, RouteAction::ShowPicker);
        assert_eq!(decision.browser_id.as_deref(), Some(browser.id.as_str()));
        assert!(decision.private_mode);
        assert_eq!(decision.reason, "rule_browser_not_running");
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

        let decision = resolve_route(&config, "https://github.com/openai/codex")
            .expect("route should resolve");

        assert_eq!(decision.action, RouteAction::OpenBrowser);
        assert_eq!(
            decision.browser_id.as_deref(),
            Some(default_browser.id.as_str())
        );
        assert!(decision.private_mode);
        assert_eq!(decision.reason, "rule_browser_not_running_use_default");
    }
}
