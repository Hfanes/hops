use crate::browsers::{
    detect_browsers, is_browser_running, merge_detected_browsers, running_processes,
};
use crate::config::{
    load_or_init_config, normalize_config, reset_config_with_detected_browsers,
    save_config_internal, validate_config,
};
use crate::models::{
    AppConfig, BrowserRegistrationStatus, OpenUrlRequest, PickerLaunchSource, PickerSession,
    RouteAction, RouteDecision,
};
use crate::picker::{
    hide_picker_window_internal, show_picker_window, show_settings_window, PickerState,
};
use crate::registration::{
    browser_registration_status, open_windows_default_apps as open_windows_default_apps_internal,
    register_hops_as_browser as register_hops_as_browser_internal,
    unregister_hops_as_browser as unregister_hops_as_browser_internal,
};
use crate::routing::{open_url_with_browser, resolve_route};
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) fn load_config(app: AppHandle) -> Result<AppConfig, String> {
    load_or_init_config(&app, true)
}

#[tauri::command]
pub(crate) fn save_config(app: AppHandle, config: AppConfig) -> Result<AppConfig, String> {
    save_config_internal(&app, config, true)
}

#[tauri::command(async)]
pub(crate) fn refresh_browsers(app: AppHandle) -> Result<AppConfig, String> {
    let mut config = load_or_init_config(&app, false)?;
    merge_detected_browsers(&mut config, detect_browsers());
    save_config_internal(&app, config, false)
}

#[tauri::command]
pub(crate) fn reset_config(app: AppHandle) -> Result<AppConfig, String> {
    let config = reset_config_with_detected_browsers(detect_browsers(), true);
    save_config_internal(&app, config, false)
}

#[tauri::command(async)]
pub(crate) fn list_running_browser_ids(app: AppHandle) -> Result<Vec<String>, String> {
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
pub(crate) fn preview_route(app: AppHandle, url: String) -> Result<RouteDecision, String> {
    let config = load_or_init_config(&app, false)?;
    resolve_route(&config, &url)
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
pub(crate) fn route_and_open(app: AppHandle, url: String) -> Result<RouteDecision, String> {
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
pub(crate) fn route_and_open_with_config(
    config: AppConfig,
    url: String,
) -> Result<RouteDecision, String> {
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
pub(crate) fn open_url(app: AppHandle, request: OpenUrlRequest) -> Result<(), String> {
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
        "manual_open",
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

#[tauri::command]
pub(crate) fn open_windows_default_apps() -> Result<(), String> {
    open_windows_default_apps_internal()
}

#[tauri::command]
pub(crate) fn get_browser_registration_status() -> Result<BrowserRegistrationStatus, String> {
    browser_registration_status()
}

#[tauri::command]
pub(crate) fn register_hops_as_browser() -> Result<BrowserRegistrationStatus, String> {
    register_hops_as_browser_internal()
}

#[tauri::command]
pub(crate) fn unregister_hops_as_browser() -> Result<BrowserRegistrationStatus, String> {
    unregister_hops_as_browser_internal()
}
