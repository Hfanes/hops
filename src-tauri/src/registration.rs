use crate::models::BrowserRegistrationStatus;
#[cfg(target_os = "windows")]
use crate::CREATE_NO_WINDOW;
use crate::{HOPS_APP_NAME, HOPS_PROTOCOL_PROG_ID};

#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{S_FALSE, S_OK};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{AssocQueryStringW, ASSOCF_NONE, ASSOCSTR_PROGID};

pub(crate) fn open_windows_default_apps() -> Result<(), String> {
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

pub(crate) fn browser_registration_status() -> Result<BrowserRegistrationStatus, String> {
    #[cfg(target_os = "windows")]
    {
        browser_registration_status_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Browser registration status is only available on Windows.".to_string())
    }
}

pub(crate) fn register_hops_as_browser() -> Result<BrowserRegistrationStatus, String> {
    #[cfg(target_os = "windows")]
    {
        register_hops_as_browser_windows()?;
        notify_file_associations_changed();
        browser_registration_status_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Registering Hops as a browser is only supported on Windows.".to_string())
    }
}

pub(crate) fn unregister_hops_as_browser() -> Result<BrowserRegistrationStatus, String> {
    #[cfg(target_os = "windows")]
    {
        unregister_hops_as_browser_windows()?;
        notify_file_associations_changed();
        browser_registration_status_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Unregistering Hops as a browser is only supported on Windows.".to_string())
    }
}

/// Tells Windows that file and URL associations changed, so Settings > Default
/// apps and Explorer show Hops' new registration state without a sign-out.
#[cfg(target_os = "windows")]
fn notify_file_associations_changed() {
    use windows_sys::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};

    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as _,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
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
    if first_result != S_FALSE || length == 0 {
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

    if second_result != S_OK || length == 0 {
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

/// `HKCU` keys that belong to Hops. Unregistering deletes them, and so does
/// the uninstaller (`src-tauri/windows/hooks.nsh`).
const OWNED_KEYS: [&str; 2] = ["Software\\Classes\\HopsURL", "Software\\Hops"];

/// `HKCU` keys written by Hops 0.1.6 and earlier that are no longer used: the
/// `.htm`/`.html` association (its command could not open files) and the
/// `hops:` custom scheme (nothing handled it, and any web page could launch
/// Hops through it). Registering, unregistering and installing delete them.
const LEGACY_KEYS: [&str; 3] = [
    "Software\\Classes\\HopsHTML",
    "Software\\Classes\\Hops",
    "Software\\Hops\\Capabilities\\FileAssociations",
];

const CAPABILITIES_KEY: &str = "Software\\Hops\\Capabilities";
const REGISTERED_APPLICATIONS_KEY: &str = "Software\\RegisteredApplications";

/// One registry value under `HKCU`. An empty `name` is the key's default value.
#[derive(Debug, PartialEq, Eq)]
struct RegistryValue {
    key: String,
    name: &'static str,
    data: String,
}

impl RegistryValue {
    fn new(key: impl Into<String>, name: &'static str, data: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            name,
            data: data.into(),
        }
    }
}

/// Everything "Register Hops" writes: the `http`/`https` handler class and the
/// capabilities that list Hops in Windows Default Apps.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn registration_values(executable: &str) -> Vec<RegistryValue> {
    let url_class = format!("Software\\Classes\\{HOPS_PROTOCOL_PROG_ID}");
    let url_associations = format!("{CAPABILITIES_KEY}\\URLAssociations");

    vec![
        RegistryValue::new(&url_class, "", "Hops URL Handler"),
        RegistryValue::new(&url_class, "URL Protocol", ""),
        RegistryValue::new(
            format!("{url_class}\\DefaultIcon"),
            "",
            format!("\"{executable}\",0"),
        ),
        RegistryValue::new(
            format!("{url_class}\\shell\\open\\command"),
            "",
            format!("\"{executable}\" --url \"%1\""),
        ),
        RegistryValue::new(CAPABILITIES_KEY, "ApplicationName", HOPS_APP_NAME),
        RegistryValue::new(
            CAPABILITIES_KEY,
            "ApplicationDescription",
            "Hops routes external links to the right browser.",
        ),
        RegistryValue::new(&url_associations, "http", HOPS_PROTOCOL_PROG_ID),
        RegistryValue::new(&url_associations, "https", HOPS_PROTOCOL_PROG_ID),
        RegistryValue::new(REGISTERED_APPLICATIONS_KEY, HOPS_APP_NAME, CAPABILITIES_KEY),
    ]
}

#[cfg(target_os = "windows")]
fn delete_keys(hkcu: &RegKey, keys: &[&str]) {
    for key in keys {
        // A missing key is already the state we want.
        let _ = hkcu.delete_subkey_all(key);
    }
}

#[cfg(target_os = "windows")]
fn register_hops_as_browser_windows() -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("Could not resolve executable path: {error}"))?;
    let executable = executable.to_string_lossy();

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    delete_keys(&hkcu, &LEGACY_KEYS);

    for value in registration_values(&executable) {
        let (key, _) = hkcu
            .create_subkey(&value.key)
            .map_err(|error| format!("Could not create HKCU\\{}: {error}", value.key))?;
        key.set_value(value.name, &value.data).map_err(|error| {
            format!(
                "Could not write '{}' in HKCU\\{}: {error}",
                value.name, value.key
            )
        })?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn unregister_hops_as_browser_windows() -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    if let Ok(registered_apps) = hkcu.open_subkey_with_flags(REGISTERED_APPLICATIONS_KEY, KEY_WRITE)
    {
        let _ = registered_apps.delete_value(HOPS_APP_NAME);
    }

    delete_keys(&hkcu, &OWNED_KEYS);
    delete_keys(&hkcu, &LEGACY_KEYS);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXECUTABLE: &str = "C:\\Users\\Test\\AppData\\Local\\hops\\hops.exe";

    fn is_under(key: &str, root: &str) -> bool {
        let key = key.to_ascii_lowercase();
        let root = root.to_ascii_lowercase();
        key == root || key.starts_with(&format!("{root}\\"))
    }

    #[test]
    fn registration_only_writes_keys_hops_owns() {
        for value in registration_values(EXECUTABLE) {
            let owned = OWNED_KEYS.iter().any(|root| is_under(&value.key, root))
                || value.key == REGISTERED_APPLICATIONS_KEY;
            assert!(owned, "unexpected key HKCU\\{}", value.key);
            assert!(
                !LEGACY_KEYS.iter().any(|root| is_under(&value.key, root)),
                "registration writes legacy key HKCU\\{}",
                value.key
            );
        }
    }

    #[test]
    fn registration_has_no_custom_scheme_or_file_association() {
        let values = registration_values(EXECUTABLE);

        let url_protocol_keys: Vec<_> = values
            .iter()
            .filter(|value| value.name == "URL Protocol")
            .map(|value| value.key.as_str())
            .collect();
        assert_eq!(url_protocol_keys, ["Software\\Classes\\HopsURL"]);

        assert!(values
            .iter()
            .all(|value| !value.key.contains("FileAssociations")));
    }

    #[test]
    fn uninstaller_hook_deletes_every_key_hops_writes() {
        let hooks = include_str!("../windows/hooks.nsh");

        for key in OWNED_KEYS.iter().chain(LEGACY_KEYS.iter()) {
            let line = format!("DeleteRegKey HKCU \"{key}\"");
            assert!(hooks.contains(&line), "hooks.nsh is missing: {line}");
        }

        let line =
            format!("DeleteRegValue HKCU \"{REGISTERED_APPLICATIONS_KEY}\" \"{HOPS_APP_NAME}\"");
        assert!(hooks.contains(&line), "hooks.nsh is missing: {line}");
    }

    #[test]
    fn registration_command_passes_the_url_argument() {
        let values = registration_values(EXECUTABLE);
        let command = values
            .iter()
            .find(|value| value.key.ends_with("\\shell\\open\\command"))
            .expect("command value");
        assert_eq!(command.data, format!("\"{EXECUTABLE}\" --url \"%1\""));
    }
}
