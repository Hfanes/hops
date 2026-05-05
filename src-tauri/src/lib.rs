use globset::GlobBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[cfg(target_os = "windows")]
use std::ffi::OsStr;
use std::fs;
use std::hash::{Hash, Hasher};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use url::Url;

#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_WRITE};

const CONFIG_FILENAME: &str = "config.json";
const HOPS_APP_NAME: &str = "Hops";
const HOPS_PROTOCOL_PROG_ID: &str = "HopsURL";
const HOPS_HTML_PROG_ID: &str = "HopsHTML";
const HOPS_CUSTOM_URI_SCHEME: &str = "Hops";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(target_os = "windows")]
const ASSOCF_NONE: u32 = 0;
#[cfg(target_os = "windows")]
const ASSOCSTR_PROGID: u32 = 20;

#[cfg(target_os = "windows")]
#[link(name = "Shlwapi")]
unsafe extern "system" {
    fn AssocQueryStringW(
        flags: u32,
        str: u32,
        psz_assoc: *const u16,
        psz_extra: *const u16,
        psz_out: *mut u16,
        pcch_out: *mut u32,
    ) -> i32;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum BrowserSource {
    Detected,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RulePatternType {
    Hostname,
    HostnameSubdomains,
    Prefix,
    Contains,
    FullUrl,
    Glob,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserConfig {
    id: String,
    name: String,
    path: String,
    private_flag: Option<String>,
    source: BrowserSource,
    is_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuleConfig {
    id: String,
    pattern: String,
    pattern_type: RulePatternType,
    browser_id: String,
    private_mode: bool,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
    version: u32,
    always_show_picker: bool,
    use_defaults_when_not_running: bool,
    disable_transparency: bool,
    #[serde(default)]
    onboarding_completed: bool,
    default_browser_id: Option<String>,
    browsers: Vec<BrowserConfig>,
    rules: Vec<RuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RouteAction {
    OpenBrowser,
    ShowPicker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RouteDecision {
    action: RouteAction,
    reason: String,
    browser_id: Option<String>,
    browser_name: Option<String>,
    private_mode: bool,
    matched_rule_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenUrlRequest {
    browser_id: String,
    url: String,
    private_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserRegistrationStatus {
    registered: bool,
    is_default_http: bool,
    is_default_https: bool,
    is_fully_default: bool,
    current_http_prog_id: Option<String>,
    current_https_prog_id: Option<String>,
}

#[tauri::command]
fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    load_or_init_config(&app, true)
}

#[tauri::command]
fn save_config(app: AppHandle, config: AppConfig) -> Result<AppConfig, String> {
    save_config_internal(&app, config, true)
}

#[tauri::command]
fn refresh_browsers(app: AppHandle) -> Result<AppConfig, String> {
    let mut config = load_or_init_config(&app, false)?;
    merge_detected_browsers(&mut config, detect_browsers());
    save_config_internal(&app, config, false)
}

#[tauri::command]
fn list_running_browser_ids(app: AppHandle) -> Result<Vec<String>, String> {
    let config = load_or_init_config(&app, false)?;
    let running = running_processes()?;

    Ok(config
        .browsers
        .iter()
        .filter(|browser| !browser.is_hidden && is_browser_running(browser, &running))
        .map(|browser| browser.id.clone())
        .collect())
}

#[tauri::command]
fn preview_route(app: AppHandle, url: String) -> Result<RouteDecision, String> {
    let config = load_or_init_config(&app, false)?;
    resolve_route(&config, &url)
}

#[tauri::command]
fn preview_route_with_config(config: AppConfig, url: String) -> Result<RouteDecision, String> {
    let mut normalized = config;
    normalize_config(&mut normalized);
    validate_config(&normalized)?;
    resolve_route(&normalized, &url)
}

#[tauri::command]
fn route_and_open(app: AppHandle, url: String) -> Result<RouteDecision, String> {
    let config = load_or_init_config(&app, false)?;
    let decision = resolve_route(&config, &url)?;

    if decision.action == RouteAction::OpenBrowser {
        if let Some(browser_id) = decision.browser_id.as_deref() {
            open_url_with_browser(&config, browser_id, &url, decision.private_mode)?;
        }
    }

    Ok(decision)
}

#[tauri::command]
fn route_and_open_with_config(config: AppConfig, url: String) -> Result<RouteDecision, String> {
    let mut normalized = config;
    normalize_config(&mut normalized);
    validate_config(&normalized)?;
    let decision = resolve_route(&normalized, &url)?;

    if decision.action == RouteAction::OpenBrowser {
        if let Some(browser_id) = decision.browser_id.as_deref() {
            open_url_with_browser(&normalized, browser_id, &url, decision.private_mode)?;
        }
    }

    Ok(decision)
}

#[tauri::command]
fn open_url(app: AppHandle, request: OpenUrlRequest) -> Result<(), String> {
    let config = load_or_init_config(&app, false)?;
    open_url_with_browser(
        &config,
        &request.browser_id,
        &request.url,
        request.private_mode,
    )
}

#[tauri::command]
fn open_windows_default_apps() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer.exe");
        command.arg("ms-settings:defaultapps");
        command
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| format!("Could not open Windows default apps settings: {error}"))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Opening Windows Default Apps is only supported on Windows.".to_string())
    }
}

#[tauri::command]
fn get_browser_registration_status() -> Result<BrowserRegistrationStatus, String> {
    #[cfg(target_os = "windows")]
    {
        browser_registration_status_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Browser registration status is only available on Windows.".to_string())
    }
}

#[tauri::command]
fn register_hops_as_browser() -> Result<BrowserRegistrationStatus, String> {
    #[cfg(target_os = "windows")]
    {
        register_hops_as_browser_windows()?;
        browser_registration_status_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Registering Hops as a browser is only supported on Windows.".to_string())
    }
}

#[tauri::command]
fn unregister_hops_as_browser() -> Result<BrowserRegistrationStatus, String> {
    #[cfg(target_os = "windows")]
    {
        unregister_hops_as_browser_windows()?;
        browser_registration_status_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Unregistering Hops as a browser is only supported on Windows.".to_string())
    }
}

fn config_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    if let Ok(app_data_roaming) = std::env::var("APPDATA") {
        let app_data = PathBuf::from(app_data_roaming).join("Hops");
        fs::create_dir_all(&app_data).map_err(|error| {
            format!(
                "Could not create app data directory {:?}: {error}",
                app_data
            )
        })?;
        return Ok(app_data.join(CONFIG_FILENAME));
    }

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve app data directory: {error}"))?;

    fs::create_dir_all(&app_data)
        .map_err(|error| format!("Could not create app data directory {:?}: {error}", app_data))?;

    Ok(app_data.join(CONFIG_FILENAME))
}

#[cfg(target_os = "windows")]
fn browser_registration_status_windows() -> Result<BrowserRegistrationStatus, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let registered = hkcu
        .open_subkey("Software\\RegisteredApplications")
        .ok()
        .and_then(|key| key.get_value::<String, _>(HOPS_APP_NAME).ok())
        .is_some_and(|value| value == "Software\\Hops\\Capabilities");

    let current_http_prog_id = query_url_association_prog_id("http")
        .or_else(|| read_url_user_choice_prog_id("http"));
    let current_https_prog_id = query_url_association_prog_id("https")
        .or_else(|| read_url_user_choice_prog_id("https"));

    let is_default_http = current_http_prog_id
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case(HOPS_PROTOCOL_PROG_ID));
    let is_default_https = current_https_prog_id
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case(HOPS_PROTOCOL_PROG_ID));

    Ok(BrowserRegistrationStatus {
        registered,
        is_default_http,
        is_default_https,
        is_fully_default: is_default_http && is_default_https,
        current_http_prog_id,
        current_https_prog_id,
    })
}

