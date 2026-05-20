use crate::models::{
    AppConfig, BrowserConfig, BrowserRecognition, BrowserSource, ManualBrowserTrust,
    ManualBrowserValidationRequest, ManualBrowserValidationResult,
};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "windows"))]
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
#[cfg(target_os = "windows")]
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use winreg::HKEY;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserFamily {
    Chromium,
    Firefox,
    Edge,
    Opera,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
struct KnownBrowserDefinition {
    executable_aliases: &'static [&'static str],
    display_name: &'static str,
    family: BrowserFamily,
    private_flag_override: Option<&'static str>,
    use_family_private_flag: bool,
    known_install_path_suffixes: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedBrowserMetadata {
    pub(crate) name: String,
    pub(crate) family: BrowserFamily,
    pub(crate) private_flag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserRecognitionResult {
    pub(crate) recognition: BrowserRecognition,
    pub(crate) name: String,
    pub(crate) family: Option<BrowserFamily>,
    pub(crate) private_flag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidatedBrowserLaunch {
    pub(crate) executable_path: String,
    pub(crate) private_flag: Option<String>,
}

pub(crate) fn hydrate_detected_browser_defaults(config: &mut AppConfig) {
    for browser in &mut config.browsers {
        if browser.source != BrowserSource::Detected || browser.private_flag.is_some() {
            continue;
        }

        let resolved = resolve_browser_metadata(&browser.path, Some(browser.name.as_str()), None);
        if resolved.private_flag.is_some() {
            browser.private_flag = resolved.private_flag;
        }
    }
}

pub(crate) fn normalize_manual_browser_entries(config: &mut AppConfig) {
    for browser in &mut config.browsers {
        if browser.source != BrowserSource::Manual {
            continue;
        }

        let Ok(recognition) = classify_browser_path(&browser.path, Some(browser.name.as_str())) else {
            continue;
        };

        match recognition.recognition {
            BrowserRecognition::Known | BrowserRecognition::RecognizedFamily => {
                browser.manual_trust = Some(ManualBrowserTrust::Verified);
                browser.private_flag = recognition.private_flag.clone();
                if browser.name.trim().is_empty() {
                    browser.name = recognition.name;
                }
            }
            BrowserRecognition::UnverifiedManual => {
                if browser.manual_trust != Some(ManualBrowserTrust::UserConfirmed) {
                    browser.manual_trust = None;
                }
                browser.private_flag = None;
            }
        }
    }
}

pub(crate) fn merge_detected_browsers(
    config: &mut AppConfig,
    detected_browsers: Vec<BrowserConfig>,
) {
    let hidden_by_detected_id: HashMap<String, bool> = config
        .browsers
        .iter()
        .filter(|browser| browser.source == BrowserSource::Detected)
        .map(|browser| (browser.id.clone(), browser.is_hidden))
        .collect();

    let manual_browsers: Vec<BrowserConfig> = config
        .browsers
        .iter()
        .filter(|browser| browser.source == BrowserSource::Manual)
        .cloned()
        .collect();

    let manual_paths: HashSet<String> = manual_browsers
        .iter()
        .map(|browser| normalize_path(&browser.path))
        .collect();

    let mut merged = manual_browsers;

    for mut browser in detected_browsers {
        if manual_paths.contains(&normalize_path(&browser.path)) {
            continue;
        }

        browser.is_hidden = hidden_by_detected_id
            .get(&browser.id)
            .copied()
            .unwrap_or(false);
        merged.push(browser);
    }

    merged.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    config.browsers = merged;
}

pub(crate) fn detect_browsers() -> Vec<BrowserConfig> {
    #[cfg(target_os = "windows")]
    {
        detect_browsers_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "windows")]
fn detect_browsers_windows() -> Vec<BrowserConfig> {
    let mut browsers: Vec<BrowserConfig> = Vec::new();
    let mut seen_paths: HashSet<String> = HashSet::new();

    for browser in detect_browsers_from_registry() {
        let key = normalize_path(&browser.path);
        if seen_paths.insert(key) {
            browsers.push(browser);
        }
    }

    for browser in detect_browsers_from_known_paths(&seen_paths) {
        let key = normalize_path(&browser.path);
        if seen_paths.insert(key) {
            browsers.push(browser);
        }
    }

    browsers.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    browsers
}

#[cfg(target_os = "windows")]
fn detect_browsers_from_known_paths(seen_paths: &HashSet<String>) -> Vec<BrowserConfig> {
    let mut browsers = Vec::new();
    let mut local_seen = seen_paths.clone();

    for definition in known_browser_definitions() {
        for candidate in known_install_paths(definition) {
            if !candidate.exists() {
                continue;
            }

            let path_string = candidate.to_string_lossy().to_string();
            let normalized = normalize_path(&path_string);
            if local_seen.contains(&normalized) {
                continue;
            }

            local_seen.insert(normalized.clone());
            browsers.push(build_detected_browser(
                &path_string,
                Some(definition.display_name),
                None,
            ));
            break;
        }
    }

    browsers
}

#[cfg(target_os = "windows")]
fn detect_browsers_from_registry() -> Vec<BrowserConfig> {
    let mut browsers = Vec::new();
    let mut local_seen = HashSet::new();
    for (root_hkey, base) in registry_browser_roots() {
        let root = RegKey::predef(*root_hkey);
        let Ok(root_key) = root.open_subkey(base) else {
            continue;
        };

        for subkey_name in root_key.enum_keys().flatten() {
            let command_key_path = format!(r"{base}\{subkey_name}\shell\open\command");
            let Ok(command_key) = root.open_subkey(command_key_path) else {
                continue;
            };

            let Ok(raw_command): Result<String, _> = command_key.get_value("") else {
                continue;
            };

            let Some(executable_path) = extract_executable_path(&raw_command) else {
                continue;
            };

            if !Path::new(&executable_path).exists() {
                continue;
            }

            let normalized = normalize_path(&executable_path);
            if local_seen.contains(&normalized) {
                continue;
            }

            local_seen.insert(normalized.clone());
            browsers.push(build_detected_browser(
                &executable_path,
                None,
                Some(subkey_name.as_str()),
            ));
        }
    }

    browsers
}

#[cfg(target_os = "windows")]
pub(crate) fn registry_browser_roots() -> &'static [(HKEY, &'static str)] {
    &[
        (HKEY_CURRENT_USER, r"SOFTWARE\Clients\StartMenuInternet"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Clients\StartMenuInternet"),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Clients\StartMenuInternet",
        ),
    ]
}

#[cfg(target_os = "windows")]
fn extract_executable_path(command: &str) -> Option<String> {
    let trimmed = command.trim();

    if let Some(stripped) = trimmed.strip_prefix('"') {
        let end = stripped.find('"')?;
        return Some(stripped[..end].to_string());
    }

    let lower = trimmed.to_lowercase();
    let end = lower.find(".exe")?;
    Some(trimmed[..end + 4].trim().to_string())
}

pub(crate) fn build_detected_browser(
    path: &str,
    display_name_hint: Option<&str>,
    registry_key_hint: Option<&str>,
) -> BrowserConfig {
    let normalized = normalize_path(path);
    let metadata = resolve_browser_metadata(path, display_name_hint, registry_key_hint);

    BrowserConfig {
        id: stable_id("detected", &normalized),
        name: metadata.name,
        path: path.to_string(),
        private_flag: metadata.private_flag,
        manual_trust: None,
        source: BrowserSource::Detected,
        is_hidden: false,
    }
}

pub(crate) fn validate_browser_for_launch(
    browser: &BrowserConfig,
) -> Result<ValidatedBrowserLaunch, String> {
    let path = browser.path.trim();
    if path.is_empty() {
        return Err(format!(
            "Browser '{}' is missing an executable path.",
            browser.name
        ));
    }

    #[cfg(target_os = "windows")]
    if !path.to_ascii_lowercase().ends_with(".exe") {
        return Err(format!(
            "Browser '{}' must point to a .exe executable.",
            browser.name
        ));
    }

    if !Path::new(path).exists() {
        return Err(format!("Browser executable does not exist: {path}"));
    }

    let recognition = classify_browser_path(path, Some(browser.name.as_str()))?;
    let private_flag = validate_private_flag(browser, &recognition)?;

    if browser.source == BrowserSource::Manual {
        match recognition.recognition {
            BrowserRecognition::Known | BrowserRecognition::RecognizedFamily => {}
            BrowserRecognition::UnverifiedManual => {
                if browser.manual_trust != Some(ManualBrowserTrust::UserConfirmed) {
                    return Err(format!(
                        "Manual browser '{}' needs explicit confirmation before Hops can launch it.",
                        browser.name
                    ));
                }
            }
        }
    }

    Ok(ValidatedBrowserLaunch {
        executable_path: path.to_string(),
        private_flag,
    })
}

pub(crate) fn validate_manual_browser_request(
    request: &ManualBrowserValidationRequest,
) -> Result<ManualBrowserValidationResult, String> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err("Manual browser needs an executable path.".to_string());
    }

    #[cfg(target_os = "windows")]
    if !path.to_ascii_lowercase().ends_with(".exe") {
        return Err("Manual browser executable must be a .exe file.".to_string());
    }

    if !Path::new(path).exists() {
        return Err(format!("Browser executable does not exist: {path}"));
    }

    let recognition = classify_browser_path(path, Some(request.name.as_str()))?;
    let private_flag = validate_private_flag_for_request(request, &recognition)?;

    let (manual_trust, requires_confirmation, message) = match recognition.recognition {
        BrowserRecognition::Known => (
            Some(ManualBrowserTrust::Verified),
            false,
            format!("Recognized as {}.", recognition.name),
        ),
        BrowserRecognition::RecognizedFamily => (
            Some(ManualBrowserTrust::Verified),
            false,
            format!(
                "Recognized as a {}-family browser.",
                browser_family_label(recognition.family)
            ),
        ),
        BrowserRecognition::UnverifiedManual if request.allow_user_confirmed => (
            Some(ManualBrowserTrust::UserConfirmed),
            false,
            format!(
                "Saved '{}' as a user-confirmed manual browser.",
                preferred_browser_name(request.name.as_str(), path)
            ),
        ),
        BrowserRecognition::UnverifiedManual => (
            None,
            true,
            "This executable is not recognized as a supported browser. Confirm to allow Hops to launch it as a manual browser.".to_string(),
        ),
    };

    Ok(ManualBrowserValidationResult {
        recognition: recognition.recognition,
        manual_trust,
        browser_name: match recognition.recognition {
            BrowserRecognition::Known | BrowserRecognition::RecognizedFamily => {
                recognition.name.clone()
            }
            BrowserRecognition::UnverifiedManual => {
                preferred_browser_name(request.name.as_str(), path)
            }
        },
        private_flag,
        family: recognition
            .family
            .and_then(|family| browser_family_key(family).map(str::to_string)),
        requires_confirmation,
        message,
    })
}

