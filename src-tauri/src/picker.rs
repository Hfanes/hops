use crate::browsers::RunningProcessSnapshot;
use crate::config::load_or_init_config;
use crate::models::{
    AppConfig, PickerBrowserEntry, PickerLaunchSource, PickerSession, RouteAction, RouteDecision,
};
use crate::routing::{
    normalize_http_url, open_url_with_browser, picker_decision,
    resolve_route_with_running_processes,
};
use crate::{
    PICKER_CURSOR_OFFSET_X, PICKER_CURSOR_OFFSET_Y, PICKER_IDLE_DESTROY_SECONDS,
    PICKER_MENU_CHROME_HEIGHT, PICKER_MENU_MAX_HEIGHT, PICKER_MENU_MIN_HEIGHT,
    PICKER_MENU_ROW_HEIGHT, PICKER_MENU_WIDTH, PICKER_SESSION_EVENT, PICKER_WINDOW_LABEL,
};
#[cfg(target_os = "windows")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewUrl,
    WebviewWindowBuilder,
};

#[cfg(target_os = "windows")]
const VK_SHIFT: i32 = 0x10;
#[cfg(target_os = "windows")]
const VK_CONTROL: i32 = 0x11;
#[cfg(target_os = "windows")]
const VK_MENU: i32 = 0x12;
#[cfg(target_os = "windows")]
const VK_LSHIFT: i32 = 0xA0;
#[cfg(target_os = "windows")]
const VK_RSHIFT: i32 = 0xA1;
#[cfg(target_os = "windows")]
const VK_LCONTROL: i32 = 0xA2;
#[cfg(target_os = "windows")]
const VK_RCONTROL: i32 = 0xA3;
#[cfg(target_os = "windows")]
const WH_KEYBOARD_LL: i32 = 13;
#[cfg(target_os = "windows")]
const HC_ACTION: i32 = 0;
#[cfg(target_os = "windows")]
const WM_KEYDOWN: u32 = 0x0100;
#[cfg(target_os = "windows")]
const WM_KEYUP: u32 = 0x0101;
#[cfg(target_os = "windows")]
const WM_SYSKEYDOWN: u32 = 0x0104;
#[cfg(target_os = "windows")]
const WM_SYSKEYUP: u32 = 0x0105;
const PICKER_SHORTCUT_LATCH_DURATION: Duration = Duration::from_millis(2_000);