#[cfg(target_os = "windows")]
fn query_url_association_prog_id(scheme: &str) -> Option<String> {
    let assoc = wide_null(scheme);
    let mut length: u32 = 0;

    let first_result = unsafe {
        AssocQueryStringW(
            ASSOCF_NONE,
            ASSOCSTR_PROGID,
            assoc.as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            &mut length,
        )
    };

    // S_FALSE means the first call returned the required length.
    if first_result != 1 || length == 0 {
        return None;
    }

    let mut buffer = vec![0u16; length as usize];
    let second_result = unsafe {
        AssocQueryStringW(
            ASSOCF_NONE,
            ASSOCSTR_PROGID,
            assoc.as_ptr(),
            std::ptr::null(),
            buffer.as_mut_ptr(),
            &mut length,
        )
    };

    if second_result != 0 || length == 0 {
        return None;
    }

    let end = buffer.iter().position(|value| *value == 0).unwrap_or(buffer.len());
    String::from_utf16(&buffer[..end]).ok()
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn read_url_user_choice_prog_id(scheme: &str) -> Option<String> {
    let key_path =
        format!("Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\{scheme}\\UserChoice");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(key_path)
        .ok()
        .and_then(|key| key.get_value::<String, _>("ProgId").ok())
}

#[cfg(target_os = "windows")]
fn register_hops_as_browser_windows() -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("Could not resolve executable path: {error}"))?;
    let executable_string = executable.to_string_lossy().to_string();
    let command_with_url = format!("\"{executable_string}\" --url \"%1\"");

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let (hops_url_key, _) = hkcu
        .create_subkey(format!("Software\\Classes\\{HOPS_PROTOCOL_PROG_ID}"))
        .map_err(|error| format!("Could not create Hops URL class key: {error}"))?;
    hops_url_key
        .set_value("", &"Hops URL Handler")
        .map_err(|error| format!("Could not write HopsURL default value: {error}"))?;
    hops_url_key
        .set_value("URL Protocol", &"")
        .map_err(|error| format!("Could not write HopsURL protocol marker: {error}"))?;

    let (hops_url_icon, _) = hkcu
        .create_subkey(format!(
            "Software\\Classes\\{HOPS_PROTOCOL_PROG_ID}\\DefaultIcon"
        ))
        .map_err(|error| format!("Could not create HopsURL icon key: {error}"))?;
    hops_url_icon
        .set_value("", &format!("\"{executable_string}\",0"))
        .map_err(|error| format!("Could not write HopsURL icon value: {error}"))?;

    let (hops_url_command, _) = hkcu
        .create_subkey(format!(
            "Software\\Classes\\{HOPS_PROTOCOL_PROG_ID}\\shell\\open\\command"
        ))
        .map_err(|error| format!("Could not create HopsURL command key: {error}"))?;
    hops_url_command
        .set_value("", &command_with_url)
        .map_err(|error| format!("Could not write HopsURL command value: {error}"))?;

    let (hops_html_key, _) = hkcu
        .create_subkey(format!("Software\\Classes\\{HOPS_HTML_PROG_ID}"))
        .map_err(|error| format!("Could not create Hops HTML class key: {error}"))?;
    hops_html_key
        .set_value("", &"Hops HTML Document")
        .map_err(|error| format!("Could not write HopsHTML default value: {error}"))?;

    let (hops_html_command, _) = hkcu
        .create_subkey(format!(
            "Software\\Classes\\{HOPS_HTML_PROG_ID}\\shell\\open\\command"
        ))
        .map_err(|error| format!("Could not create HopsHTML command key: {error}"))?;
    hops_html_command
        .set_value("", &command_with_url)
        .map_err(|error| format!("Could not write HopsHTML command value: {error}"))?;

    let (hops_custom_scheme_key, _) = hkcu
        .create_subkey(format!("Software\\Classes\\{HOPS_CUSTOM_URI_SCHEME}"))
        .map_err(|error| format!("Could not create Hops custom scheme key: {error}"))?;
    hops_custom_scheme_key
        .set_value("", &"Hops Custom URI")
        .map_err(|error| format!("Could not write Hops custom scheme name: {error}"))?;
    hops_custom_scheme_key
        .set_value("URL Protocol", &"")
        .map_err(|error| format!("Could not write Hops custom scheme marker: {error}"))?;

    let (hops_custom_scheme_command, _) = hkcu
        .create_subkey(format!(
            "Software\\Classes\\{HOPS_CUSTOM_URI_SCHEME}\\shell\\open\\command"
        ))
        .map_err(|error| format!("Could not create Hops custom scheme command key: {error}"))?;
    hops_custom_scheme_command
        .set_value("", &command_with_url)
        .map_err(|error| format!("Could not write Hops custom scheme command: {error}"))?;

    let (capabilities_key, _) = hkcu
        .create_subkey("Software\\Hops\\Capabilities")
        .map_err(|error| format!("Could not create Hops capabilities key: {error}"))?;
    capabilities_key
        .set_value("ApplicationName", &HOPS_APP_NAME)
        .map_err(|error| format!("Could not write ApplicationName: {error}"))?;
    capabilities_key
        .set_value(
            "ApplicationDescription",
            &"Hops routes external links to the right browser.",
        )
        .map_err(|error| format!("Could not write ApplicationDescription: {error}"))?;

    let (url_associations, _) = hkcu
        .create_subkey("Software\\Hops\\Capabilities\\URLAssociations")
        .map_err(|error| format!("Could not create URLAssociations key: {error}"))?;
    url_associations
        .set_value("http", &HOPS_PROTOCOL_PROG_ID)
        .map_err(|error| format!("Could not write HTTP association: {error}"))?;
    url_associations
        .set_value("https", &HOPS_PROTOCOL_PROG_ID)
        .map_err(|error| format!("Could not write HTTPS association: {error}"))?;

    let (file_associations, _) = hkcu
        .create_subkey("Software\\Hops\\Capabilities\\FileAssociations")
        .map_err(|error| format!("Could not create FileAssociations key: {error}"))?;
    file_associations
        .set_value(".htm", &HOPS_HTML_PROG_ID)
        .map_err(|error| format!("Could not write .htm association: {error}"))?;
    file_associations
        .set_value(".html", &HOPS_HTML_PROG_ID)
        .map_err(|error| format!("Could not write .html association: {error}"))?;

    let (registered_apps_key, _) = hkcu
        .create_subkey("Software\\RegisteredApplications")
        .map_err(|error| format!("Could not create RegisteredApplications key: {error}"))?;
    registered_apps_key
        .set_value(HOPS_APP_NAME, &"Software\\Hops\\Capabilities")
        .map_err(|error| format!("Could not register Hops in RegisteredApplications: {error}"))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn unregister_hops_as_browser_windows() -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    if let Ok(registered_apps) = hkcu.open_subkey_with_flags("Software\\RegisteredApplications", KEY_WRITE)
    {
        let _ = registered_apps.delete_value(HOPS_APP_NAME);
    }

    for path in [
        "Software\\Hops",
        &format!("Software\\Classes\\{HOPS_PROTOCOL_PROG_ID}"),
        &format!("Software\\Classes\\{HOPS_HTML_PROG_ID}"),
        &format!("Software\\Classes\\{HOPS_CUSTOM_URI_SCHEME}"),
    ] {
        let _ = hkcu.delete_subkey_all(path);
    }

    Ok(())
}

