use crate::models::{AppConfig, BrowserConfig, BrowserSource};
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
    known_install_path_suffixes: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedBrowserMetadata {
    pub(crate) name: String,
    pub(crate) family: BrowserFamily,
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
        source: BrowserSource::Detected,
        is_hidden: false,
    }
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
            private_flag: definition
                .private_flag_override
                .map(str::to_string)
                .or_else(|| default_private_flag_for_family(definition.family)),
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
            private_flag: definition
                .private_flag_override
                .map(str::to_string)
                .or_else(|| default_private_flag_for_family(definition.family)),
        };
    }

    if let Some(definition) = known_browser_definitions()
        .iter()
        .find(|definition| definition.executable_aliases.contains(&exe_name.as_str()))
    {
        return ResolvedBrowserMetadata {
            name: definition.display_name.to_string(),
            family: definition.family,
            private_flag: definition
                .private_flag_override
                .map(str::to_string)
                .or_else(|| default_private_flag_for_family(definition.family)),
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

fn find_known_browser_definition_by_display_name(
    name: &str,
) -> Option<&'static KnownBrowserDefinition> {
    known_browser_definitions()
        .iter()
        .find(|definition| definition.display_name.eq_ignore_ascii_case(name))
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
            known_install_path_suffixes: &["Google\\Chrome\\Application\\chrome.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["firefox"],
            display_name: "Mozilla Firefox",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            known_install_path_suffixes: &["Mozilla Firefox\\firefox.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["librewolf"],
            display_name: "LibreWolf",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            known_install_path_suffixes: &["LibreWolf\\librewolf.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["waterfox"],
            display_name: "Waterfox",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            known_install_path_suffixes: &["Waterfox\\waterfox.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["floorp"],
            display_name: "Floorp",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            known_install_path_suffixes: &["Floorp\\floorp.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["zen"],
            display_name: "Zen",
            family: BrowserFamily::Firefox,
            private_flag_override: None,
            known_install_path_suffixes: &["Zen Browser\\zen.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["msedge"],
            display_name: "Microsoft Edge",
            family: BrowserFamily::Edge,
            private_flag_override: None,
            known_install_path_suffixes: &["Microsoft\\Edge\\Application\\msedge.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["brave", "bravebrowser", "brave-browser"],
            display_name: "Brave",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            known_install_path_suffixes: &["BraveSoftware\\Brave-Browser\\Application\\brave.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["opera", "launcher"],
            display_name: "Opera",
            family: BrowserFamily::Opera,
            private_flag_override: None,
            known_install_path_suffixes: &["Programs\\Opera\\opera.exe", "Opera\\launcher.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["vivaldi"],
            display_name: "Vivaldi",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
            known_install_path_suffixes: &["Vivaldi\\Application\\vivaldi.exe"],
        },
        KnownBrowserDefinition {
            executable_aliases: &["helium"],
            display_name: "Helium",
            family: BrowserFamily::Chromium,
            private_flag_override: None,
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
        assert_eq!(metadata.family, BrowserFamily::Unknown);
        assert_eq!(metadata.name, "Arc Browser");
        assert_eq!(metadata.private_flag, None);
    }

    #[test]
    fn manual_browser_suppresses_detected_browser_with_same_path() {
        let path = "C:\\Program Files\\LibreWolf\\librewolf.exe";
        let mut config = test_app_config(vec![BrowserConfig {
            id: "manual-librewolf".to_string(),
            name: "LibreWolf Custom".to_string(),
            path: path.to_string(),
            private_flag: Some("--my-private".to_string()),
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
                source: BrowserSource::Detected,
                is_hidden: false,
            },
            BrowserConfig {
                id: "detected-librewolf".to_string(),
                name: "LibreWolf".to_string(),
                path: "C:\\Program Files\\LibreWolf\\librewolf.exe".to_string(),
                private_flag: None,
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