#[cfg(target_os = "windows")]
#[repr(C)]
struct WinPoint {
    x: i32,
    y: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct WinMsg {
    hwnd: isize,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    point: WinPoint,
    private: u32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct KeyboardHookEvent {
    vk_code: u32,
    scan_code: u32,
    flags: u32,
    time: u32,
    extra_info: usize,
}

#[cfg(target_os = "windows")]
type HookProc = Option<unsafe extern "system" fn(i32, usize, isize) -> isize>;

#[cfg(target_os = "windows")]
#[link(name = "User32")]
unsafe extern "system" {
    fn GetAsyncKeyState(v_key: i32) -> i16;
    fn GetCursorPos(lp_point: *mut WinPoint) -> i32;
    fn SetWindowsHookExW(
        id_hook: i32,
        hook_proc: HookProc,
        instance: isize,
        thread_id: u32,
    ) -> isize;
    fn CallNextHookEx(hook: isize, code: i32, w_param: usize, l_param: isize) -> isize;
    fn GetMessageW(message: *mut WinMsg, window: isize, filter_min: u32, filter_max: u32) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "Kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> isize;
}

#[derive(Default)]
pub(crate) struct PickerState {
    pub(crate) session: Mutex<Option<PickerSession>>,
    pub(crate) idle_destroy_token: Mutex<u64>,
    shortcut_latch: Arc<PickerShortcutLatch>,
}

#[derive(Default)]
struct PickerShortcutLatch {
    last_ctrl_shift_seen: Mutex<Option<Instant>>,
}

impl PickerState {
    #[cfg(test)]
    fn remember_picker_shortcut_at(&self, seen_at: Instant) {
        self.shortcut_latch.remember_at(seen_at);
    }

    fn consume_recent_picker_shortcut_at(&self, now: Instant) -> bool {
        self.shortcut_latch
            .consume_if_recent_at(now, PICKER_SHORTCUT_LATCH_DURATION)
    }
}

impl PickerShortcutLatch {
    fn remember_now(&self) {
        self.remember_at(Instant::now());
    }

    fn remember_at(&self, seen_at: Instant) {
        match self.last_ctrl_shift_seen.lock() {
            Ok(mut last_seen) => {
                *last_seen = Some(seen_at);
            }
            Err(_) => eprintln!("Hops picker: shortcut latch lock was poisoned."),
        }
    }

    fn consume_if_recent_at(&self, now: Instant, max_age: Duration) -> bool {
        let mut last_seen = match self.last_ctrl_shift_seen.lock() {
            Ok(last_seen) => last_seen,
            Err(_) => {
                eprintln!("Hops picker: shortcut latch lock was poisoned.");
                return false;
            }
        };

        let Some(seen_at) = *last_seen else {
            return false;
        };

        *last_seen = None;
        now.checked_duration_since(seen_at)
            .is_none_or(|age| age <= max_age)
    }
}

fn clean_cli_value(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

#[cfg(target_os = "windows")]
fn is_ctrl_shift_picker_trigger_currently_active() -> bool {
    let ctrl_pressed = unsafe { GetAsyncKeyState(VK_CONTROL) } < 0;
    let shift_pressed = unsafe { GetAsyncKeyState(VK_SHIFT) } < 0;
    ctrl_pressed && shift_pressed
}

#[cfg(not(target_os = "windows"))]
fn is_ctrl_shift_picker_trigger_currently_active() -> bool {
    false
}

fn is_ctrl_shift_picker_trigger_active_with_latch<F>(
    state: &PickerState,
    mut is_currently_active: F,
    now: Instant,
) -> bool
where
    F: FnMut() -> bool,
{
    if is_currently_active() {
        return true;
    }

    state.consume_recent_picker_shortcut_at(now)
}

fn is_ctrl_shift_picker_trigger_active(state: &PickerState) -> bool {
    is_ctrl_shift_picker_trigger_active_with_latch(
        state,
        is_ctrl_shift_picker_trigger_currently_active,
        Instant::now(),
    )
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

#[cfg(target_os = "windows")]
struct PickerShortcutObserver {
    latch: Arc<PickerShortcutLatch>,
    ctrl_down: AtomicBool,
    shift_down: AtomicBool,
}

#[cfg(target_os = "windows")]
impl PickerShortcutObserver {
    fn new(latch: Arc<PickerShortcutLatch>) -> Self {
        Self {
            latch,
            ctrl_down: AtomicBool::new(false),
            shift_down: AtomicBool::new(false),
        }
    }

    fn handle_keyboard_message(&self, vk_code: i32, message: u32) {
        let is_down = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
        let is_up = matches!(message, WM_KEYUP | WM_SYSKEYUP);
        if !is_down && !is_up {
            return;
        }

        if is_ctrl_virtual_key(vk_code) {
            self.ctrl_down.store(is_down, Ordering::Relaxed);
        } else if is_shift_virtual_key(vk_code) {
            self.shift_down.store(is_down, Ordering::Relaxed);
        } else {
            return;
        }

        if is_down
            && self.ctrl_down.load(Ordering::Relaxed)
            && self.shift_down.load(Ordering::Relaxed)
        {
            self.latch.remember_now();
        }
    }
}

#[cfg(target_os = "windows")]
static PICKER_SHORTCUT_OBSERVER: OnceLock<Arc<PickerShortcutObserver>> = OnceLock::new();

#[cfg(target_os = "windows")]
fn is_ctrl_virtual_key(vk_code: i32) -> bool {
    matches!(vk_code, VK_CONTROL | VK_LCONTROL | VK_RCONTROL)
}

#[cfg(target_os = "windows")]
fn is_shift_virtual_key(vk_code: i32) -> bool {
    matches!(vk_code, VK_SHIFT | VK_LSHIFT | VK_RSHIFT)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn picker_keyboard_hook_proc(
    code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if code == HC_ACTION {
        if let Some(observer) = PICKER_SHORTCUT_OBSERVER.get() {
            let event = unsafe { &*(l_param as *const KeyboardHookEvent) };
            observer.handle_keyboard_message(event.vk_code as i32, w_param as u32);
        }
    }

    unsafe { CallNextHookEx(0, code, w_param, l_param) }
}

#[cfg(target_os = "windows")]
fn run_picker_shortcut_hook_message_loop() {
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    let hook =
        unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(picker_keyboard_hook_proc), module, 0) };

    if hook == 0 {
        eprintln!("Hops picker: could not install Ctrl+Shift shortcut observer.");
        return;
    }

    let mut message = WinMsg {
        hwnd: 0,
        message: 0,
        w_param: 0,
        l_param: 0,
        time: 0,
        point: WinPoint { x: 0, y: 0 },
        private: 0,
    };

    while unsafe { GetMessageW(&mut message, 0, 0, 0) } > 0 {}
}

#[cfg(target_os = "windows")]
pub(crate) fn install_picker_shortcut_observer(state: &PickerState) {
    let observer = Arc::new(PickerShortcutObserver::new(Arc::clone(
        &state.shortcut_latch,
    )));

    if PICKER_SHORTCUT_OBSERVER.set(observer).is_err() {
        return;
    }

    thread::spawn(run_picker_shortcut_hook_message_loop);
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn install_picker_shortcut_observer(_state: &PickerState) {}

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
        return;
    }

    let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == "main")
    else {
        eprintln!("Hops settings: main window configuration was not found.");
        return;
    };

    match WebviewWindowBuilder::from_config(app, config).and_then(|builder| builder.build()) {
        Ok(window) => {
            let _ = window.show();
            let _ = window.set_focus();
        }
        Err(error) => eprintln!("Hops settings: could not create settings window: {error}"),
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
        WebviewUrl::App("picker.html".into()),
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
    let mut running = RunningProcessSnapshot::current();
    build_picker_session_with_running_processes(
        config,
        url,
        source,
        reason,
        preferred_browser_id,
        preferred_private_mode,
        &mut running,
    )
}

fn build_picker_session_with_running_processes<F>(
    config: &AppConfig,
    url: &str,
    source: PickerLaunchSource,
    reason: &str,
    preferred_browser_id: Option<&str>,
    preferred_private_mode: bool,
    running: &mut RunningProcessSnapshot<F>,
) -> Result<PickerSession, String>
where
    F: FnMut() -> std::collections::HashSet<String>,
{
    let normalized_url = normalize_http_url(url)?.to_string();

    let mut browsers: Vec<PickerBrowserEntry> = config
        .browsers
        .iter()
        .filter(|browser| !browser.is_hidden)
        .map(|browser| PickerBrowserEntry {
            id: browser.id.clone(),
            name: browser.name.clone(),
            private_flag: browser.private_flag.clone(),
            icon_key: browser.icon_key.clone(),
            is_default: config
                .default_browser_id
                .as_ref()
                .is_some_and(|default_id| default_id == &browser.id),
            is_running: running.is_browser_running(browser),
        })
        .collect();
    browsers.sort_by(|left, right| {
        picker_browser_sort_key(left, preferred_browser_id)
            .cmp(&picker_browser_sort_key(right, preferred_browser_id))
    });

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

fn picker_browser_sort_key(
    browser: &PickerBrowserEntry,
    preferred_browser_id: Option<&str>,
) -> (u8, String) {
    let priority = if preferred_browser_id.is_some_and(|id| id == browser.id) {
        0
    } else if browser.is_default {
        1
    } else if browser.is_running {
        2
    } else {
        3
    };

    (priority, browser.name.to_lowercase())
}

fn load_config_with_picker_trigger_snapshot<F, L>(
    mut is_picker_trigger_active: F,
    mut load_config: L,
) -> Result<(AppConfig, bool), String>
where
    F: FnMut() -> bool,
    L: FnMut() -> Result<AppConfig, String>,
{
    let picker_trigger_active_before_config_load = is_picker_trigger_active();
    let config = load_config()?;
    let force_picker = picker_trigger_active_before_config_load || is_picker_trigger_active();

    Ok((config, force_picker))
}

fn resolve_incoming_url_decision_with_running_processes<F>(
    config: &AppConfig,
    url: &str,
    force_picker: bool,
    running: &mut RunningProcessSnapshot<F>,
) -> Result<RouteDecision, String>
where
    F: FnMut() -> std::collections::HashSet<String>,
{
    if force_picker {
        return Ok(picker_decision("ctrl_shift_click"));
    }

    resolve_route_with_running_processes(config, url, running)
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
    show_picker_window_with_session(app, state, session)
}

fn show_picker_window_with_running_processes<F>(
    app: &AppHandle,
    state: &PickerState,
    config: &AppConfig,
    url: &str,
    source: PickerLaunchSource,
    reason: &str,
    preferred_browser_id: Option<&str>,
    preferred_private_mode: bool,
    running: &mut RunningProcessSnapshot<F>,
) -> Result<(), String>
where
    F: FnMut() -> std::collections::HashSet<String>,
{
    cancel_picker_idle_destroy(state)?;

    let session = build_picker_session_with_running_processes(
        config,
        url,
        source,
        reason,
        preferred_browser_id,
        preferred_private_mode,
        running,
    )?;
    show_picker_window_with_session(app, state, session)
}

fn show_picker_window_with_session(
    app: &AppHandle,
    state: &PickerState,
    session: PickerSession,
) -> Result<(), String> {
    store_picker_session(state, session.clone())?;

    let window = ensure_picker_window(app)?;
    let menu_height = picker_window_height(session.browsers.len());
    window
        .set_size(Size::Logical(LogicalSize::new(
            PICKER_MENU_WIDTH as f64,
            menu_height as f64,
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
    let (config, force_picker) = load_config_with_picker_trigger_snapshot(
        || is_ctrl_shift_picker_trigger_active(state),
        || load_or_init_config(app, false),
    )?;
    let mut running = RunningProcessSnapshot::current();
    let decision = resolve_incoming_url_decision_with_running_processes(
        &config,
        url,
        force_picker,
        &mut running,
    )?;

    if decision.action == RouteAction::ShowPicker && decision.reason == "ctrl_shift_click" {
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

    if decision.action == RouteAction::OpenBrowser {
        hide_picker_window_internal(app, state)?;
        if let Some(browser_id) = decision.browser_id.as_deref() {
            open_url_with_browser(&config, browser_id, url, decision.private_mode)?;
        }
    } else {
        show_picker_window_with_running_processes(
            app,
            state,
            &config,
            url,
            PickerLaunchSource::Route,
            &decision.reason,
            decision.browser_id.as_deref(),
            decision.private_mode,
            &mut running,
        )?;
    }

    Ok(decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        BrowserConfig, BrowserSource, RuleConfig, RulePatternType, ThemePreference,
    };
    use std::cell::Cell;
    use std::collections::HashSet;
    use std::rc::Rc;

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
            icon_key: Some(id.to_string()),
            manual_trust: None,
            source: BrowserSource::Manual,
            is_hidden: false,
        }
    }

    fn test_rule(pattern_type: RulePatternType, pattern: &str, browser_id: &str) -> RuleConfig {
        RuleConfig {
            id: "test".to_string(),
            pattern: pattern.to_string(),
            pattern_type,
            browser_id: browser_id.to_string(),
            private_mode: false,
            enabled: true,
        }
    }

    fn counted_empty_snapshot() -> (
        RunningProcessSnapshot<impl FnMut() -> HashSet<String>>,
        Rc<Cell<usize>>,
    ) {
        let scan_count = Rc::new(Cell::new(0));
        let scan_count_for_load = Rc::clone(&scan_count);

        let snapshot = RunningProcessSnapshot::new(move || {
            scan_count_for_load.set(scan_count_for_load.get() + 1);
            HashSet::new()
        });

        (snapshot, scan_count)
    }

    fn running_snapshot(
        running_paths: HashSet<String>,
    ) -> RunningProcessSnapshot<impl FnMut() -> HashSet<String>> {
        RunningProcessSnapshot::new(move || {
            running_paths
                .iter()
                .map(|path| path.trim().replace('/', "\\").to_lowercase())
                .collect()
        })
    }

    fn build_route_picker_session<F>(
        config: &AppConfig,
        url: &str,
        decision: &RouteDecision,
        running: &mut RunningProcessSnapshot<F>,
    ) -> PickerSession
    where
        F: FnMut() -> HashSet<String>,
    {
        build_picker_session_with_running_processes(
            config,
            url,
            PickerLaunchSource::Route,
            &decision.reason,
            decision.browser_id.as_deref(),
            decision.private_mode,
            running,
        )
        .expect("picker session should build")
    }

    #[test]
    fn picker_trigger_is_checked_before_config_load() {
        let events = Rc::new(std::cell::RefCell::new(Vec::new()));
        let trigger_events = Rc::clone(&events);
        let load_events = Rc::clone(&events);

        let (config, force_picker) = load_config_with_picker_trigger_snapshot(
            || {
                trigger_events.borrow_mut().push("trigger");
                false
            },
            || {
                load_events.borrow_mut().push("config");
                Ok(test_app_config(vec![manual_browser("chrome", "Chrome")]))
            },
        )
        .expect("config should load");

        assert_eq!(config.browsers.len(), 1);
        assert!(!force_picker);
        assert_eq!(events.borrow().as_slice(), ["trigger", "config", "trigger"]);
    }

    #[test]
    fn captured_picker_trigger_forces_picker_before_default_fallback() {
        let default = manual_browser("chrome", "Chrome");
        let mut config = test_app_config(vec![default.clone()]);
        config.default_browser_id = Some(default.id);
        config.use_defaults_when_not_running = true;
        let (mut running, scan_count) = counted_empty_snapshot();

        let decision = resolve_incoming_url_decision_with_running_processes(
            &config,
            "https://example.com",
            true,
            &mut running,
        )
        .expect("route should resolve");

        assert_eq!(decision.action, RouteAction::ShowPicker);
        assert_eq!(decision.reason, "ctrl_shift_click");
        assert_eq!(scan_count.get(), 0);
    }

    #[test]
    fn recent_picker_shortcut_latch_forces_picker_after_current_keys_are_released() {
        let state = PickerState::default();
        let now = std::time::Instant::now();
        state.remember_picker_shortcut_at(now);

        assert!(is_ctrl_shift_picker_trigger_active_with_latch(
            &state,
            || false,
            now + Duration::from_millis(500)
        ));
    }

    #[test]
    fn expired_picker_shortcut_latch_does_not_force_picker() {
        let state = PickerState::default();
        let now = std::time::Instant::now();
        state.remember_picker_shortcut_at(now);

        assert!(!is_ctrl_shift_picker_trigger_active_with_latch(
            &state,
            || false,
            now + Duration::from_millis(2_001)
        ));
    }

    #[test]
    fn consumed_picker_shortcut_latch_does_not_force_second_url() {
        let state = PickerState::default();
        let now = std::time::Instant::now();
        state.remember_picker_shortcut_at(now);

        assert!(is_ctrl_shift_picker_trigger_active_with_latch(
            &state,
            || false,
            now + Duration::from_millis(500)
        ));
        assert!(!is_ctrl_shift_picker_trigger_active_with_latch(
            &state,
            || false,
            now + Duration::from_millis(600)
        ));
    }

    #[test]
    fn non_shortcut_keeps_default_fallback_behavior() {
        let default = manual_browser("chrome", "Chrome");
        let mut config = test_app_config(vec![default.clone()]);
        config.default_browser_id = Some(default.id);
        config.use_defaults_when_not_running = true;
        let (mut running, scan_count) = counted_empty_snapshot();

        let decision = resolve_incoming_url_decision_with_running_processes(
            &config,
            "https://example.com",
            false,
            &mut running,
        )
        .expect("route should resolve");

        assert_eq!(decision.action, RouteAction::OpenBrowser);
        assert_eq!(decision.reason, "default_browser");
        assert_eq!(scan_count.get(), 0);
    }

    #[test]
    fn matched_non_running_rule_reuses_route_snapshot_for_picker_badges() {
        let browser = manual_browser("brave", "Brave");
        let mut config = test_app_config(vec![browser.clone()]);
        config.rules.push(test_rule(
            RulePatternType::Hostname,
            "github.com",
            &browser.id,
        ));
        let (mut running, scan_count) = counted_empty_snapshot();

        let decision = resolve_route_with_running_processes(
            &config,
            "https://github.com/openai/codex",
            &mut running,
        )
        .expect("route should resolve");
        assert_eq!(decision.action, RouteAction::ShowPicker);
        assert_eq!(decision.reason, "rule_browser_not_running");
        assert_eq!(scan_count.get(), 1);

        let session = build_route_picker_session(
            &config,
            "https://github.com/openai/codex",
            &decision,
            &mut running,
        );

        assert_eq!(session.browsers.len(), 1);
        assert!(!session.browsers[0].is_running);
        assert_eq!(session.browsers[0].icon_key.as_deref(), Some("brave"));
        assert_eq!(scan_count.get(), 1);
    }

    #[test]
    fn always_show_picker_scans_only_when_picker_badges_are_built() {
        let browser = manual_browser("chrome", "Chrome");
        let mut config = test_app_config(vec![browser]);
        config.always_show_picker = true;
        let (mut running, scan_count) = counted_empty_snapshot();

        let decision =
            resolve_route_with_running_processes(&config, "https://example.com", &mut running)
                .expect("route should resolve");

        assert_eq!(decision.reason, "always_show_picker");
        assert_eq!(scan_count.get(), 0);

        let session =
            build_route_picker_session(&config, "https://example.com", &decision, &mut running);

        assert_eq!(session.browsers.len(), 1);
        assert_eq!(scan_count.get(), 1);
    }

    #[test]
    fn no_match_scans_only_when_picker_badges_are_built() {
        let browser = manual_browser("chrome", "Chrome");
        let mut config = test_app_config(vec![browser.clone()]);
        config.rules.push(test_rule(
            RulePatternType::Hostname,
            "github.com",
            &browser.id,
        ));
        let (mut running, scan_count) = counted_empty_snapshot();

        let decision =
            resolve_route_with_running_processes(&config, "https://example.com", &mut running)
                .expect("route should resolve");

        assert_eq!(decision.reason, "no_match");
        assert_eq!(scan_count.get(), 0);

        let session =
            build_route_picker_session(&config, "https://example.com", &decision, &mut running);

        assert_eq!(session.browsers.len(), 1);
        assert_eq!(scan_count.get(), 1);
    }

    #[test]
    fn preferred_browser_sorts_before_default_and_running_browsers() {
        let preferred = manual_browser("brave", "Brave");
        let default = manual_browser("chrome", "Chrome");
        let running_browser = manual_browser("firefox", "Firefox");
        let mut config = test_app_config(vec![
            default.clone(),
            running_browser.clone(),
            preferred.clone(),
        ]);
        config.default_browser_id = Some(default.id.clone());
        let mut running = running_snapshot(HashSet::from([running_browser.path.clone()]));

        let session = build_picker_session_with_running_processes(
            &config,
            "https://example.com",
            PickerLaunchSource::Route,
            "test",
            Some(preferred.id.as_str()),
            false,
            &mut running,
        )
        .expect("picker session should build");

        let browser_ids: Vec<&str> = session
            .browsers
            .iter()
            .map(|browser| browser.id.as_str())
            .collect();
        assert_eq!(browser_ids, vec!["brave", "chrome", "firefox"]);
    }

    #[test]
    fn default_browser_sorts_before_running_when_no_preferred_browser_exists() {
        let running_browser = manual_browser("brave", "Brave");
        let default = manual_browser("chrome", "Chrome");
        let other = manual_browser("firefox", "Firefox");
        let mut config = test_app_config(vec![
            other.clone(),
            running_browser.clone(),
            default.clone(),
        ]);
        config.default_browser_id = Some(default.id.clone());
        let mut running = running_snapshot(HashSet::from([running_browser.path.clone()]));

        let session = build_picker_session_with_running_processes(
            &config,
            "https://example.com",
            PickerLaunchSource::Manual,
            "test",
            None,
            false,
            &mut running,
        )
        .expect("picker session should build");

        let browser_ids: Vec<&str> = session
            .browsers
            .iter()
            .map(|browser| browser.id.as_str())
            .collect();
        assert_eq!(browser_ids, vec!["chrome", "brave", "firefox"]);
    }

    #[test]
    fn running_browsers_sort_before_other_non_default_browsers() {
        let other_a = manual_browser("arc", "Arc");
        let running_browser = manual_browser("vivaldi", "Vivaldi");
        let other_b = manual_browser("zen", "Zen");
        let config = test_app_config(vec![
            other_b.clone(),
            other_a.clone(),
            running_browser.clone(),
        ]);
        let mut running = running_snapshot(HashSet::from([running_browser.path.clone()]));

        let session = build_picker_session_with_running_processes(
            &config,
            "https://example.com",
            PickerLaunchSource::Manual,
            "test",
            None,
            false,
            &mut running,
        )
        .expect("picker session should build");

        let browser_ids: Vec<&str> = session
            .browsers
            .iter()
            .map(|browser| browser.id.as_str())
            .collect();
        assert_eq!(browser_ids, vec!["vivaldi", "arc", "zen"]);
    }

    #[test]
    fn picker_browser_order_falls_back_to_browser_name() {
        let config = test_app_config(vec![
            manual_browser("zen", "Zen"),
            manual_browser("brave", "Brave"),
            manual_browser("arc", "Arc"),
        ]);
        let mut running = running_snapshot(HashSet::new());

        let session = build_picker_session_with_running_processes(
            &config,
            "https://example.com",
            PickerLaunchSource::Manual,
            "test",
            None,
            false,
            &mut running,
        )
        .expect("picker session should build");

        let browser_ids: Vec<&str> = session
            .browsers
            .iter()
            .map(|browser| browser.id.as_str())
            .collect();
        assert_eq!(browser_ids, vec!["arc", "brave", "zen"]);
    }
}