pub(crate) fn resolve_browser_metadata(
    path: &str,
    display_name_hint: Option<&str>,
    registry_key_hint: Option<&str>,
) -> ResolvedBrowserMetadata {
    let exe_name = executable_name_from_path(path).unwrap_or_else(|| "browser".to_string());
    let exe_name = exe_name.to_lowercase();
    let normalized_path = normalize_path(path);

    if let Some(definition) =
        display_name_hint.and_then(find_known_browser_definition_by_display_name)
    {
        return ResolvedBrowserMetadata {
            name: definition.display_name.to_string(),
            family: definition.family,
            private_flag: resolved_private_flag_for_definition(definition),
        };
    }

    if let Some(definition) = known_browser_definitions().iter().find(|definition| {
        definition
            .known_install_path_suffixes
            .iter()
            .any(|suffix| normalized_path.ends_with(&normalize_path(suffix)))
    }) {
        return ResolvedBrowserMetadata {
            name: definition.display_name.to_string(),
            family: definition.family,
            private_flag: resolved_private_flag_for_definition(definition),
        };
    }

    if let Some(definition) = known_browser_definitions()
        .iter()
        .find(|definition| definition.executable_aliases.contains(&exe_name.as_str()))
    {
        return ResolvedBrowserMetadata {
            name: definition.display_name.to_string(),
            family: definition.family,
            private_flag: resolved_private_flag_for_definition(definition),
        };
    }

    let family = BrowserFamily::Unknown;
    let name = display_name_hint
        .or(registry_key_hint)
        .map(|value| value.replace('_', " "))
        .unwrap_or_else(|| exe_name.clone());

    ResolvedBrowserMetadata {
        name,
        family,
        private_flag: default_private_flag_for_family(family),
    }
}

