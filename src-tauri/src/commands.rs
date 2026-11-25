use crate::browsers::{
    browser_executable_exists, detect_browsers, is_browser_running, merge_detected_browsers,
    running_processes, validate_manual_browser_request,
};
use crate::config::{
    config_file_path, load_or_init_config, normalize_config, reset_config_with_detected_browsers,
    save_config_internal, take_config_recovery_notice as take_config_recovery_notice_internal,
    update_config, validate_config,
};
use crate::models::{
    AppConfig, BrowserRegistrationStatus, ManualBrowserValidationRequest,
    ManualBrowserValidationResult, OpenUrlRequest, PickerLaunchSource, PickerSession, RouteAction,
    RouteDecision, RouteReason,
};
use crate::picker::{
    hide_picker_window_internal, show_picker_window, show_settings_window, PickerState,
};
use crate::registration::{
    browser_registration_status, open_windows_default_apps as open_windows_default_apps_internal,
    register_hops_as_browser as register_hops_as_browser_internal,
    unregister_hops_as_browser as unregister_hops_as_browser_internal,
};
use crate::routing::{open_url_with_browser, regex_pattern_error, resolve_route};
use tauri::{AppHandle, State};

// Commands that touch the disk or registry, scan processes, or spawn programs
// are `async` so they run off the main thread and never freeze the UI. Config
// access is serialized by `ConfigCache`. Only commands that read in-memory state
// or move windows stay synchronous.

#[tauri::command(async)]
pub(crate) fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    load_or_init_config(&app, true)
}

/// Set once after an invalid `config.json` was backed up and replaced; the
/// settings window shows it and later calls return `None`.
#[tauri::command(async)]
pub(crate) fn take_config_recovery_notice(app: AppHandle) -> Option<String> {
    take_config_recovery_notice_internal(&app)
}

#[tauri::command(async)]
pub(crate) fn get_config_file_path(app: AppHandle) -> Result<String, String> {
    config_file_path(&app).map(|path| path.display().to_string())
}

#[tauri::command(async)]
pub(crate) fn open_config_folder(app: AppHandle) -> Result<(), String> {
    let config_path = config_file_path(&app)?;
    let config_folder = config_path
        .parent()
        .ok_or_else(|| "Could not resolve config folder.".to_string())?;

    #[cfg(target_os = "windows")]
    {
        use crate::CREATE_NO_WINDOW;
        use std::os::windows::process::CommandExt;
        use std::process::Command;

        let mut command = Command::new("explorer.exe");
        command.arg(config_folder);
        command
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| format!("Could not open config folder: {error}"))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Opening the config folder is only supported on Windows.".to_string())
    }
}

#[tauri::command(async)]
pub(crate) fn save_config(app: AppHandle, config: AppConfig) -> Result<AppConfig, String> {
    save_config_internal(&app, config, true)
}

#[tauri::command(async)]
pub(crate) fn validate_manual_browser(
    request: ManualBrowserValidationRequest,
) -> Result<ManualBrowserValidationResult, String> {
    validate_manual_browser_request(&request)
}

/// Validates regex rule patterns with the same engine routing uses.
/// Returns one entry per pattern, in order: `None` when valid, else the error.
#[tauri::command(async)]
pub(crate) fn validate_regex_patterns(patterns: Vec<String>) -> Vec<Option<String>> {
    patterns
        .iter()
        .map(|pattern| regex_pattern_error(pattern))
        .collect()
}

#[tauri::command(async)]
pub(crate) fn refresh_browsers(app: AppHandle) -> Result<AppConfig, String> {
    // Detect first: it reads the registry and disk, and the update holds the
    // config lock that URL handling also waits for.
    let detected = detect_browsers();
    update_config(&app, |config| merge_detected_browsers(config, detected))
}

#[tauri::command(async)]
pub(crate) fn reset_config(app: AppHandle) -> Result<AppConfig, String> {
    let config = reset_config_with_detected_browsers(detect_browsers(), true);
    save_config_internal(&app, config, false)
}

#[tauri::command(async)]
pub(crate) fn list_running_browser_ids(app: AppHandle) -> Result<Vec<String>, String> {
    let config = load_or_init_config(&app, false)?;
    let running = running_processes(&config.browsers)?;

    Ok(config
        .browsers
        .iter()
        .filter(|browser| !browser.is_hidden && is_browser_running(browser, &running))
        .map(|browser| browser.id.clone())
        .collect())
}

