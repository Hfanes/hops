use crate::models::BrowserRegistrationStatus;
#[cfg(target_os = "windows")]
use crate::CREATE_NO_WINDOW;
use crate::{HOPS_APP_NAME, HOPS_CUSTOM_URI_SCHEME, HOPS_HTML_PROG_ID, HOPS_PROTOCOL_PROG_ID};

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
        browser_registration_status_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Unregistering Hops as a browser is only supported on Windows.".to_string())
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