fn load_or_init_config(app: &AppHandle, auto_populate_browsers: bool) -> Result<AppConfig, String> {
    let path = config_file_path(app)?;

    let mut config = if path.exists() {
        let data = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read config from {:?}: {error}", path))?;
        serde_json::from_str::<AppConfig>(&data)
            .map_err(|error| format!("Config file {:?} is invalid JSON: {error}", path))?
    } else {
        let mut config = default_config();
        merge_detected_browsers(&mut config, detect_browsers());
        write_config_file(&path, &config)?;
        config
    };

    normalize_config(&mut config);

    if auto_populate_browsers && config.browsers.is_empty() {
        merge_detected_browsers(&mut config, detect_browsers());
    }

    Ok(config)
}

fn default_config() -> AppConfig {
    AppConfig {
        version: 1,
        always_show_picker: false,
        use_defaults_when_not_running: false,
        disable_transparency: false,
        onboarding_completed: false,
        default_browser_id: None,
        browsers: Vec::new(),
        rules: Vec::new(),
    }
}

fn normalize_config(config: &mut AppConfig) {
    if config.version == 0 {
        config.version = 1;
    }

    let browser_ids: HashSet<String> = config.browsers.iter().map(|browser| browser.id.clone()).collect();

    if let Some(default_browser_id) = config.default_browser_id.clone() {
        if !browser_ids.contains(&default_browser_id) {
            config.default_browser_id = None;
        }
    }

    config
        .rules
        .retain(|rule| browser_ids.contains(&rule.browser_id));
}

