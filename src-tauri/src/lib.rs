use globset::GlobBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[cfg(target_os = "windows")]
use std::ffi::OsStr;
use std::fs;
use std::hash::{Hash, Hasher};
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, State, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};
use url::Url;

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
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_WRITE};
#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use winreg::HKEY;

const CONFIG_FILENAME: &str = "config.json";
const HOPS_APP_NAME: &str = "Hops";
const HOPS_PROTOCOL_PROG_ID: &str = "HopsURL";
const HOPS_HTML_PROG_ID: &str = "HopsHTML";
const HOPS_CUSTOM_URI_SCHEME: &str = "Hops";
const PICKER_WINDOW_LABEL: &str = "picker";
const PICKER_SESSION_EVENT: &str = "picker-session";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(target_os = "windows")]
const ASSOCF_NONE: u32 = 0;
#[cfg(target_os = "windows")]
const ASSOCSTR_PROGID: u32 = 20;
#[cfg(target_os = "windows")]
const VK_SHIFT: i32 = 0x10;
#[cfg(target_os = "windows")]
const VK_CONTROL: i32 = 0x11;
#[cfg(target_os = "windows")]
const VK_MENU: i32 = 0x12;
const PICKER_MENU_WIDTH: u32 = 280;
const PICKER_MENU_MIN_HEIGHT: u32 = 128;
const PICKER_MENU_MAX_HEIGHT: u32 = 340;
const PICKER_MENU_ROW_HEIGHT: u32 = 46;
const PICKER_MENU_CHROME_HEIGHT: u32 = 92;
const PICKER_CURSOR_OFFSET_X: i32 = 6;
const PICKER_CURSOR_OFFSET_Y: i32 = 10;
const PICKER_IDLE_DESTROY_SECONDS: u64 = 15;

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

#[cfg(target_os = "windows")]
#[repr(C)]
struct WinPoint {
    x: i32,
    y: i32,
}