pub(crate) fn classify_browser_path(
    path: &str,
    display_name_hint: Option<&str>,
) -> Result<BrowserRecognitionResult, String> {
    if path.trim().is_empty() {
        return Err("Browser executable path cannot be empty.".to_string());
    }

    let metadata = resolve_browser_metadata(path, display_name_hint, None);
    if metadata.family != BrowserFamily::Unknown {
        let known = is_known_browser_match(path, display_name_hint);
        return Ok(BrowserRecognitionResult {
            recognition: if known {
                BrowserRecognition::Known
            } else {
                BrowserRecognition::RecognizedFamily
            },
            name: metadata.name,
            family: Some(metadata.family),
            private_flag: metadata.private_flag,
        });
    }

    let normalized_path = normalize_path(path);
    let hint = display_name_hint.unwrap_or_default().to_lowercase();
    let stem = executable_name_from_path(path)
        .unwrap_or_else(|| "browser".to_string())
        .to_lowercase();
    let haystack = format!("{normalized_path} {hint} {stem}");

    if haystack.contains("tor browser") || haystack.contains("\\tor\\browser\\firefox.exe") {
        return Ok(BrowserRecognitionResult {
            recognition: BrowserRecognition::RecognizedFamily,
            name: "Tor Browser".to_string(),
            family: Some(BrowserFamily::Firefox),
            private_flag: None,
        });
    }

    if firefox_family_tokens().iter().any(|token| haystack.contains(token)) {
        return Ok(BrowserRecognitionResult {
            recognition: BrowserRecognition::RecognizedFamily,
            name: preferred_browser_name(display_name_hint.unwrap_or_default(), path),
            family: Some(BrowserFamily::Firefox),
            private_flag: default_private_flag_for_family(BrowserFamily::Firefox),
        });
    }

    if edge_family_tokens().iter().any(|token| haystack.contains(token)) {
        return Ok(BrowserRecognitionResult {
            recognition: BrowserRecognition::RecognizedFamily,
            name: preferred_browser_name(display_name_hint.unwrap_or_default(), path),
            family: Some(BrowserFamily::Edge),
            private_flag: default_private_flag_for_family(BrowserFamily::Edge),
        });
    }

    if opera_family_tokens().iter().any(|token| haystack.contains(token)) {
        return Ok(BrowserRecognitionResult {
            recognition: BrowserRecognition::RecognizedFamily,
            name: preferred_browser_name(display_name_hint.unwrap_or_default(), path),
            family: Some(BrowserFamily::Opera),
            private_flag: default_private_flag_for_family(BrowserFamily::Opera),
        });
    }

    if chromium_family_tokens()
        .iter()
        .any(|token| haystack.contains(token))
    {
        return Ok(BrowserRecognitionResult {
            recognition: BrowserRecognition::RecognizedFamily,
            name: preferred_browser_name(display_name_hint.unwrap_or_default(), path),
            family: Some(BrowserFamily::Chromium),
            private_flag: default_private_flag_for_family(BrowserFamily::Chromium),
        });
    }

    Ok(BrowserRecognitionResult {
        recognition: BrowserRecognition::UnverifiedManual,
        name: preferred_browser_name(display_name_hint.unwrap_or_default(), path),
        family: None,
        private_flag: None,
    })
}

