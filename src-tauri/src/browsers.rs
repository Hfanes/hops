use crate::models::{
    AppConfig, BrowserConfig, BrowserRecognition, BrowserSource, ManualBrowserTrust,
    ManualBrowserValidationRequest, ManualBrowserValidationResult,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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
    icon_key: &'static str,
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
    pub(crate) icon_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserRecognitionResult {
    pub(crate) recognition: BrowserRecognition,
    pub(crate) name: String,
    pub(crate) family: Option<BrowserFamily>,
    pub(crate) private_flag: Option<String>,
    pub(crate) icon_key: Option<String>,
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

pub(crate) fn hydrate_browser_icon_keys(config: &mut AppConfig) {
    for browser in &mut config.browsers {
        if browser.icon_key.is_some() {
            continue;
        }

        let resolved = resolve_browser_metadata(&browser.path, Some(browser.name.as_str()), None);
        browser.icon_key = resolved.icon_key.or_else(|| Some("generic".to_string()));
    }
}

pub(crate) fn normalize_manual_browser_entries(config: &mut AppConfig) {
    for browser in &mut config.browsers {
        if browser.source != BrowserSource::Manual {
            continue;
        }

        let Ok(recognition) = classify_browser_path(&browser.path, Some(browser.name.as_str()))
        else {
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
    // Match existing detected entries by path so their IDs (which rules and the
    // default browser reference) survive refreshes even if ID generation changes.
    let existing_detected_by_path: HashMap<String, (String, bool)> = config
        .browsers
        .iter()
        .filter(|browser| browser.source == BrowserSource::Detected)
        .map(|browser| {
            (
                normalize_path(&browser.path),
                (browser.id.clone(), browser.is_hidden),
            )
        })
        .collect();
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
        let normalized = normalize_path(&browser.path);
        if manual_paths.contains(&normalized) {
            continue;
        }

        if let Some((existing_id, is_hidden)) = existing_detected_by_path.get(&normalized) {
            browser.id = existing_id.clone();
            browser.is_hidden = *is_hidden;
        } else {
            browser.is_hidden = hidden_by_detected_id
                .get(&browser.id)
                .copied()
                .unwrap_or(false);
        }
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
        icon_key: metadata.icon_key,
        manual_trust: None,
        source: BrowserSource::Detected,
        is_hidden: false,
    }
}

const HOPS_SELF_LAUNCH_ERROR: &str =
    "Hops can't open links in itself. Pick a real browser instead.";

/// Whether `path` points to Hops, which must never be used as a browser:
/// Hops would hand each link back to itself and keep relaunching forever.
///
/// Matches the running executable's path, and also any `hops.exe`, so another
/// copy of Hops (e.g. the installed app while a dev build runs) is caught too,
/// since the two would just pass links back and forth.
fn is_hops_executable(path: &str) -> bool {
    let normalized = normalize_path(path.trim());
    if normalized.rsplit('\\').next() == Some("hops.exe") {
        return true;
    }

    std::env::current_exe()
        .is_ok_and(|current| normalize_path(&current.to_string_lossy()) == normalized)
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

    if is_hops_executable(path) {
        return Err(HOPS_SELF_LAUNCH_ERROR.to_string());
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

    if is_hops_executable(path) {
        return Err(HOPS_SELF_LAUNCH_ERROR.to_string());
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
        icon_key: recognition.icon_key.or_else(|| Some("generic".to_string())),
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
            icon_key: Some(definition.icon_key.to_string()),
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
            icon_key: Some(definition.icon_key.to_string()),
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
            icon_key: Some(definition.icon_key.to_string()),
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
        icon_key: None,
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
            icon_key: metadata.icon_key,
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
            icon_key: Some("tor".to_string()),
        });
    }

    if firefox_family_tokens()
        .iter()
        .any(|token| haystack.contains(token))
    {
        return Ok(BrowserRecognitionResult {
            recognition: BrowserRecognition::RecognizedFamily,
            name: preferred_browser_name(display_name_hint.unwrap_or_default(), path),
            family: Some(BrowserFamily::Firefox),
            private_flag: default_private_flag_for_family(BrowserFamily::Firefox),
            icon_key: browser_family_key(BrowserFamily::Firefox).map(str::to_string),
        });
    }

    if edge_family_tokens()
        .iter()
        .any(|token| haystack.contains(token))
    {
        return Ok(BrowserRecognitionResult {
            recognition: BrowserRecognition::RecognizedFamily,
            name: preferred_browser_name(display_name_hint.unwrap_or_default(), path),
            family: Some(BrowserFamily::Edge),
            private_flag: default_private_flag_for_family(BrowserFamily::Edge),
            icon_key: browser_family_key(BrowserFamily::Edge).map(str::to_string),
        });
    }

    if opera_family_tokens()
        .iter()
        .any(|token| haystack.contains(token))
    {
        return Ok(BrowserRecognitionResult {
            recognition: BrowserRecognition::RecognizedFamily,
            name: preferred_browser_name(display_name_hint.unwrap_or_default(), path),
            family: Some(BrowserFamily::Opera),
            private_flag: default_private_flag_for_family(BrowserFamily::Opera),
            icon_key: browser_family_key(BrowserFamily::Opera).map(str::to_string),
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
            icon_key: browser_family_key(BrowserFamily::Chromium).map(str::to_string),
        });
    }

    Ok(BrowserRecognitionResult {
        recognition: BrowserRecognition::UnverifiedManual,
        name: preferred_browser_name(display_name_hint.unwrap_or_default(), path),
        family: None,
        private_flag: None,
        icon_key: Some("generic".to_string()),
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
        BrowserRecognition::Known | BrowserRecognition::RecognizedFamily => {
            match (trimmed, recognition.private_flag.as_deref()) {
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
            }
        }
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
    &["chromium", "chrome", "brave", "vivaldi", "arc", "helium"]
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
        BrowserFamily::Chromium => Some("chrome"),
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
            icon_key: "chrome",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Google\\Chrome\\Application\\chrome.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &[],
            display_name: "Tor Browser",
            icon_key: "tor",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: false,
            known_install_path_suffixes: &["Tor Browser\\Browser\\firefox.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["firefox"],
            display_name: "Mozilla Firefox",
            icon_key: "firefox",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Mozilla Firefox\\firefox.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["librewolf"],
            display_name: "LibreWolf",
            icon_key: "librewolf",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["LibreWolf\\librewolf.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["waterfox"],
            display_name: "Waterfox",
            icon_key: "generic",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Waterfox\\waterfox.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["floorp"],
            display_name: "Floorp",
            icon_key: "floorp",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Floorp\\floorp.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["zen"],
            display_name: "Zen",
            icon_key: "zen",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Zen Browser\\zen.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["msedge"],
            display_name: "Microsoft Edge",
            icon_key: "edge",
            family: BrowserFamily::Edge,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Microsoft\\Edge\\Application\\msedge.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["brave", "bravebrowser", "brave-browser"],
            display_name: "Brave",
            icon_key: "brave",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["BraveSoftware\\Brave-Browser\\Application\\brave.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["opera", "launcher"],
            display_name: "Opera",
            icon_key: "opera",
            family: BrowserFamily::Opera,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Programs\\Opera\\opera.exe", "Opera\\launcher.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["vivaldi"],
            display_name: "Vivaldi",
            icon_key: "vivaldi",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["Vivaldi\\Application\\vivaldi.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["arc"],
            display_name: "Arc",
            icon_key: "arc",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            use_family_private_flag: true,
            known_install_path_suffixes: &["The Browser Company\\Arc\\Arc.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["helium"],
            display_name: "Helium",
            icon_key: "helium",
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

/// Builds an ID from a 64-bit FNV-1a hash of the lowercased key.
///
/// FNV-1a is fixed by specification, unlike `DefaultHasher`, whose output may
/// change between Rust releases and would silently orphan persisted rules.
fn stable_id(prefix: &str, key: &str) -> String {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let hash = key
        .to_lowercase()
        .bytes()
        .fold(FNV_OFFSET_BASIS, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
        });
    format!("{prefix}-{hash:016x}")
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

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
/// Lowercased executable file names (e.g. `chrome.exe`) of the given browsers.
fn browser_executable_names(browsers: &[BrowserConfig]) -> HashSet<String> {
    browsers
        .iter()
        .filter_map(|browser| {
            normalize_path(browser.path.trim())
                .rsplit('\\')
                .next()
                .filter(|name| !name.is_empty())
                .map(str::to_string)
        })
        .collect()
}

/// Normalized full paths of running processes that could be one of `browsers`.
///
/// The process snapshot already carries each executable's file name, so only
/// processes whose name matches a configured browser are opened to read their
/// full path. That turns hundreds of `OpenProcess` calls into a handful.
#[cfg(target_os = "windows")]
pub(crate) fn running_processes(browsers: &[BrowserConfig]) -> Result<HashSet<String>, String> {
    let wanted_names = browser_executable_names(browsers);
    if wanted_names.is_empty() {
        return Ok(HashSet::new());
    }

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(
            "Could not create process snapshot while checking running browsers.".to_string(),
        );
    }

    let mut paths = HashSet::new();
    let mut path_buffer = vec![0u16; 32768];
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..unsafe { std::mem::zeroed() }
    };

    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) != 0 };
    while has_entry {
        if process_entry_name_matches(&entry.szExeFile, &wanted_names) {
            if let Some(path) = query_process_image_path(entry.th32ProcessID, &mut path_buffer) {
                paths.insert(normalize_path(&path));
            }
        }

        has_entry = unsafe { Process32NextW(snapshot, &mut entry) != 0 };
    }

    unsafe {
        CloseHandle(snapshot);
    }

    Ok(paths)
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
/// Whether a null-terminated UTF-16 executable name is in `wanted_names`
/// (which holds lowercased names).
fn process_entry_name_matches(exe_file: &[u16], wanted_names: &HashSet<String>) -> bool {
    let length = exe_file
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(exe_file.len());
    String::from_utf16(&exe_file[..length])
        .is_ok_and(|name| wanted_names.contains(&name.to_lowercase()))
}

/// Hops only supports Windows; elsewhere no browser is reported as running.
#[cfg(not(target_os = "windows"))]
pub(crate) fn running_processes(_browsers: &[BrowserConfig]) -> Result<HashSet<String>, String> {
    Ok(HashSet::new())
}

#[cfg(target_os = "windows")]
fn query_process_image_path(process_id: u32, buffer: &mut [u16]) -> Option<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if handle.is_null() {
        return None;
    }

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

pub(crate) fn is_browser_running(
    browser: &BrowserConfig,
    running_processes: &HashSet<String>,
) -> bool {
    let normalized_path = normalize_path(&browser.path);
    running_processes.contains(&normalized_path)
}

pub(crate) struct RunningProcessSnapshot<F>
where
    F: FnMut() -> HashSet<String>,
{
    running: Option<HashSet<String>>,
    load: F,
}

/// A lazy snapshot that scans only for the given browsers' executables, and
/// only the first time a running state is actually needed.
pub(crate) fn current_running_processes(
    browsers: &[BrowserConfig],
) -> RunningProcessSnapshot<impl FnMut() -> HashSet<String> + '_> {
    RunningProcessSnapshot::new(move || running_processes(browsers).unwrap_or_default())
}

impl<F> RunningProcessSnapshot<F>
where
    F: FnMut() -> HashSet<String>,
{
    pub(crate) fn new(load: F) -> Self {
        Self {
            running: None,
            load,
        }
    }

    pub(crate) fn is_browser_running(&mut self, browser: &BrowserConfig) -> bool {
        let running = self.running.get_or_insert_with(|| (self.load)());
        is_browser_running(browser, running)
    }
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
    fn known_browser_definitions_have_icon_keys() {
        for definition in known_browser_definitions() {
            assert!(
                !definition.icon_key.trim().is_empty(),
                "{} should have an icon key",
                definition.display_name
            );
        }
    }

    #[test]
    fn known_browser_metadata_gets_specific_icon_keys() {
        for (path, name, icon_key) in [
            (
                "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
                "Google Chrome",
                "chrome",
            ),
            (
                "C:\\Users\\Hugo\\Desktop\\Tor Browser\\Browser\\firefox.exe",
                "Tor Browser",
                "tor",
            ),
            (
                "C:\\Program Files\\Mozilla Firefox\\firefox.exe",
                "Mozilla Firefox",
                "firefox",
            ),
            ("C:\\Program Files\\LibreWolf\\librewolf.exe", "LibreWolf", "librewolf"),
            ("C:\\Program Files\\Waterfox\\waterfox.exe", "Waterfox", "generic"),
            ("C:\\Program Files\\Floorp\\floorp.exe", "Floorp", "floorp"),
            ("C:\\Program Files\\Zen Browser\\zen.exe", "Zen", "zen"),
            (
                "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
                "Microsoft Edge",
                "edge",
            ),
            (
                "C:\\Users\\Hugo\\AppData\\Local\\BraveSoftware\\Brave-Browser\\Application\\brave.exe",
                "Brave",
                "brave",
            ),
            (
                "C:\\Users\\Hugo\\AppData\\Local\\Programs\\Opera\\opera.exe",
                "Opera",
                "opera",
            ),
            (
                "C:\\Users\\Hugo\\AppData\\Local\\Vivaldi\\Application\\vivaldi.exe",
                "Vivaldi",
                "vivaldi",
            ),
            ("C:\\Tools\\Arc\\arc.exe", "Arc", "arc"),
            (
                "C:\\Program Files\\imput\\Helium\\Application\\chrome.exe",
                "Helium",
                "helium",
            ),
        ] {
            let metadata = resolve_browser_metadata(path, Some(name), None);
            assert_eq!(metadata.icon_key.as_deref(), Some(icon_key));
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
            icon_key: Some("librewolf".to_string()),
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
    fn stable_id_is_fixed_fnv1a_and_case_insensitive() {
        // Pinned values: these must never change, or persisted rules lose their browser.
        assert_eq!(stable_id("detected", ""), "detected-cbf29ce484222325");
        assert_eq!(stable_id("detected", "a"), "detected-af63dc4c8601ec8c");
        assert_eq!(
            stable_id("detected", "C:\\Chrome.exe"),
            stable_id("detected", "c:\\chrome.exe")
        );
    }

    #[test]
    fn refresh_keeps_existing_detected_browser_id_by_path() {
        let path = "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe";
        let mut existing = build_detected_browser(path, Some("Google Chrome"), None);
        existing.id = "detected-legacy-id".to_string();
        existing.is_hidden = true;
        let mut config = test_app_config(vec![existing]);

        merge_detected_browsers(
            &mut config,
            vec![build_detected_browser(
                "c:/program files/google/chrome/application/chrome.exe",
                Some("Google Chrome"),
                None,
            )],
        );

        assert_eq!(config.browsers.len(), 1);
        assert_eq!(config.browsers[0].id, "detected-legacy-id");
        assert!(config.browsers[0].is_hidden);
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
                icon_key: None,
                manual_trust: None,
                source: BrowserSource::Detected,
                is_hidden: false,
            },
            BrowserConfig {
                id: "detected-librewolf".to_string(),
                name: "LibreWolf".to_string(),
                path: "C:\\Program Files\\LibreWolf\\librewolf.exe".to_string(),
                private_flag: None,
                icon_key: None,
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
            icon_key: Some("chrome".to_string()),
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
            icon_key: Some("generic".to_string()),
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
            icon_key: Some("firefox".to_string()),
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
        assert_eq!(validated.icon_key.as_deref(), Some("floorp"));

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
        assert_eq!(validated.icon_key.as_deref(), Some("generic"));
        assert!(!validated.requires_confirmation);

        let browser = BrowserConfig {
            id: "manual-custom".to_string(),
            name: "Custom Browser".to_string(),
            path: path.to_string_lossy().to_string(),
            private_flag: None,
            icon_key: Some("generic".to_string()),
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
            icon_key: Some("generic".to_string()),
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

    fn browser_at(path: &str) -> BrowserConfig {
        BrowserConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            path: path.to_string(),
            private_flag: None,
            icon_key: None,
            manual_trust: None,
            source: BrowserSource::Manual,
            is_hidden: false,
        }
    }

    fn utf16_exe_name(name: &str) -> [u16; 260] {
        let mut buffer = [0u16; 260];
        for (slot, unit) in buffer.iter_mut().zip(name.encode_utf16()) {
            *slot = unit;
        }
        buffer
    }

    #[test]
    fn browser_executable_names_are_lowercased_file_names() {
        let names = browser_executable_names(&[
            browser_at("C:\\Program Files\\Google\\Chrome\\Application\\Chrome.EXE"),
            browser_at("  C:/Tools/Firefox/firefox.exe  "),
            browser_at(""),
        ]);

        assert_eq!(
            names,
            HashSet::from(["chrome.exe".to_string(), "firefox.exe".to_string()])
        );
    }

    #[test]
    fn process_entry_name_matching_is_case_insensitive_and_stops_at_null() {
        let wanted = HashSet::from(["chrome.exe".to_string()]);

        assert!(process_entry_name_matches(
            &utf16_exe_name("CHROME.exe"),
            &wanted
        ));
        assert!(!process_entry_name_matches(
            &utf16_exe_name("chrome.exe.bak"),
            &wanted
        ));
        assert!(!process_entry_name_matches(
            &utf16_exe_name("svchost.exe"),
            &wanted
        ));
    }

    #[test]
    fn hops_executable_is_recognized_by_name_or_running_path() {
        assert!(is_hops_executable(
            "C:\\Users\\me\\AppData\\Local\\hops\\hops.exe"
        ));
        assert!(is_hops_executable(
            "  C:/Projects/hops/src-tauri/target/debug/HOPS.EXE "
        ));

        let current_exe = std::env::current_exe().expect("current exe should resolve");
        assert!(is_hops_executable(&current_exe.to_string_lossy()));

        assert!(!is_hops_executable(
            "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
        ));
        assert!(!is_hops_executable("C:\\Games\\hopscotch.exe"));
    }

    #[test]
    fn hops_cannot_be_added_or_launched_as_a_browser() {
        let request = ManualBrowserValidationRequest {
            name: "Hops".to_string(),
            path: "C:\\Users\\me\\AppData\\Local\\hops\\hops.exe".to_string(),
            private_flag: None,
            allow_user_confirmed: true,
        };
        let error = validate_manual_browser_request(&request).expect_err("Hops must be rejected");
        assert_eq!(error, HOPS_SELF_LAUNCH_ERROR);

        // Configs saved before this check existed are refused at launch.
        let mut browser = browser_at("C:\\Users\\me\\AppData\\Local\\hops\\hops.exe");
        browser.manual_trust = Some(ManualBrowserTrust::UserConfirmed);
        let error = validate_browser_for_launch(&browser).expect_err("Hops must not launch");
        assert_eq!(error, HOPS_SELF_LAUNCH_ERROR);
    }

    #[test]
    fn running_processes_skips_scan_without_browsers() {
        assert!(running_processes(&[])
            .expect("empty scan should succeed")
            .is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn running_processes_finds_a_live_process_by_full_path() {
        // The test binary itself is a running process we know the path of.
        let current_exe = std::env::current_exe().expect("current exe should resolve");
        let current_path = current_exe.to_string_lossy().to_string();

        let running = running_processes(&[browser_at(&current_path)])
            .expect("process snapshot should succeed");

        assert!(is_browser_running(&browser_at(&current_path), &running));
        // Only processes sharing the wanted executable name are resolved.
        let wanted = browser_executable_names(&[browser_at(&current_path)]);
        assert!(running.iter().all(|path| wanted
            .iter()
            .any(|name| path.ends_with(&format!("\\{name}")))));
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