#[cfg(target_os = "windows")]
#[link(name = "User32")]
unsafe extern "system" {
    fn GetAsyncKeyState(v_key: i32) -> i16;
    fn GetCursorPos(lp_point: *mut WinPoint) -> i32;
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ThemePreference {
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct BrowserConfig {
    id: String,
    name: String,
    path: String,
    private_flag: Option<String>,
    source: BrowserSource,
    is_hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserFamily {
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
struct ResolvedBrowserMetadata {
    name: String,
    family: BrowserFamily,
    private_flag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RuleConfig {
    id: String,
    pattern: String,
    pattern_type: RulePatternType,
    browser_id: String,
    private_mode: bool,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
    version: u32,
    always_show_picker: bool,
    use_defaults_when_not_running: bool,
    disable_transparency: bool,
    #[serde(default = "default_theme_preference")]
    theme_preference: ThemePreference,
    #[serde(default)]
    onboarding_completed: bool,
    default_browser_id: Option<String>,
    browsers: Vec<BrowserConfig>,
    rules: Vec<RuleConfig>,
}

fn default_theme_preference() -> ThemePreference {
    ThemePreference::Light
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PickerLaunchSource {
    Route,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PickerBrowserEntry {
    id: String,
    name: String,
    private_flag: Option<String>,
    is_default: bool,
    is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PickerSession {
    url: String,
    reason: String,
    source: PickerLaunchSource,
    preferred_browser_id: Option<String>,
    preferred_private_mode: bool,
    disable_transparency: bool,
    theme_preference: ThemePreference,
    always_show_picker: bool,
    alt_pressed: bool,
    browsers: Vec<PickerBrowserEntry>,
}

#[derive(Default)]
struct PickerState {
    session: Mutex<Option<PickerSession>>,
    idle_destroy_token: Mutex<u64>,
}

#[tauri::command]
fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    load_or_init_config(&app, true)
}

#[tauri::command]
fn save_config(app: AppHandle, config: AppConfig) -> Result<AppConfig, String> {
    save_config_internal(&app, config, true)
}

#[tauri::command(async)]
fn refresh_browsers(app: AppHandle) -> Result<AppConfig, String> {
    let mut config = load_or_init_config(&app, false)?;
    merge_detected_browsers(&mut config, detect_browsers());
    save_config_internal(&app, config, false)
}

#[tauri::command]
fn reset_config(app: AppHandle) -> Result<AppConfig, String> {
    let config = reset_config_with_detected_browsers(detect_browsers(), true);
    save_config_internal(&app, config, false)
}

#[tauri::command(async)]
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

#[tauri::command(async)]
fn preview_route(app: AppHandle, url: String) -> Result<RouteDecision, String> {
    let config = load_or_init_config(&app, false)?;
    resolve_route(&config, &url)
}

#[tauri::command(async)]
fn preview_route_with_config(config: AppConfig, url: String) -> Result<RouteDecision, String> {
    let mut normalized = config;
    normalize_config(&mut normalized);
    validate_config(&normalized)?;
    resolve_route(&normalized, &url)
}

#[tauri::command(async)]
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

#[tauri::command(async)]
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

#[tauri::command(async)]
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
fn get_picker_state(state: State<'_, PickerState>) -> Result<Option<PickerSession>, String> {
    let session = state
        .session
        .lock()
        .map_err(|_| "Picker state lock was poisoned.".to_string())?;
    Ok(session.clone())
}

#[tauri::command(async)]
fn show_picker_for_url(
    app: AppHandle,
    state: State<'_, PickerState>,
    url: String,
    preferred_browser_id: Option<String>,
    preferred_private_mode: Option<bool>,
) -> Result<(), String> {
    let config = load_or_init_config(&app, false)?;
    show_picker_window(
        &app,
        &state,
        &config,
        &url,
        PickerLaunchSource::Manual,
        "manual_open",
        preferred_browser_id.as_deref(),
        preferred_private_mode.unwrap_or(false),
    )
}

#[tauri::command]
fn hide_picker_window(app: AppHandle, state: State<'_, PickerState>) -> Result<(), String> {
    hide_picker_window_internal(&app, &state)
}

#[tauri::command]
fn show_settings_window_command(
    app: AppHandle,
    state: State<'_, PickerState>,
) -> Result<(), String> {
    hide_picker_window_internal(&app, &state)?;
    show_settings_window(&app);
    Ok(())
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

    fs::create_dir_all(&app_data).map_err(|error| {
        format!(
            "Could not create app data directory {:?}: {error}",
            app_data
        )
    })?;

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

    let current_http_prog_id =
        query_url_association_prog_id("http").or_else(|| read_url_user_choice_prog_id("http"));
    let current_https_prog_id =
        query_url_association_prog_id("https").or_else(|| read_url_user_choice_prog_id("https"));

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

    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16(&buffer[..end]).ok()
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
fn read_url_user_choice_prog_id(scheme: &str) -> Option<String> {
    let key_path = format!(
        "Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\{scheme}\\UserChoice"
    );
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

    if let Ok(registered_apps) =
        hkcu.open_subkey_with_flags("Software\\RegisteredApplications", KEY_WRITE)
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

    let original_config = config.clone();
    normalize_config(&mut config);

    if auto_populate_browsers && config.browsers.is_empty() {
        merge_detected_browsers(&mut config, detect_browsers());
    }

    if config != original_config {
        write_config_file(&path, &config)?;
    }

    Ok(config)
}

fn default_config() -> AppConfig {
    AppConfig {
        version: 1,
        always_show_picker: false,
        use_defaults_when_not_running: false,
        disable_transparency: false,
        theme_preference: default_theme_preference(),
        onboarding_completed: false,
        default_browser_id: None,
        browsers: Vec::new(),
        rules: Vec::new(),
    }
}

fn reset_config_with_detected_browsers(
    detected_browsers: Vec<BrowserConfig>,
    onboarding_completed: bool,
) -> AppConfig {
    let mut config = default_config();
    config.onboarding_completed = onboarding_completed;
    merge_detected_browsers(&mut config, detected_browsers);
    config
}

fn normalize_config(config: &mut AppConfig) {
    if config.version == 0 {
        config.version = 1;
    }

    hydrate_detected_browser_defaults(config);

    let browser_ids: HashSet<String> = config
        .browsers
        .iter()
        .map(|browser| browser.id.clone())
        .collect();

    if let Some(default_browser_id) = config.default_browser_id.clone() {
        if !browser_ids.contains(&default_browser_id) {
            config.default_browser_id = None;
        }
    }

    config
        .rules
        .retain(|rule| browser_ids.contains(&rule.browser_id));
}

fn hydrate_detected_browser_defaults(config: &mut AppConfig) {
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

fn validate_config(config: &AppConfig) -> Result<(), String> {
    let browser_ids: HashSet<String> = config
        .browsers
        .iter()
        .map(|browser| browser.id.clone())
        .collect();

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
fn registry_browser_roots() -> &'static [(HKEY, &'static str)] {
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

fn build_detected_browser(
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

fn resolve_browser_metadata(
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
fn running_processes() -> Result<HashSet<String>, String> {
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
fn running_processes() -> Result<HashSet<String>, String> {
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

fn is_browser_running(browser: &BrowserConfig, running_processes: &HashSet<String>) -> bool {
    let normalized_path = normalize_path(&browser.path);
    running_processes.contains(&normalized_path)
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

fn rule_matches(rule: &RuleConfig, url: &str) -> bool {
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

#[cfg(target_os = "windows")]
fn is_ctrl_shift_picker_trigger_active() -> bool {
    let ctrl_pressed = unsafe { GetAsyncKeyState(VK_CONTROL) } < 0;
    let shift_pressed = unsafe { GetAsyncKeyState(VK_SHIFT) } < 0;
    ctrl_pressed && shift_pressed
}

#[cfg(not(target_os = "windows"))]
fn is_ctrl_shift_picker_trigger_active() -> bool {
    false
}

#[cfg(target_os = "windows")]
fn is_alt_pressed() -> bool {
    (unsafe { GetAsyncKeyState(VK_MENU) }) < 0
}

#[cfg(not(target_os = "windows"))]
fn is_alt_pressed() -> bool {
    false
}

#[cfg(target_os = "windows")]
fn current_cursor_position() -> Option<(i32, i32)> {
    let mut point = WinPoint { x: 0, y: 0 };
    let success = unsafe { GetCursorPos(&mut point) };
    if success == 0 {
        return None;
    }

    Some((point.x, point.y))
}

#[cfg(not(target_os = "windows"))]
fn current_cursor_position() -> Option<(i32, i32)> {
    None
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

fn picker_window_height(browser_count: usize) -> u32 {
    let browser_rows_height = browser_count as u32 * PICKER_MENU_ROW_HEIGHT;
    let desired = PICKER_MENU_CHROME_HEIGHT + browser_rows_height;
    desired.clamp(PICKER_MENU_MIN_HEIGHT, PICKER_MENU_MAX_HEIGHT)
}

fn picker_window_position(
    app: &AppHandle,
    cursor_x: i32,
    cursor_y: i32,
    width: u32,
    height: u32,
) -> (i32, i32) {
    let desired_x = cursor_x + PICKER_CURSOR_OFFSET_X;
    let desired_y = cursor_y + PICKER_CURSOR_OFFSET_Y;

    if let Ok(Some(monitor)) = app.monitor_from_point(cursor_x as f64, cursor_y as f64) {
        let monitor_position = monitor.position();
        let monitor_size = monitor.size();
        let min_x = monitor_position.x;
        let min_y = monitor_position.y;
        let max_x = (monitor_position.x + monitor_size.width as i32 - width as i32).max(min_x);
        let max_y = (monitor_position.y + monitor_size.height as i32 - height as i32).max(min_y);
        return (desired_x.clamp(min_x, max_x), desired_y.clamp(min_y, max_y));
    }

    (desired_x.max(0), desired_y.max(0))
}

fn picker_debug_log(message: &str) {
    #[cfg(debug_assertions)]
    eprintln!("Hops picker: {message}");
}

fn next_picker_idle_destroy_token(state: &PickerState) -> Result<u64, String> {
    let mut token = state
        .idle_destroy_token
        .lock()
        .map_err(|_| "Picker idle destroy lock was poisoned.".to_string())?;
    *token += 1;
    Ok(*token)
}

fn cancel_picker_idle_destroy(state: &PickerState) -> Result<(), String> {
    let _ = next_picker_idle_destroy_token(state)?;
    picker_debug_log("destroy canceled");
    Ok(())
}

fn schedule_picker_idle_destroy(app: &AppHandle, state: &PickerState) -> Result<(), String> {
    let token = next_picker_idle_destroy_token(state)?;
    picker_debug_log("destroy scheduled");

    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(PICKER_IDLE_DESTROY_SECONDS));

        let app_for_main = app.clone();
        if let Err(error) = app.run_on_main_thread(move || {
            let state = app_for_main.state::<PickerState>();
            let current_token = match state.idle_destroy_token.lock() {
                Ok(current) => *current,
                Err(_) => {
                    eprintln!("Hops picker: idle destroy lock was poisoned.");
                    return;
                }
            };

            if current_token != token {
                return;
            }

            if let Some(window) = app_for_main.get_webview_window(PICKER_WINDOW_LABEL) {
                if let Err(error) = window.destroy() {
                    eprintln!("Hops picker: could not destroy idle picker window: {error}");
                    return;
                }

                picker_debug_log("destroyed");
            }
        }) {
            eprintln!("Hops picker: could not schedule idle destroy on main thread: {error}");
        }
    });

    Ok(())
}

fn ensure_picker_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(PICKER_WINDOW_LABEL) {
        picker_debug_log("reused");
        return Ok(window);
    }

    let window = WebviewWindowBuilder::new(
        app,
        PICKER_WINDOW_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Hops Picker")
    .inner_size(PICKER_MENU_WIDTH as f64, PICKER_MENU_MIN_HEIGHT as f64)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .visible(false)
    .focused(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    .build()
    .map_err(|error| format!("Could not build picker window during picker create: {error}"))?;

    picker_debug_log("created");
    Ok(window)
}

fn hide_picker_window_internal(app: &AppHandle, state: &PickerState) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(PICKER_WINDOW_LABEL) {
        let was_visible = window.is_visible().unwrap_or(true);
        window
            .hide()
            .map_err(|error| format!("Could not hide picker window during picker hide: {error}"))?;

        if was_visible {
            picker_debug_log("hidden");
            schedule_picker_idle_destroy(app, state)?;
        }
    }

    Ok(())
}

fn store_picker_session(state: &PickerState, session: PickerSession) -> Result<(), String> {
    let mut current = state
        .session
        .lock()
        .map_err(|_| "Picker state lock was poisoned.".to_string())?;
    *current = Some(session);
    Ok(())
}

fn build_picker_session(
    config: &AppConfig,
    url: &str,
    source: PickerLaunchSource,
    reason: &str,
    preferred_browser_id: Option<&str>,
    preferred_private_mode: bool,
) -> Result<PickerSession, String> {
    let normalized_url = normalize_http_url(url)?.to_string();
    let running = running_processes().unwrap_or_default();

    let browsers = config
        .browsers
        .iter()
        .filter(|browser| !browser.is_hidden)
        .map(|browser| PickerBrowserEntry {
            id: browser.id.clone(),
            name: browser.name.clone(),
            private_flag: browser.private_flag.clone(),
            is_default: config
                .default_browser_id
                .as_ref()
                .is_some_and(|default_id| default_id == &browser.id),
            is_running: is_browser_running(browser, &running),
        })
        .collect();

    Ok(PickerSession {
        url: normalized_url,
        reason: reason.to_string(),
        source,
        preferred_browser_id: preferred_browser_id.map(str::to_string),
        preferred_private_mode,
        disable_transparency: config.disable_transparency,
        theme_preference: config.theme_preference,
        always_show_picker: config.always_show_picker,
        alt_pressed: is_alt_pressed(),
        browsers,
    })
}

fn show_picker_window(
    app: &AppHandle,
    state: &PickerState,
    config: &AppConfig,
    url: &str,
    source: PickerLaunchSource,
    reason: &str,
    preferred_browser_id: Option<&str>,
    preferred_private_mode: bool,
) -> Result<(), String> {
    cancel_picker_idle_destroy(state)?;

    let session = build_picker_session(
        config,
        url,
        source,
        reason,
        preferred_browser_id,
        preferred_private_mode,
    )?;
    store_picker_session(state, session.clone())?;

    let window = ensure_picker_window(app)?;
    let menu_height = picker_window_height(session.browsers.len());
    window
        .set_size(Size::Physical(PhysicalSize::new(
            PICKER_MENU_WIDTH,
            menu_height,
        )))
        .map_err(|error| format!("Could not resize picker window during picker show: {error}"))?;

    if let Some((cursor_x, cursor_y)) = current_cursor_position() {
        let (x, y) =
            picker_window_position(app, cursor_x, cursor_y, PICKER_MENU_WIDTH, menu_height);
        window
            .set_position(Position::Physical(PhysicalPosition::new(x, y)))
            .map_err(|error| format!("Could not move picker window during picker show: {error}"))?;
    } else {
        let _ = window.center();
    }

    window
        .show()
        .map_err(|error| format!("Could not show picker window during picker show: {error}"))?;
    let _ = window.unminimize();
    let _ = window.set_focus();
    app.emit_to(PICKER_WINDOW_LABEL, PICKER_SESSION_EVENT, session)
        .map_err(|error| format!("Could not emit picker state during picker show: {error}"))?;

    Ok(())
}

fn handle_incoming_url(
    app: &AppHandle,
    state: &PickerState,
    url: &str,
) -> Result<RouteDecision, String> {
    let config = load_or_init_config(app, false)?;

    if is_ctrl_shift_picker_trigger_active() {
        let decision = picker_decision("ctrl_shift_click");
        show_picker_window(
            app,
            state,
            &config,
            url,
            PickerLaunchSource::Route,
            &decision.reason,
            decision.browser_id.as_deref(),
            decision.private_mode,
        )?;
        return Ok(decision);
    }

    let decision = resolve_route(&config, url)?;

    if decision.action == RouteAction::OpenBrowser {
        hide_picker_window_internal(app, state)?;
        if let Some(browser_id) = decision.browser_id.as_deref() {
            open_url_with_browser(&config, browser_id, url, decision.private_mode)?;
        }
    } else {
        show_picker_window(
            app,
            state,
            &config,
            url,
            PickerLaunchSource::Route,
            &decision.reason,
            decision.browser_id.as_deref(),
            decision.private_mode,
        )?;
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
        .manage(PickerState::default())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name(HOPS_APP_NAME)
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(url) = extract_url_from_args(&args) {
                let state = app.state::<PickerState>();
                if let Err(error) = handle_incoming_url(&app, &state, &url) {
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
                hide_settings_window(&app.handle());
                let state = app.state::<PickerState>();
                if let Err(error) = handle_incoming_url(&app.handle(), &state, &url) {
                    eprintln!("Hops could not process startup URL '{url}': {error}");
                }
            } else {
                match load_or_init_config(&app.handle(), true) {
                    Ok(config) if config.onboarding_completed => {
                        hide_settings_window(&app.handle())
                    }
                    Ok(_) => show_settings_window(&app.handle()),
                    Err(error) => {
                        eprintln!("Could not load config during startup: {error}");
                        show_settings_window(&app.handle());
                    }
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            ("main", WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            (PICKER_WINDOW_LABEL, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let app = window.app_handle();
                let state = app.state::<PickerState>();
                let _ = hide_picker_window_internal(app, &state);
            }
            (PICKER_WINDOW_LABEL, WindowEvent::Focused(false)) => {
                let app = window.app_handle();
                let state = app.state::<PickerState>();
                let _ = hide_picker_window_internal(app, &state);
            }
            _ => {}
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
            get_picker_state,
            show_picker_for_url,
            hide_picker_window,
            show_settings_window_command,
            open_windows_default_apps,
            get_browser_registration_status,
            register_hops_as_browser,
            unregister_hops_as_browser,
            reset_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    use super::registry_browser_roots;
    use super::{
        build_detected_browser, hydrate_detected_browser_defaults, merge_detected_browsers,
        reset_config_with_detected_browsers, resolve_browser_metadata, resolve_route, rule_matches,
        write_config_file, AppConfig, BrowserConfig, BrowserFamily, BrowserSource, RouteAction,
        RuleConfig, RulePatternType, ThemePreference,
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    #[cfg(target_os = "windows")]
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

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

    #[test]
    fn chromium_family_metadata_gets_incognito_flag() {
        for browser in [
            ("C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe", "Google Chrome"),
            ("C:\\Users\\Hugo\\AppData\\Local\\Vivaldi\\Application\\vivaldi.exe", "Vivaldi"),
            (
                "C:\\Users\\Hugo\\AppData\\Local\\BraveSoftware\\Brave-Browser\\Application\\brave.exe",
                "Brave",
            ),
            ("C:\\Tools\\Chromium\\chromium.exe", "Google Chrome"),
            ("C:\\Program Files\\imput\\Helium\\Application\\chrome.exe", "Helium"),
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

    #[test]
    fn reset_config_restores_defaults_and_keeps_only_detected_browsers() {
        let config = reset_config_with_detected_browsers(
            vec![
                build_detected_browser(
                    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
                    Some("Google Chrome"),
                    None,
                ),
                build_detected_browser(
                    "C:\\Program Files\\Mozilla Firefox\\firefox.exe",
                    Some("Mozilla Firefox"),
                    None,
                ),
            ],
            true,
        );

        assert_eq!(config.version, 1);
        assert!(!config.always_show_picker);
        assert!(!config.use_defaults_when_not_running);
        assert!(!config.disable_transparency);
        assert_eq!(config.theme_preference, ThemePreference::Light);
        assert!(config.onboarding_completed);
        assert_eq!(config.default_browser_id, None);
        assert!(config.rules.is_empty());
        assert_eq!(config.browsers.len(), 2);
        assert!(config
            .browsers
            .iter()
            .all(|browser| browser.source == BrowserSource::Detected));
    }

    #[test]
    fn reset_config_persists_valid_json() {
        let config = reset_config_with_detected_browsers(
            vec![build_detected_browser(
                "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
                Some("Google Chrome"),
                None,
            )],
            true,
        );
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("hops-reset-config-{unique}.json"));

        write_config_file(&path, &config).expect("reset config should write");
        let data = fs::read_to_string(&path).expect("reset config file should be readable");
        let loaded =
            serde_json::from_str::<AppConfig>(&data).expect("reset config file should parse");

        assert_eq!(loaded.rules.len(), 0);
        assert_eq!(loaded.default_browser_id, None);
        assert!(loaded.onboarding_completed);

        let _ = fs::remove_file(path);
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