fn find_known_browser_definition_by_display_name(
    name: &str,
) -> Option<&'static KnownBrowserDefinition> {
    known_browser_definitions()
        .iter()
        .find(|definition| definition.display_name.eq_ignore_ascii_case(name))
}

fn is_known_browser_match(path: &str, display_name_hint: Option<&str>) -> bool {
    let normalized_path = normalize_path(path);
    let exe_name = executable_name_from_path(path)
        .unwrap_or_else(|| "browser".to_string())
        .to_lowercase();

    if known_browser_definitions().iter().any(|definition| {
        definition
            .known_install_path_suffixes
            .iter()
            .any(|suffix| normalized_path.ends_with(&normalize_path(suffix)))
    }) {
        return true;
    }

    if display_name_hint
        .and_then(find_known_browser_definition_by_display_name)
        .is_some()
    {
        return true;
    }

    known_browser_definitions()
        .iter()
        .any(|definition| definition.executable_aliases.contains(&exe_name.as_str()))
}

fn validate_private_flag(
    browser: &BrowserConfig,
    recognition: &BrowserRecognitionResult,
) -> Result<Option<String>, String> {
    validate_private_flag_value(
        browser.private_flag.as_deref(),
        recognition,
        &browser.name,
        browser.manual_trust,
    )
}

fn validate_private_flag_for_request(
    request: &ManualBrowserValidationRequest,
    recognition: &BrowserRecognitionResult,
) -> Result<Option<String>, String> {
    validate_private_flag_value(
        request.private_flag.as_deref(),
        recognition,
        request.name.as_str(),
        if request.allow_user_confirmed {
            Some(ManualBrowserTrust::UserConfirmed)
        } else {
            None
        },
    )
}

fn validate_private_flag_value(
    private_flag: Option<&str>,
    recognition: &BrowserRecognitionResult,
    browser_name: &str,
    manual_trust: Option<ManualBrowserTrust>,
) -> Result<Option<String>, String> {
    let trimmed = private_flag.map(str::trim).filter(|flag| !flag.is_empty());
    match recognition.recognition {
        BrowserRecognition::Known | BrowserRecognition::RecognizedFamily => match (
            trimmed,
            recognition.private_flag.as_deref(),
        ) {
            (Some(flag), Some(expected)) if flag != expected => Err(format!(
                "Browser '{}' only allows the private flag '{}'.",
                browser_name, expected
            )),
            (_, Some(expected)) => Ok(Some(expected.to_string())),
            (Some(_), None) => Err(format!(
                "Browser '{}' does not support a configured private flag.",
                browser_name
            )),
            (None, None) => Ok(None),
        },
        BrowserRecognition::UnverifiedManual => {
            if trimmed.is_some() {
                return Err(format!(
                    "Unverified manual browser '{}' cannot use a private flag.",
                    browser_name
                ));
            }

            if manual_trust == Some(ManualBrowserTrust::UserConfirmed) {
                Ok(None)
            } else {
                Ok(None)
            }
        }
    }
}

fn preferred_browser_name(display_name_hint: &str, path: &str) -> String {
    let trimmed = display_name_hint.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    executable_name_from_path(path).unwrap_or_else(|| "Browser".to_string())
}