fn validate_config(config: &AppConfig) -> Result<(), String> {
    let browser_ids: HashSet<String> = config.browsers.iter().map(|browser| browser.id.clone()).collect();

    if let Some(default_browser_id) = config.default_browser_id.as_deref() {
        if !browser_ids.contains(default_browser_id) {
            return Err("Default browser must reference an existing browser entry.".to_string());
        }
    }

    for rule in &config.rules {
        if !browser_ids.contains(&rule.browser_id) {
            return Err(format!(
                "Rule '{}' references browser '{}' which does not exist.",
                rule.id, rule.browser_id
            ));
        }

        if rule.pattern_type == RulePatternType::Regex {
            Regex::new(&rule.pattern).map_err(|error| {
                format!(
                    "Rule '{}' has an invalid regex pattern '{}': {error}",
                    rule.id, rule.pattern
                )
            })?;
        }
    }

    Ok(())
}

fn save_config_internal(
    app: &AppHandle,
    mut config: AppConfig,
    validate_before_write: bool,
) -> Result<AppConfig, String> {
    normalize_config(&mut config);

    if validate_before_write {
        validate_config(&config)?;
    }

    let path = config_file_path(app)?;
    write_config_file(&path, &config)?;
    Ok(config)
}

fn write_config_file(path: &Path, config: &AppConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|error| format!("Could not serialize config JSON: {error}"))?;
    fs::write(path, json).map_err(|error| format!("Could not write config to {:?}: {error}", path))
}

