use crate::browsers::{is_browser_running, running_processes};
use crate::config::load_or_init_config;
use crate::models::{
    AppConfig, PickerBrowserEntry, PickerLaunchSource, PickerSession, RouteAction, RouteDecision,
};
use crate::routing::{normalize_http_url, open_url_with_browser, picker_decision, resolve_route};
use crate::{
    PICKER_CURSOR_OFFSET_X, PICKER_CURSOR_OFFSET_Y, PICKER_IDLE_DESTROY_SECONDS,
    PICKER_MENU_CHROME_HEIGHT, PICKER_MENU_MAX_HEIGHT, PICKER_MENU_MIN_HEIGHT,
    PICKER_MENU_ROW_HEIGHT, PICKER_MENU_WIDTH, PICKER_SESSION_EVENT, PICKER_WINDOW_LABEL,
};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewUrl,
    WebviewWindowBuilder,
};

#[cfg(target_os = "windows")]
const VK_SHIFT: i32 = 0x10;
#[cfg(target_os = "windows")]
const VK_CONTROL: i32 = 0x11;
#[cfg(target_os = "windows")]
const VK_MENU: i32 = 0x12;

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

#[derive(Default)]
pub(crate) struct PickerState {
    pub(crate) session: Mutex<Option<PickerSession>>,
    pub(crate) idle_destroy_token: Mutex<u64>,
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

pub(crate) fn extract_url_from_args(args: &[String]) -> Option<String> {
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

pub(crate) fn show_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub(crate) fn hide_settings_window(app: &AppHandle) {
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

pub(crate) fn hide_picker_window_internal(
    app: &AppHandle,
    state: &PickerState,
) -> Result<(), String> {
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

pub(crate) fn show_picker_window(
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

pub(crate) fn handle_incoming_url(
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
