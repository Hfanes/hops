use crate::picker::show_settings_window;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

pub(crate) fn setup_tray(app: &AppHandle) -> Result<(), String> {
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