fn merge_detected_browsers(config: &mut AppConfig, detected_browsers: Vec<BrowserConfig>) {
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

fn detect_browsers() -> Vec<BrowserConfig> {
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

    let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
    let program_files_x86 =
        std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();

    let known_paths: Vec<(&str, Vec<PathBuf>)> = vec![
        (
            "Google Chrome",
            vec![
                PathBuf::from(&program_files).join("Google\\Chrome\\Application\\chrome.exe"),
                PathBuf::from(&program_files_x86).join("Google\\Chrome\\Application\\chrome.exe"),
                PathBuf::from(&local_app_data).join("Google\\Chrome\\Application\\chrome.exe"),
            ],
        ),
        (
            "Mozilla Firefox",
            vec![
                PathBuf::from(&program_files).join("Mozilla Firefox\\firefox.exe"),
                PathBuf::from(&program_files_x86).join("Mozilla Firefox\\firefox.exe"),
            ],
        ),
        (
            "Microsoft Edge",
            vec![
                PathBuf::from(&program_files).join("Microsoft\\Edge\\Application\\msedge.exe"),
                PathBuf::from(&program_files_x86).join("Microsoft\\Edge\\Application\\msedge.exe"),
            ],
        ),
        (
            "Brave",
            vec![
                PathBuf::from(&program_files)
                    .join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
                PathBuf::from(&program_files_x86)
                    .join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
                PathBuf::from(&local_app_data)
                    .join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"),
            ],
        ),
        (
            "Opera",
            vec![
                PathBuf::from(&local_app_data).join("Programs\\Opera\\opera.exe"),
                PathBuf::from(&program_files).join("Opera\\launcher.exe"),
            ],
        ),
        (
            "Vivaldi",
            vec![
                PathBuf::from(&local_app_data).join("Vivaldi\\Application\\vivaldi.exe"),
                PathBuf::from(&program_files).join("Vivaldi\\Application\\vivaldi.exe"),
                PathBuf::from(&program_files_x86).join("Vivaldi\\Application\\vivaldi.exe"),
            ],
        ),
        (
             "Helium",
            vec![
                PathBuf::from(&program_files)
                    .join("imput\\Helium\\Application\\chrome.exe"),
                PathBuf::from(&program_files_x86)
                    .join("imput\\Helium\\Application\\chrome.exe"),
                PathBuf::from(&local_app_data)
                    .join("imput\\Helium\\Application\\chrome.exe"),
            ],
        ),
    ];

    for (name, candidates) in known_paths {
        if let Some(path) = candidates.into_iter().find(|candidate| candidate.exists()) {
            let path_string = path.to_string_lossy().to_string();
            let path_key = normalize_path(&path_string);

            if seen_paths.insert(path_key.clone()) {
                let exe_name = executable_name_from_path(&path_string).unwrap_or_default();
                browsers.push(BrowserConfig {
                    id: stable_id("detected", &path_key),
                    name: name.to_string(),
                    path: path_string,
                    private_flag: default_private_flag(&exe_name),
                    source: BrowserSource::Detected,
                    is_hidden: false,
                });
            }
        }
    }

    for browser in detect_browsers_from_registry(&seen_paths) {
        let key = normalize_path(&browser.path);
        if seen_paths.insert(key) {
            browsers.push(browser);
        }
    }

    browsers.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    browsers
}

#[cfg(target_os = "windows")]
fn detect_browsers_from_registry(seen_paths: &HashSet<String>) -> Vec<BrowserConfig> {
    let mut browsers = Vec::new();
    let mut local_seen = seen_paths.clone();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    for base in [
        r"SOFTWARE\Clients\StartMenuInternet",
        r"SOFTWARE\WOW6432Node\Clients\StartMenuInternet",
    ] {
        let Ok(root_key) = hklm.open_subkey(base) else {
            continue;
        };

        for subkey_name in root_key.enum_keys().flatten() {
            let command_key_path = format!(r"{base}\{subkey_name}\shell\open\command");
            let Ok(command_key) = hklm.open_subkey(command_key_path) else {
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
            let exe_name = executable_name_from_path(&executable_path).unwrap_or_else(|| "browser".to_string());

            browsers.push(BrowserConfig {
                id: stable_id("detected", &normalized),
                name: friendly_browser_name(&exe_name, &subkey_name),
                path: executable_path,
                private_flag: default_private_flag(&exe_name),
                source: BrowserSource::Detected,
                is_hidden: false,
            });
        }
    }

    browsers
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

fn friendly_browser_name(exe_name: &str, registry_key: &str) -> String {
    match exe_name.to_lowercase().as_str() {
        "chrome" => "Google Chrome".to_string(),
        "firefox" => "Mozilla Firefox".to_string(),
        "msedge" => "Microsoft Edge".to_string(),
        "brave" | "bravebrowser" | "brave-browser" => "Brave".to_string(),
        "opera" | "launcher" => "Opera".to_string(),
        "vivaldi" => "Vivaldi".to_string(),
        _ => registry_key.replace('_', " "),
    }
}

fn default_private_flag(exe_name: &str) -> Option<String> {
    match exe_name.to_lowercase().as_str() {
        "chrome" | "brave" | "vivaldi" => Some("--incognito".to_string()),
        "firefox" => Some("-private-window".to_string()),
        "msedge" => Some("--inprivate".to_string()),
        "opera" | "launcher" => Some("--private".to_string()),
        _ => None,
    }
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

fn running_processes() -> Result<HashSet<String>, String> {
    let mut command = Command::new("tasklist");
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);
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

fn is_browser_running(browser: &BrowserConfig, running_processes: &HashSet<String>) -> bool {
    let process_name = Path::new(&browser.path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase());

    process_name
        .as_ref()
        .is_some_and(|name| running_processes.contains(name))
}

fn resolve_route(config: &AppConfig, url: &str) -> Result<RouteDecision, String> {
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
                            false,
                            Some(rule.id.clone()),
                            "rule_browser_not_running_use_default",
                        ));
                    }
                }

                return Ok(picker_decision("rule_browser_not_running_no_default"));
            }
        }

        return Ok(picker_decision("rule_browser_not_running"));
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

