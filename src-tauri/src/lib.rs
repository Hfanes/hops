use tauri::{Manager, WindowEvent};

mod browsers;
mod commands;
mod config;
mod models;
mod picker;
mod registration;
mod routing;
mod tray;

use config::load_or_init_config;
use picker::{
    extract_url_from_args, handle_incoming_url, hide_picker_window_internal, hide_settings_window,
    show_settings_window, PickerState,
};
use tray::setup_tray;

const CONFIG_FILENAME: &str = "config.json";
const HOPS_APP_NAME: &str = "Hops";
const HOPS_PROTOCOL_PROG_ID: &str = "HopsURL";
const HOPS_HTML_PROG_ID: &str = "HopsHTML";
const HOPS_CUSTOM_URI_SCHEME: &str = "Hops";
const PICKER_WINDOW_LABEL: &str = "picker";
const PICKER_SESSION_EVENT: &str = "picker-session";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const PICKER_MENU_WIDTH: u32 = 280;
const PICKER_MENU_MIN_HEIGHT: u32 = 128;
const PICKER_MENU_MAX_HEIGHT: u32 = 340;
const PICKER_MENU_ROW_HEIGHT: u32 = 46;
const PICKER_MENU_CHROME_HEIGHT: u32 = 92;
const PICKER_CURSOR_OFFSET_X: i32 = 6;
const PICKER_CURSOR_OFFSET_Y: i32 = 10;
const PICKER_IDLE_DESTROY_SECONDS: u64 = 15;

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
        .plugin(tauri_plugin_process::init())
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
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

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
            commands::load_config,
            commands::get_config_file_path,
            commands::save_config,
            commands::validate_manual_browser,
            commands::refresh_browsers,
            commands::list_running_browser_ids,
            commands::preview_route,
            commands::preview_route_with_config,
            commands::route_and_open,
            commands::route_and_open_with_config,
            commands::open_url,
            commands::get_picker_state,
            commands::show_picker_for_url,
            commands::hide_picker_window,
            commands::show_settings_window_command,
            commands::open_windows_default_apps,
            commands::get_browser_registration_status,
            commands::register_hops_as_browser,
            commands::unregister_hops_as_browser,
            commands::reset_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