fn chromium_family_tokens() -> &'static [&'static str] {
    &[
        "chromium",
        "chrome",
        "brave",
        "vivaldi",
        "arc",
        "helium",
    ]
}

fn firefox_family_tokens() -> &'static [&'static str] {
    &[
        "firefox",
        "librewolf",
        "waterfox",
        "floorp",
        "zen",
        "tor browser",
    ]
}

fn edge_family_tokens() -> &'static [&'static str] {
    &["msedge", "microsoft edge", "\\edge\\"]
}

fn opera_family_tokens() -> &'static [&'static str] {
    &["opera", "\\opera\\"]
}

fn browser_family_key(family: BrowserFamily) -> Option<&'static str> {
    match family {
        BrowserFamily::Chromium => Some("chromium"),
        BrowserFamily::Firefox => Some("firefox"),
        BrowserFamily::Edge => Some("edge"),
        BrowserFamily::Opera => Some("opera"),
        BrowserFamily::Unknown => None,
    }
}

fn browser_family_label(family: Option<BrowserFamily>) -> &'static str {
    match family {
        Some(BrowserFamily::Chromium) => "Chromium",
        Some(BrowserFamily::Firefox) => "Firefox",
        Some(BrowserFamily::Edge) => "Edge",
        Some(BrowserFamily::Opera) => "Opera",
        _ => "supported",
    }
}

fn resolved_private_flag_for_definition(definition: &KnownBrowserDefinition) -> Option<String> {
    definition
        .private_flag_override
        .map(str::to_string)
        .or_else(|| {
            if definition.use_family_private_flag {
                default_private_flag_for_family(definition.family)
            } else {
                None
            }
        })
}

fn default_private_flag_for_family(family: BrowserFamily) -> Option<String> {
    match family {
        BrowserFamily::Chromium => Some("--incognito".to_string()),
        BrowserFamily::Firefox => Some("--private-window".to_string()),
        BrowserFamily::Edge => Some("--inprivate".to_string()),
        BrowserFamily::Opera => Some("--private".to_string()),
        BrowserFamily::Unknown => None,
    }
}

fn known_browser_definitions() -> &'static [KnownBrowserDefinition] {
    const DEFINITIONS: &[KnownBrowserDefinition] = &[
        KnownBrowserDefinition {
            executable_aliases: &["chrome", "chromium", "chrome_proxy"],
            display_name: "Google Chrome",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Google\\Chrome\\Application\\chrome.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &[],
            display_name: "Tor Browser",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: false,
            known_install_path_suffixes: &["Tor Browser\\Browser\\firefox.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["firefox"],
            display_name: "Mozilla Firefox",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Mozilla Firefox\\firefox.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["librewolf"],
            display_name: "LibreWolf",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["LibreWolf\\librewolf.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["waterfox"],
            display_name: "Waterfox",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Waterfox\\waterfox.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["floorp"],
            display_name: "Floorp",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Floorp\\floorp.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["zen"],
            display_name: "Zen",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Zen Browser\\zen.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["msedge"],
            display_name: "Microsoft Edge",
            family: BrowserFamily::Edge,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Microsoft\\Edge\\Application\\msedge.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["brave", "bravebrowser", "brave-browser"],
            display_name: "Brave",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["BraveSoftware\\Brave-Browser\\Application\\brave.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["opera", "launcher"],
            display_name: "Opera",
            family: BrowserFamily::Opera,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Programs\\Opera\\opera.exe", "Opera\\launcher.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["vivaldi"],
            display_name: "Vivaldi",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Vivaldi\\Application\\vivaldi.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["arc"],
            display_name: "Arc",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["The Browser Company\\Arc\\Arc.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["helium"],
            display_name: "Helium",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["imput\\Helium\\Application\\chrome.exe"],
        },
    ];

    DEFINITIONS
}

#[cfg(target_os = "windows")]
fn known_install_paths(definition: &KnownBrowserDefinition) -> Vec<PathBuf> {
    let program_files =
        std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
    let program_files_x86 = std::env::var("ProgramFiles(x86)")
        .unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let roots = [
        PathBuf::from(&program_files),
        PathBuf::from(&program_files_x86),
        PathBuf::from(&local_app_data),
    ];

    roots
        .iter()
        .flat_map(|root| {
            definition
                .known_install_path_suffixes
                .iter()
                .map(|suffix| root.join(suffix))
        })
        .collect()
}

fn stable_id(prefix: &str, key: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.to_lowercase().hash(&mut hasher);
    format!("{prefix}-{:016x}", hasher.finish())
}

fn normalize_path(path: &str) -> String {
    path.replace('/', "\\").to_lowercase()
}

fn executable_name_from_path(path: &str) -> Option<String> {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
}

#[cfg(target_os = "windows")]
pub(crate) fn running_processes() -> Result<HashSet<String>, String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(
            "Could not create process snapshot while checking running browsers.".to_string(),
        );
    }

    let mut paths = HashSet::new();
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..unsafe { std::mem::zeroed() }
    };

    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) != 0 };
    while has_entry {
        if let Some(path) = query_process_image_path(entry.th32ProcessID) {
            paths.insert(normalize_path(&path));
        }

        has_entry = unsafe { Process32NextW(snapshot, &mut entry) != 0 };
    }

    unsafe {
        CloseHandle(snapshot);
    }

    Ok(paths)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn running_processes() -> Result<HashSet<String>, String> {
    let mut command = Command::new("tasklist");
    command.stdin(Stdio::null()).stderr(Stdio::null());

    let output = command
        .args(["/FO", "CSV", "/NH"])
        .output()
        .map_err(|error| format!("Could not run tasklist to detect running browsers: {error}"))?;

    if !output.status.success() {
        return Err("tasklist failed while checking running browsers.".to_string());
    }

    let mut names = HashSet::new();
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if let Some(name) = parse_first_csv_column(line) {
            names.insert(name.to_lowercase());
        }
    }

    Ok(names)
}