/// Browsers whose executable is no longer on disk, for the "missing" badge.
#[tauri::command(async)]
pub(crate) fn list_missing_browser_ids(app: AppHandle) -> Result<Vec<String>, String> {
    let config = load_or_init_config(&app, false)?;
    Ok(config
        .browsers
        .iter()
        .filter(|browser| !browser_executable_exists(browser))
        .map(|browser| browser.id.clone())
        .collect())
}

#[tauri::command(async)]
pub(crate) fn preview_route_with_config(
    config: AppConfig,
    url: String,
) -> Result<RouteDecision, String> {
    let mut normalized = config;
    normalize_config(&mut normalized);
    validate_config(&normalized)?;
    resolve_route(&normalized, &url)
}

#[tauri::command(async)]
pub(crate) fn route_and_open_with_config(
    app: AppHandle,
    config: AppConfig,
    url: String,
) -> Result<RouteDecision, String> {
    let mut normalized = config;
    normalize_config(&mut normalized);
    validate_config(&normalized)?;
    let decision = resolve_route(&normalized, &url)?;

    if decision.action == RouteAction::OpenBrowser {
        if let Some(browser_id) = decision.browser_id.as_deref() {
            let trusted_config = load_or_init_config(&app, false)?;
            open_url_with_browser(&trusted_config, browser_id, &url, decision.private_mode)?;
        }
    }

    Ok(decision)
}

#[tauri::command(async)]
pub(crate) fn open_url(
    app: AppHandle,
    state: State<'_, PickerState>,
    request: OpenUrlRequest,
) -> Result<(), String> {
    {
        let session = state
            .session
            .lock()
            .map_err(|_| "Picker state lock was poisoned.".to_string())?;
        ensure_url_matches_picker_session(
            session.as_ref().map(|session| session.url.as_str()),
            &request.url,
        )?;
    }

    let config = load_or_init_config(&app, false)?;
    open_url_with_browser(
        &config,
        &request.browser_id,
        &request.url,
        request.private_mode,
    )
}

#[tauri::command]
pub(crate) fn get_picker_state(
    state: State<'_, PickerState>,
) -> Result<Option<PickerSession>, String> {
    let session = state
        .session
        .lock()
        .map_err(|_| "Picker state lock was poisoned.".to_string())?;
    Ok(session.clone())
}

#[tauri::command(async)]
pub(crate) fn show_picker_for_url(
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
        RouteReason::ManualOpen,
        preferred_browser_id.as_deref(),
        preferred_private_mode.unwrap_or(false),
    )
}

#[tauri::command]
pub(crate) fn hide_picker_window(
    app: AppHandle,
    state: State<'_, PickerState>,
) -> Result<(), String> {
    hide_picker_window_internal(&app, &state)
}

#[tauri::command]
pub(crate) fn show_settings_window_command(
    app: AppHandle,
    state: State<'_, PickerState>,
) -> Result<(), String> {
    hide_picker_window_internal(&app, &state)?;
    show_settings_window(&app);
    Ok(())
}

#[tauri::command(async)]
pub(crate) fn open_windows_default_apps() -> Result<(), String> {
    open_windows_default_apps_internal()
}

#[tauri::command(async)]
pub(crate) fn get_browser_registration_status() -> Result<BrowserRegistrationStatus, String> {
    browser_registration_status()
}

#[tauri::command(async)]
pub(crate) fn register_hops_as_browser() -> Result<BrowserRegistrationStatus, String> {
    register_hops_as_browser_internal()
}

#[tauri::command(async)]
pub(crate) fn unregister_hops_as_browser() -> Result<BrowserRegistrationStatus, String> {
    unregister_hops_as_browser_internal()
}

/// The picker may only open the URL it was shown for. If a new link replaced
/// the session after the picker rendered, the request fails instead of opening
/// the new URL in the browser picked for the old one.
fn ensure_url_matches_picker_session(
    session_url: Option<&str>,
    requested_url: &str,
) -> Result<(), String> {
    match session_url {
        Some(session_url) if session_url == requested_url => Ok(()),
        Some(_) => Err("This link is no longer the one in the picker.".to_string()),
        None => Err("There is no link to open in the picker.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_url_matches_picker_session;

    #[test]
    fn allows_the_url_of_the_current_picker_session() {
        assert!(ensure_url_matches_picker_session(
            Some("https://example.com/a"),
            "https://example.com/a",
        )
        .is_ok());
    }

    #[test]
    fn rejects_a_url_that_differs_from_the_picker_session() {
        assert!(ensure_url_matches_picker_session(
            Some("https://example.com/a"),
            "https://example.com/b",
        )
        .is_err());
    }

    #[test]
    fn rejects_any_url_when_there_is_no_picker_session() {
        assert!(ensure_url_matches_picker_session(None, "https://example.com/a").is_err());
    }
}
