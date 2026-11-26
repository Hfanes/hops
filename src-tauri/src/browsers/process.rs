//! Which configured browsers are running, from a Win32 process snapshot.

use super::normalize_path;
use crate::models::BrowserConfig;
use std::collections::HashSet;

#[cfg(target_os = "windows")]
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
/// Lowercased executable file names (e.g. `chrome.exe`) of the given browsers.
fn browser_executable_names(browsers: &[BrowserConfig]) -> HashSet<String> {
    browsers
        .iter()
        .filter_map(|browser| {
            normalize_path(&browser.path)
                .rsplit('\\')
                .next()
                .filter(|name| !name.is_empty())
                .map(str::to_string)
        })
        .collect()
}

/// Normalized full paths of running processes that could be one of `browsers`.
///
/// The process snapshot already carries each executable's file name, so only
/// processes whose name matches a configured browser are opened to read their
/// full path. That turns hundreds of `OpenProcess` calls into a handful.
#[cfg(target_os = "windows")]
pub(crate) fn running_processes(browsers: &[BrowserConfig]) -> Result<HashSet<String>, String> {
    let wanted_names = browser_executable_names(browsers);
    if wanted_names.is_empty() {
        return Ok(HashSet::new());
    }

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(
            "Could not create process snapshot while checking running browsers.".to_string(),
        );
    }
    // Closed on drop, whichever way this function returns.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(snapshot) };

    let mut paths = HashSet::new();
    let mut path_buffer = vec![0u16; 32768];
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..unsafe { std::mem::zeroed() }
    };

    let mut has_entry = unsafe { Process32FirstW(snapshot.as_raw_handle(), &mut entry) != 0 };
    while has_entry {
        if process_entry_name_matches(&entry.szExeFile, &wanted_names) {
            if let Some(path) = query_process_image_path(entry.th32ProcessID, &mut path_buffer) {
                paths.insert(normalize_path(&path));
            }
        }

        has_entry = unsafe { Process32NextW(snapshot.as_raw_handle(), &mut entry) != 0 };
    }

    Ok(paths)
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
/// Whether a null-terminated UTF-16 executable name is in `wanted_names`
/// (which holds lowercased names).
fn process_entry_name_matches(exe_file: &[u16], wanted_names: &HashSet<String>) -> bool {
    let length = exe_file
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(exe_file.len());
    String::from_utf16(&exe_file[..length])
        .is_ok_and(|name| wanted_names.contains(&name.to_lowercase()))
}

/// Hops only supports Windows; elsewhere no browser is reported as running.
#[cfg(not(target_os = "windows"))]
pub(crate) fn running_processes(_browsers: &[BrowserConfig]) -> Result<HashSet<String>, String> {
    Ok(HashSet::new())
}

#[cfg(target_os = "windows")]
fn query_process_image_path(process_id: u32, buffer: &mut [u16]) -> Option<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if handle.is_null() {
        return None;
    }
    let handle = unsafe { OwnedHandle::from_raw_handle(handle) };

    let mut length = buffer.len() as u32;
    let success = unsafe {
        QueryFullProcessImageNameW(handle.as_raw_handle(), 0, buffer.as_mut_ptr(), &mut length) != 0
    };

    if !success || length == 0 {
        return None;
    }

    String::from_utf16(&buffer[..length as usize]).ok()
}

pub(crate) fn is_browser_running(
    browser: &BrowserConfig,
    running_processes: &HashSet<String>,
) -> bool {
    let normalized_path = normalize_path(&browser.path);
    running_processes.contains(&normalized_path)
}

pub(crate) struct RunningProcessSnapshot<F>
where
    F: FnMut() -> HashSet<String>,
{
    running: Option<HashSet<String>>,
    load: F,
}

/// A lazy snapshot that scans only for the given browsers' executables, and
/// only the first time a running state is actually needed.
pub(crate) fn current_running_processes(
    browsers: &[BrowserConfig],
) -> RunningProcessSnapshot<impl FnMut() -> HashSet<String> + '_> {
    RunningProcessSnapshot::new(move || running_processes(browsers).unwrap_or_default())
}

impl<F> RunningProcessSnapshot<F>
where
    F: FnMut() -> HashSet<String>,
{
    pub(crate) fn new(load: F) -> Self {
        Self {
            running: None,
            load,
        }
    }

    pub(crate) fn is_browser_running(&mut self, browser: &BrowserConfig) -> bool {
        let running = self.running.get_or_insert_with(|| (self.load)());
        is_browser_running(browser, running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::browser_at;

    fn utf16_exe_name(name: &str) -> [u16; 260] {
        let mut buffer = [0u16; 260];
        for (slot, unit) in buffer.iter_mut().zip(name.encode_utf16()) {
            *slot = unit;
        }
        buffer
    }

    #[test]
    fn browser_executable_names_are_lowercased_file_names() {
        let names = browser_executable_names(&[
            browser_at("C:\\Program Files\\Google\\Chrome\\Application\\Chrome.EXE"),
            browser_at("  C:/Tools/Firefox/firefox.exe  "),
            browser_at(""),
        ]);

        assert_eq!(
            names,
            HashSet::from(["chrome.exe".to_string(), "firefox.exe".to_string()])
        );
    }

    #[test]
    fn process_entry_name_matching_is_case_insensitive_and_stops_at_null() {
        let wanted = HashSet::from(["chrome.exe".to_string()]);

        assert!(process_entry_name_matches(
            &utf16_exe_name("CHROME.exe"),
            &wanted
        ));
        assert!(!process_entry_name_matches(
            &utf16_exe_name("chrome.exe.bak"),
            &wanted
        ));
        assert!(!process_entry_name_matches(
            &utf16_exe_name("svchost.exe"),
            &wanted
        ));
    }

    #[test]
    fn running_processes_skips_scan_without_browsers() {
        assert!(running_processes(&[])
            .expect("empty scan should succeed")
            .is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn running_processes_finds_a_live_process_by_full_path() {
        // The test binary itself is a running process we know the path of.
        let current_exe = std::env::current_exe().expect("current exe should resolve");
        let current_path = current_exe.to_string_lossy().to_string();

        let running = running_processes(&[browser_at(&current_path)])
            .expect("process snapshot should succeed");

        assert!(is_browser_running(&browser_at(&current_path), &running));
        // Only processes sharing the wanted executable name are resolved.
        let wanted = browser_executable_names(&[browser_at(&current_path)]);
        assert!(running.iter().all(|path| wanted
            .iter()
            .any(|name| path.ends_with(&format!("\\{name}")))));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn a_path_with_surrounding_whitespace_is_still_reported_as_running() {
        let current_exe = std::env::current_exe().expect("current exe should resolve");
        let padded_path = format!("  {}\t", current_exe.to_string_lossy());

        let running = running_processes(&[browser_at(&padded_path)])
            .expect("process snapshot should succeed");

        assert!(is_browser_running(&browser_at(&padded_path), &running));
    }
}
