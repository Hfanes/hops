//! Builders and temp files shared by the unit tests of several modules.

use crate::models::{AppConfig, BrowserConfig, BrowserSource, ThemePreference};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// A config with onboarding done, no rules and everything else off.
pub(crate) fn test_app_config(browsers: Vec<BrowserConfig>) -> AppConfig {
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

/// An unconfirmed manual browser at `C:\Tools\{name}\browser.exe`, whose icon
/// key is its ID.
pub(crate) fn manual_browser(id: &str, name: &str) -> BrowserConfig {
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

/// A bare manual browser at `path`, for tests that only care about the path.
pub(crate) fn browser_at(path: &str) -> BrowserConfig {
    BrowserConfig {
        id: "test".to_string(),
        name: "Test".to_string(),
        path: path.to_string(),
        private_flag: None,
        icon_key: None,
        manual_trust: None,
        source: BrowserSource::Manual,
        is_hidden: false,
    }
}

/// A fresh directory under the system temp folder, deleted with everything in
/// it when dropped, so a failing assertion does not leave files behind.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn new(label: &str) -> Self {
        // Tests run in parallel, so the time alone is not unique enough.
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "hops-{label}-{}-{nanos}-{count}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp dir should create");
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Creates an empty file named `name` in the directory and returns its path.
    pub(crate) fn empty_file(&self, name: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, []).expect("temp file should write");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// An empty placeholder executable named `name`, and the directory that
/// owns it. Keep the directory alive for as long as the file is needed.
pub(crate) fn temp_executable(name: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new("browser-test");
    let path = dir.empty_file(name);
    (dir, path)
}