fn rule_matches(rule: &RuleConfig, url: &str) -> bool {
    let pattern = rule.pattern.trim();
    if pattern.is_empty() {
        return false;
    }

    match rule.pattern_type {
        RulePatternType::Hostname => extract_hostname(url).is_some_and(|hostname| {
            hostname.eq_ignore_ascii_case(&pattern.to_lowercase())
        }),
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

fn normalize_http_url(url: &str) -> Result<Url, String> {
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

fn picker_decision(reason: &str) -> RouteDecision {
    RouteDecision {
        action: RouteAction::ShowPicker,
        reason: reason.to_string(),
        browser_id: None,
        browser_name: None,
        private_mode: false,
        matched_rule_id: None,
    }
}

fn open_url_with_browser(
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

fn clean_cli_value(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn extract_url_from_args(args: &[String]) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--url" {
            if let Some(value) = args.get(index + 1) {
                let candidate = clean_cli_value(value);
                if candidate.starts_with("http://") || candidate.starts_with("https://") {
                    return Some(candidate);
                }
            }
            continue;
        }

        let candidate = clean_cli_value(arg);
        if candidate.starts_with("http://") || candidate.starts_with("https://") {
            return Some(candidate);
        }
    }

    None
}

fn show_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn hide_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn handle_incoming_url(app: &AppHandle, url: &str) -> Result<RouteDecision, String> {
    let config = load_or_init_config(app, false)?;
    let decision = resolve_route(&config, url)?;

    if decision.action == RouteAction::OpenBrowser {
        if let Some(browser_id) = decision.browser_id.as_deref() {
            open_url_with_browser(&config, browser_id, url, decision.private_mode)?;
        }
    } else {
        // Picker is not implemented yet. For now, surface Settings as fallback.
        show_settings_window(app);
    }

    Ok(decision)
}

fn setup_tray(app: &AppHandle) -> Result<(), String> {
    let settings_item = MenuItemBuilder::new("Settings")
        .id("settings")
        .build(app)
        .map_err(|error| format!("Could not create tray menu item (settings): {error}"))?;
    let quit_item = MenuItemBuilder::new("Quit")
        .id("quit")
        .build(app)
        .map_err(|error| format!("Could not create tray menu item (quit): {error}"))?;

    let menu = MenuBuilder::new(app)
        .items(&[&settings_item, &quit_item])
        .build()
        .map_err(|error| format!("Could not build tray menu: {error}"))?;

    let app_for_menu = app.clone();
    let app_for_click = app.clone();
    TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or_else(|| "Could not resolve default app icon for tray.".to_string())?,
        )
        .tooltip("Hops")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |_tray, event| match event.id.as_ref() {
            "settings" => show_settings_window(&app_for_menu),
            "quit" => app_for_menu.exit(0),
            _ => {}
        })
        .on_tray_icon_event(move |_tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_settings_window(&app_for_click);
            }
        })
        .build(app)
        .map_err(|error| format!("Could not build tray icon: {error}"))?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_args: Vec<String> = std::env::args().collect();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(url) = extract_url_from_args(&args) {
                if let Err(error) = handle_incoming_url(&app, &url) {
                    eprintln!("Hops could not handle incoming URL '{url}': {error}");
                }
            } else {
                show_settings_window(&app);
            }
        }))
        .setup(move |app| {
            if let Err(error) = setup_tray(&app.handle()) {
                eprintln!("{error}");
            }

            let startup_url = extract_url_from_args(&initial_args);

            if let Some(url) = startup_url {
                if let Err(error) = handle_incoming_url(&app.handle(), &url) {
                    eprintln!("Hops could not process startup URL '{url}': {error}");
                }
                app.handle().exit(0);
            } else {
                match load_or_init_config(&app.handle(), true) {
                    Ok(config) if config.onboarding_completed => hide_settings_window(&app.handle()),
                    Ok(_) => show_settings_window(&app.handle()),
                    Err(error) => {
                        eprintln!("Could not load config during startup: {error}");
                        show_settings_window(&app.handle());
                    }
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            refresh_browsers,
            list_running_browser_ids,
            preview_route,
            preview_route_with_config,
            route_and_open,
            route_and_open_with_config,
            open_url,
            open_windows_default_apps,
            get_browser_registration_status,
            register_hops_as_browser,
            unregister_hops_as_browser
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{RuleConfig, RulePatternType, rule_matches};

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
        assert!(rule_matches(&rule, "https://jira.mycompany.com/browse/ENG-11"));
        assert!(!rule_matches(&rule, "https://jira.mycompany.com/browse/OPS-11"));
    }

    #[test]
    fn regex_match_works() {
        let rule = test_rule(RulePatternType::Regex, r"^https?://(www\.)?youtube\.com/watch");
        assert!(rule_matches(&rule, "https://youtube.com/watch?v=abc"));
        assert!(!rule_matches(&rule, "https://youtube.com/shorts/abc"));
    }
}