#[cfg(target_os = "windows")]
fn query_process_image_path(process_id: u32) -> Option<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if handle.is_null() {
        return None;
    }

    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    let success =
        unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut length) != 0 };

    unsafe {
        CloseHandle(handle);
    }

    if !success || length == 0 {
        return None;
    }

    String::from_utf16(&buffer[..length as usize]).ok()
}

#[cfg(not(target_os = "windows"))]
fn parse_first_csv_column(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(stripped) = trimmed.strip_prefix('"') {
        let end = stripped.find('"')?;
        return Some(stripped[..end].to_string());
    }

    trimmed
        .split(',')
        .next()
        .map(|value| value.trim_matches('"').to_string())
}

pub(crate) fn is_browser_running(
    browser: &BrowserConfig,
    running_processes: &HashSet<String>,
) -> bool {
    let normalized_path = normalize_path(&browser.path);
    running_processes.contains(&normalized_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppConfig, BrowserConfig, BrowserSource, ThemePreference};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[cfg(target_os = "windows")]
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

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

    fn temp_executable_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hops-browser-test-{unique}"));
        fs::create_dir_all(&dir).expect("temp browser test dir should create");
        let path = dir.join(name);
        fs::write(&path, []).expect("temp executable placeholder should write");
        path
    }

    #[test]
    fn chromium_family_metadata_gets_incognito_flag() {
        for browser in [
            (
                "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
                "Google Chrome",
            ),
            (
                "C:\\Users\\Hugo\\AppData\\Local\\Vivaldi\\Application\\vivaldi.exe",
                "Vivaldi",
            ),
            (
                "C:\\Users\\Hugo\\AppData\\Local\\BraveSoftware\\Brave-Browser\\Application\\brave.exe",
                "Brave",
            ),
            ("C:\\Tools\\Chromium\\chromium.exe", "Google Chrome"),
            (
                "C:\\Program Files\\imput\\Helium\\Application\\chrome.exe",
                "Helium",
            ),
        ] {
            let metadata = resolve_browser_metadata(browser.0, Some(browser.1), None);
            assert_eq!(metadata.family, BrowserFamily::Chromium);
            assert_eq!(metadata.private_flag.as_deref(), Some("--incognito"));
        }
    }

    #[test]
    fn firefox_family_metadata_gets_private_window_flag() {
        for browser in [
            (
                "C:\\Program Files\\Mozilla Firefox\\firefox.exe",
                "Mozilla Firefox",
            ),
            ("C:\\Program Files\\LibreWolf\\librewolf.exe", "LibreWolf"),
            ("C:\\Program Files\\Waterfox\\waterfox.exe", "Waterfox"),
            ("C:\\Program Files\\Floorp\\floorp.exe", "Floorp"),
            ("C:\\Program Files\\Zen Browser\\zen.exe", "Zen"),
        ] {
            let metadata = resolve_browser_metadata(browser.0, Some(browser.1), None);
            assert_eq!(metadata.family, BrowserFamily::Firefox);
            assert_eq!(metadata.private_flag.as_deref(), Some("--private-window"));
        }
    }

    #[test]
    fn edge_and_opera_metadata_get_expected_private_flags() {
        let edge = resolve_browser_metadata(
            "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
            Some("Microsoft Edge"),
            None,
        );
        assert_eq!(edge.family, BrowserFamily::Edge);
        assert_eq!(edge.private_flag.as_deref(), Some("--inprivate"));

        let opera = resolve_browser_metadata(
            "C:\\Users\\Hugo\\AppData\\Local\\Programs\\Opera\\opera.exe",
            Some("Opera"),
            None,
        );
        assert_eq!(opera.family, BrowserFamily::Opera);
        assert_eq!(opera.private_flag.as_deref(), Some("--private"));
    }

    #[test]
    fn unknown_browser_uses_registry_name_and_has_no_private_flag() {
        let metadata =
            resolve_browser_metadata("C:\\Tools\\Arc\\arc.exe", None, Some("Arc_Browser"));
        assert_eq!(metadata.family, BrowserFamily::Chromium);
        assert_eq!(metadata.name, "Arc");
        assert_eq!(metadata.private_flag.as_deref(), Some("--incognito"));
    }

    #[test]
    fn manual_browser_suppresses_detected_browser_with_same_path() {
        let path = "C:\\Program Files\\LibreWolf\\librewolf.exe";
        let mut config = test_app_config(vec![BrowserConfig {
            id: "manual-librewolf".to_string(),
            name: "LibreWolf Custom".to_string(),
            path: path.to_string(),
            private_flag: Some("--my-private".to_string()),
            manual_trust: Some(ManualBrowserTrust::Verified),
            source: BrowserSource::Manual,
            is_hidden: false,
        }]);

        merge_detected_browsers(
            &mut config,
            vec![build_detected_browser(path, Some("LibreWolf"), None)],
        );

        assert_eq!(config.browsers.len(), 1);
        assert_eq!(config.browsers[0].id, "manual-librewolf");
        assert_eq!(
            config.browsers[0].private_flag.as_deref(),
            Some("--my-private")
        );
    }

    #[test]
    fn hidden_detected_browser_stays_hidden_after_refresh() {
        let path = "C:\\Program Files\\LibreWolf\\librewolf.exe";
        let original_detected = build_detected_browser(path, Some("LibreWolf"), None);
        let mut hidden_detected = original_detected.clone();
        hidden_detected.is_hidden = true;
        let mut config = test_app_config(vec![hidden_detected]);

        merge_detected_browsers(
            &mut config,
            vec![build_detected_browser(path, Some("LibreWolf"), None)],
        );

        assert_eq!(config.browsers.len(), 1);
        assert_eq!(config.browsers[0].id, original_detected.id);
        assert!(config.browsers[0].is_hidden);
    }

    #[test]
    fn librewolf_and_vivaldi_detected_browsers_keep_expected_flags() {
        let librewolf = build_detected_browser(
            "C:\\Program Files\\LibreWolf\\librewolf.exe",
            Some("LibreWolf"),
            None,
        );
        assert_eq!(librewolf.private_flag.as_deref(), Some("--private-window"));

        let vivaldi = build_detected_browser(
            "C:\\Users\\Hugo\\AppData\\Local\\Vivaldi\\Application\\vivaldi.exe",
            Some("Vivaldi"),
            None,
        );
        assert_eq!(vivaldi.private_flag.as_deref(), Some("--incognito"));
    }

    #[test]
    fn stale_detected_firefox_family_entries_get_private_flag_backfilled() {
        let mut config = test_app_config(vec![
            BrowserConfig {
                id: "detected-zen".to_string(),
                name: "Firefox-F0DC299D809B9700".to_string(),
                path: "C:\\Program Files\\Zen Browser\\zen.exe".to_string(),
                private_flag: None,
                manual_trust: None,
                source: BrowserSource::Detected,
                is_hidden: false,
            },
            BrowserConfig {
                id: "detected-librewolf".to_string(),
                name: "LibreWolf".to_string(),
                path: "C:\\Program Files\\LibreWolf\\librewolf.exe".to_string(),
                private_flag: None,
                manual_trust: None,
                source: BrowserSource::Detected,
                is_hidden: false,
            },
        ]);

        hydrate_detected_browser_defaults(&mut config);

        assert_eq!(
            config.browsers[0].private_flag.as_deref(),
            Some("--private-window")
        );
        assert_eq!(
            config.browsers[1].private_flag.as_deref(),
            Some("--private-window")
        );
    }

    #[test]
    fn manual_browser_validation_accepts_known_browser_path_and_expected_flag() {
        let path = temp_executable_path("chrome.exe");
        let browser = BrowserConfig {
            id: "manual-chrome".to_string(),
            name: "Portable Chrome".to_string(),
            path: path.to_string_lossy().to_string(),
            private_flag: Some("--incognito".to_string()),
            manual_trust: Some(ManualBrowserTrust::Verified),
            source: BrowserSource::Manual,
            is_hidden: false,
        };

        let validated =
            validate_browser_for_launch(&browser).expect("manual chrome should validate");
        assert_eq!(validated.private_flag.as_deref(), Some("--incognito"));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(path.parent().expect("temp path should have parent"));
    }

    #[test]
    fn manual_browser_validation_requires_confirmation_for_unknown_executable() {
        let path = temp_executable_path("notepad.exe");
        let browser = BrowserConfig {
            id: "manual-notepad".to_string(),
            name: "Definitely Not A Browser".to_string(),
            path: path.to_string_lossy().to_string(),
            private_flag: None,
            manual_trust: None,
            source: BrowserSource::Manual,
            is_hidden: false,
        };

        let error =
            validate_browser_for_launch(&browser).expect_err("unknown exe should need trust");
        assert!(error.contains("needs explicit confirmation"));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(path.parent().expect("temp path should have parent"));
    }

    #[test]
    fn manual_browser_validation_rejects_unexpected_private_flag() {
        let path = temp_executable_path("firefox.exe");
        let browser = BrowserConfig {
            id: "manual-firefox".to_string(),
            name: "Firefox".to_string(),
            path: path.to_string_lossy().to_string(),
            private_flag: Some("--profile".to_string()),
            manual_trust: Some(ManualBrowserTrust::Verified),
            source: BrowserSource::Manual,
            is_hidden: false,
        };

        let error =
            validate_browser_for_launch(&browser).expect_err("unexpected flag should reject");
        assert!(error.contains("only allows the private flag"));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(path.parent().expect("temp path should have parent"));
    }

    #[test]
    fn tor_browser_metadata_has_no_private_flag() {
        let metadata = resolve_browser_metadata(
            "C:\\Users\\Hugo\\Desktop\\Tor Browser\\Browser\\firefox.exe",
            Some("Tor Browser"),
            None,
        );
        assert_eq!(metadata.family, BrowserFamily::Firefox);
        assert_eq!(metadata.name, "Tor Browser");
        assert_eq!(metadata.private_flag, None);
    }

    #[test]
    fn recognized_firefox_family_manual_browser_gets_safe_default_flag() {
        let path = temp_executable_path("floorp.exe");
        let request = ManualBrowserValidationRequest {
            name: "Portable Floorp".to_string(),
            path: path.to_string_lossy().to_string(),
            private_flag: None,
            allow_user_confirmed: false,
        };

        let validated =
            validate_manual_browser_request(&request).expect("recognized family should validate");
        assert_eq!(validated.recognition, BrowserRecognition::Known);
        assert_eq!(validated.manual_trust, Some(ManualBrowserTrust::Verified));
        assert_eq!(validated.private_flag.as_deref(), Some("--private-window"));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(path.parent().expect("temp path should have parent"));
    }

    #[test]
    fn unknown_manual_browser_can_be_user_confirmed() {
        let path = temp_executable_path("custom-browser.exe");
        let request = ManualBrowserValidationRequest {
            name: "Custom Browser".to_string(),
            path: path.to_string_lossy().to_string(),
            private_flag: None,
            allow_user_confirmed: true,
        };

        let validated =
            validate_manual_browser_request(&request).expect("user-confirmed manual browser");
        assert_eq!(
            validated.manual_trust,
            Some(ManualBrowserTrust::UserConfirmed)
        );
        assert!(!validated.requires_confirmation);

        let browser = BrowserConfig {
            id: "manual-custom".to_string(),
            name: "Custom Browser".to_string(),
            path: path.to_string_lossy().to_string(),
            private_flag: None,
            manual_trust: Some(ManualBrowserTrust::UserConfirmed),
            source: BrowserSource::Manual,
            is_hidden: false,
        };
        assert!(validate_browser_for_launch(&browser).is_ok());

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(path.parent().expect("temp path should have parent"));
    }

    #[test]
    fn normalize_manual_browser_entries_clears_verified_trust_for_unknown_paths() {
        let path = temp_executable_path("custom-browser.exe");
        let mut config = test_app_config(vec![BrowserConfig {
            id: "manual-custom".to_string(),
            name: "Custom Browser".to_string(),
            path: path.to_string_lossy().to_string(),
            private_flag: Some("--incognito".to_string()),
            manual_trust: Some(ManualBrowserTrust::Verified),
            source: BrowserSource::Manual,
            is_hidden: false,
        }]);

        normalize_manual_browser_entries(&mut config);
        let browser = &config.browsers[0];
        assert_eq!(browser.manual_trust, None);
        assert_eq!(browser.private_flag, None);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(path.parent().expect("temp path should have parent"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn registry_browser_roots_cover_hkcu_and_hklm() {
        assert_eq!(
            registry_browser_roots(),
            &[
                (HKEY_CURRENT_USER, r"SOFTWARE\Clients\StartMenuInternet"),
                (HKEY_LOCAL_MACHINE, r"SOFTWARE\Clients\StartMenuInternet"),
                (
                    HKEY_LOCAL_MACHINE,
                    r"SOFTWARE\WOW6432Node\Clients\StartMenuInternet"
                ),
            ]
        );
    }
}
