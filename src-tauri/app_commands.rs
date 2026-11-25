// Shared by `build.rs` and the capability tests in `src/capability_tests.rs`
// through `include!`, so both always read the same list.
//
// Listing the app commands here makes Tauri generate an `allow-<command>`
// permission for each one and deny any command a window's capability does not
// grant. Without this list, every command is allowed in every window. A command
// added to `invoke_handler` must also be added here and to a capability.
pub(crate) const APP_COMMANDS: &[&str] = &[
    "load_config",
    "take_config_recovery_notice",
    "get_config_file_path",
    "open_config_folder",
    "save_config",
    "validate_manual_browser",
    "validate_regex_patterns",
    "refresh_browsers",
    "list_running_browser_ids",
    "list_missing_browser_ids",
    "preview_route_with_config",
    "route_and_open_with_config",
    "open_url",
    "get_picker_state",
    "show_picker_for_url",
    "hide_picker_window",
    "show_settings_window_command",
    "open_windows_default_apps",
    "get_browser_registration_status",
    "register_hops_as_browser",
    "unregister_hops_as_browser",
    "reset_config",
];
