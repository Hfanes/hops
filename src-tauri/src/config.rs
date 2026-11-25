use crate::browsers::{
    detect_browsers, hydrate_browser_icon_keys, hydrate_detected_browser_defaults,
    merge_detected_browsers, normalize_manual_browser_entries, validate_browser_entry,
};
use crate::models::{AppConfig, BrowserConfig, RulePatternType, ThemePreference};
use crate::routing::regex_pattern_error;
use crate::CONFIG_FILENAME;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

pub(crate) fn config_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    if let Ok(app_data_roaming) = std::env::var("APPDATA") {
        let app_data = PathBuf::from(app_data_roaming).join("Hops");
        fs::create_dir_all(&app_data).map_err(|error| {
            format!(
                "Could not create app data directory {:?}: {error}",
                app_data
            )
        })?;
        return Ok(app_data.join(CONFIG_FILENAME));
    }

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve app data directory: {error}"))?;

    fs::create_dir_all(&app_data).map_err(|error| {
        format!(
            "Could not create app data directory {:?}: {error}",
            app_data
        )
    })?;

    Ok(app_data.join(CONFIG_FILENAME))
}

/// In-memory copy of `config.json`, so handling a URL does not re-read,
/// re-parse and re-normalize the file on every command.
///
/// Every use compares the file's modified time and size with the cached ones,
/// a single cheap `stat`, so hand edits to `config.json` are still picked up
/// without restarting Hops. The mutex also serializes config reads and writes
/// across commands running on different threads.
#[derive(Default)]
pub(crate) struct ConfigCache {
    state: Mutex<CachedConfigState>,
}

#[derive(Default)]
struct CachedConfigState {
    path: Option<PathBuf>,
    entry: Option<CachedConfig>,
    /// Set when an invalid `config.json` was replaced, until the settings
    /// window reads it once.
    recovery_notice: Option<String>,
}

/// What happened when an invalid `config.json` was backed up and replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfigRecovery {
    backup_path: PathBuf,
    error: String,
}

impl ConfigRecovery {
    fn backup_file_name(&self) -> String {
        self.backup_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| self.backup_path.display().to_string())
    }

    fn notice(&self) -> String {
        format!(
            "config.json could not be read ({}), so Hops started over with default settings. \
             The old file was kept as {}.",
            self.error,
            self.backup_path.display()
        )
    }
}

struct CachedConfig {
    config: AppConfig,
    stamp: FileStamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    modified: SystemTime,
    len: u64,
}

impl FileStamp {
    fn of(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        Some(Self {
            modified: metadata.modified().ok()?,
            len: metadata.len(),
        })
    }
}

impl CachedConfigState {
    fn path(&mut self, app: &AppHandle) -> Result<PathBuf, String> {
        if let Some(path) = &self.path {
            return Ok(path.clone());
        }

        let path = config_file_path(app)?;
        self.path = Some(path.clone());
        Ok(path)
    }

    /// Returns the config, and the recovery when this call had to replace an
    /// invalid file.
    fn load(
        &mut self,
        path: &Path,
        auto_populate_browsers: bool,
    ) -> Result<(AppConfig, Option<ConfigRecovery>), String> {
        if let Some(entry) = &self.entry {
            let needs_browser_detection =
                auto_populate_browsers && entry.config.browsers.is_empty();
            if !needs_browser_detection && FileStamp::of(path) == Some(entry.stamp) {
                return Ok((entry.config.clone(), None));
            }
        }

        let (config, recovery) = load_or_init_config_from_path(path, auto_populate_browsers)?;
        self.remember(path, &config);
        if let Some(recovery) = &recovery {
            self.recovery_notice = Some(recovery.notice());
        }
        Ok((config, recovery))
    }

    fn save(&mut self, path: &Path, config: &AppConfig) -> Result<(), String> {
        let result = write_config_file(path, config);
        match result {
            Ok(()) => self.remember(path, config),
            // The file may or may not have changed; read it fresh next time.
            Err(_) => self.entry = None,
        }
        result
    }

    fn remember(&mut self, path: &Path, config: &AppConfig) {
        self.entry = FileStamp::of(path).map(|stamp| CachedConfig {
            config: config.clone(),
            stamp,
        });
    }
}

fn lock_config_cache(app: &AppHandle) -> MutexGuard<'_, CachedConfigState> {
    app.state::<ConfigCache>()
        .inner()
        .state
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn load_or_init_config(
    app: &AppHandle,
    auto_populate_browsers: bool,
) -> Result<AppConfig, String> {
    let (config, recovery) = {
        let mut cache = lock_config_cache(app);
        let path = cache.path(app)?;
        cache.load(&path, auto_populate_browsers)?
    };

    if let Some(recovery) = recovery {
        notify_config_recovery(app, &recovery);
    }

    Ok(config)
}

/// The one-time notice about a replaced invalid config, if there is one.
/// Reading it clears it, so the settings window shows it once.
pub(crate) fn take_config_recovery_notice(app: &AppHandle) -> Option<String> {
    lock_config_cache(app).recovery_notice.take()
}

/// Links can be routed with no window open, so the recovery is also announced
/// with a Windows notification.
fn notify_config_recovery(app: &AppHandle, recovery: &ConfigRecovery) {
    let result = app
        .notification()
        .builder()
        .title("Hops reset its settings")
        .body(format!(
            "config.json could not be read, so Hops started over with default settings. \
             The old file was kept as {}.",
            recovery.backup_file_name()
        ))
        .show();
    if let Err(error) = result {
        eprintln!("Could not show the config recovery notification: {error}");
    }
}

fn load_or_init_config_from_path(
    path: &Path,
    auto_populate_browsers: bool,
) -> Result<(AppConfig, Option<ConfigRecovery>), String> {
    let mut recovery = None;
    let mut config = match read_config_file(path)? {
        Some(Ok(config)) => config,
        Some(Err(error)) => {
            let backup_path = back_up_invalid_config(path)?;
            recovery = Some(ConfigRecovery { backup_path, error });
            let config = reset_config_with_detected_browsers(detect_browsers(), true);
            write_config_file(path, &config)?;
            config
        }
        None => {
            let mut config = default_config();
            merge_detected_browsers(&mut config, detect_browsers());
            write_config_file(path, &config)?;
            config
        }
    };

    let original_config = config.clone();
    normalize_config(&mut config);

    if auto_populate_browsers && config.browsers.is_empty() {
        merge_detected_browsers(&mut config, detect_browsers());
    }

    if config != original_config {
        write_config_file(path, &config)?;
    }

    Ok((config, recovery))
}

/// Reads `config.json`: `None` when there is no file, `Some(Err)` with the
/// parse error when it is not a valid config. Any serde error counts, from a
/// syntax error or an empty file to a missing field or an unknown enum value.
/// A file that exists but cannot be read is an error, not a broken config, so
/// it is never replaced.
fn read_config_file(path: &Path) -> Result<Option<Result<AppConfig, String>>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read(path)
        .map_err(|error| format!("Could not read config from {:?}: {error}", path))?;
    Ok(Some(
        serde_json::from_slice::<AppConfig>(&data).map_err(|error| error.to_string()),
    ))
}

/// Renames an invalid `config.json` to `config.json.broken-<unix seconds>`,
/// adding a counter when that name is taken so an earlier backup is never
/// overwritten.
fn back_up_invalid_config(path: &Path) -> Result<PathBuf, String> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    let base = format!("{}.broken-{seconds}", path.display());

    let backup_path = (0u32..)
        .map(|attempt| match attempt {
            0 => PathBuf::from(&base),
            n => PathBuf::from(format!("{base}-{n}")),
        })
        .find(|candidate| !candidate.exists())
        .expect("an unused backup name always exists");

    fs::rename(path, &backup_path).map_err(|error| {
        format!(
            "Config file {:?} is invalid and could not be backed up to {:?}: {error}",
            path, backup_path
        )
    })?;
    Ok(backup_path)
}

fn default_config() -> AppConfig {
    AppConfig {
        version: 1,
        always_show_picker: false,
        use_defaults_when_not_running: false,
        disable_transparency: false,
        theme_preference: ThemePreference::Light,
        onboarding_completed: false,
        default_browser_id: None,
        browsers: Vec::new(),
        rules: Vec::new(),
    }
}

fn normalize_browser_path(path: &str) -> String {
    path.trim().replace('/', "\\").to_lowercase()
}

pub(crate) fn reset_config_with_detected_browsers(
    detected_browsers: Vec<BrowserConfig>,
    onboarding_completed: bool,
) -> AppConfig {
    let mut config = default_config();
    config.onboarding_completed = onboarding_completed;
    merge_detected_browsers(&mut config, detected_browsers);
    config
}

pub(crate) fn normalize_config(config: &mut AppConfig) {
    if config.version == 0 {
        config.version = 1;
    }

    hydrate_detected_browser_defaults(config);
    hydrate_browser_icon_keys(config);
    normalize_manual_browser_entries(config);

    let browser_ids: HashSet<String> = config
        .browsers
        .iter()
        .map(|browser| browser.id.clone())
        .collect();

    if let Some(default_browser_id) = config.default_browser_id.clone() {
        if !browser_ids.contains(&default_browser_id) {
            config.default_browser_id = None;
        }
    }

    config
        .rules
        .retain(|rule| browser_ids.contains(&rule.browser_id));
}

pub(crate) fn validate_config(config: &AppConfig) -> Result<(), String> {
    let browser_ids: HashSet<String> = config
        .browsers
        .iter()
        .map(|browser| browser.id.clone())
        .collect();
    let mut browser_paths: HashSet<String> = HashSet::new();

    for browser in &config.browsers {
        validate_browser_entry(browser)?;

        let normalized_path = normalize_browser_path(&browser.path);
        if !browser_paths.insert(normalized_path) {
            return Err(format!(
                "Browser path '{}' is duplicated. Each browser entry must use a unique executable path.",
                browser.path
            ));
        }
    }

    if let Some(default_browser_id) = config.default_browser_id.as_deref() {
        if !browser_ids.contains(default_browser_id) {
            return Err("Default browser must reference an existing browser entry.".to_string());
        }
    }

    for rule in &config.rules {
        if !browser_ids.contains(&rule.browser_id) {
            return Err(format!(
                "Rule '{}' references browser '{}' which does not exist.",
                rule.id, rule.browser_id
            ));
        }

        if rule.pattern_type == RulePatternType::Regex {
            if let Some(error) = regex_pattern_error(&rule.pattern) {
                return Err(format!(
                    "Rule '{}' has an invalid regex pattern '{}': {error}",
                    rule.id, rule.pattern
                ));
            }
        }
    }

    Ok(())
}

pub(crate) fn save_config_internal(
    app: &AppHandle,
    mut config: AppConfig,
    validate_before_write: bool,
) -> Result<AppConfig, String> {
    normalize_config(&mut config);

    if validate_before_write {
        validate_config(&config)?;
    }

    let mut cache = lock_config_cache(app);
    let path = cache.path(app)?;
    cache.save(&path, &config)?;
    Ok(config)
}

pub(crate) fn write_config_file(path: &Path, config: &AppConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|error| format!("Could not serialize config JSON: {error}"))?;
    // The config path is resolved once and cached, so recreate the folder if it
    // was removed while Hops was running.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create config directory {:?}: {error}", parent))?;
    }
    write_file_atomically(path, json.as_bytes())
        .map_err(|error| format!("Could not write config to {:?}: {error}", path))
}

/// Writes to a sibling temp file, flushes it to disk, then renames it over `path`.
///
/// A crash or power loss mid-write leaves either the old file or the new one,
/// never a truncated mix. On Windows, `fs::rename` replaces the target
/// (`MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`).
fn write_file_atomically(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut temp_name = path.as_os_str().to_owned();
    temp_name.push(".tmp");
    let temp_path = PathBuf::from(temp_name);

    let result = (|| {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(contents)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browsers::build_detected_browser;
    use crate::models::{
        AppConfig, BrowserConfig, BrowserSource, ManualBrowserTrust, ThemePreference,
    };
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn temp_executable_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hops-config-test-{unique}"));
        fs::create_dir_all(&dir).expect("temp config test dir should create");
        let path = dir.join(name);
        fs::write(&path, []).expect("temp executable placeholder should write");
        path
    }

    #[test]
    fn reset_config_restores_defaults_and_keeps_only_detected_browsers() {
        let config = reset_config_with_detected_browsers(
            vec![
                build_detected_browser(
                    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
                    Some("Google Chrome"),
                    None,
                ),
                build_detected_browser(
                    "C:\\Program Files\\Mozilla Firefox\\firefox.exe",
                    Some("Mozilla Firefox"),
                    None,
                ),
            ],
            true,
        );

        assert_eq!(config.version, 1);
        assert!(!config.always_show_picker);
        assert!(!config.use_defaults_when_not_running);
        assert!(!config.disable_transparency);
        assert_eq!(config.theme_preference, ThemePreference::Light);
        assert!(config.onboarding_completed);
        assert_eq!(config.default_browser_id, None);
        assert!(config.rules.is_empty());
        assert_eq!(config.browsers.len(), 2);
        assert!(config
            .browsers
            .iter()
            .all(|browser| browser.source == BrowserSource::Detected));
    }

    #[test]
    fn reset_config_persists_valid_json() {
        let config = reset_config_with_detected_browsers(
            vec![build_detected_browser(
                "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
                Some("Google Chrome"),
                None,
            )],
            true,
        );
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("hops-reset-config-{unique}.json"));

        write_config_file(&path, &config).expect("reset config should write");
        let data = fs::read_to_string(&path).expect("reset config file should be readable");
        let loaded =
            serde_json::from_str::<AppConfig>(&data).expect("reset config file should parse");

        assert_eq!(loaded.rules.len(), 0);
        assert_eq!(loaded.default_browser_id, None);
        assert!(loaded.onboarding_completed);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn write_config_file_replaces_existing_file_without_leaving_temp_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hops-atomic-write-{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should create");
        let path = dir.join("config.json");
        fs::write(&path, "stale content that is longer than nothing")
            .expect("existing config should write");

        let mut config = reset_config_with_detected_browsers(Vec::new(), false);
        write_config_file(&path, &config).expect("first write should succeed");
        config.onboarding_completed = true;
        write_config_file(&path, &config).expect("overwrite should succeed");

        let data = fs::read_to_string(&path).expect("config should be readable");
        let loaded = serde_json::from_str::<AppConfig>(&data).expect("config should parse");
        assert!(loaded.onboarding_completed);

        let leftovers: Vec<_> = fs::read_dir(&dir)
            .expect("temp dir should list")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();
        assert_eq!(leftovers, vec![std::ffi::OsString::from("config.json")]);

        let _ = fs::remove_dir_all(&dir);
    }

    fn temp_config_path(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hops-config-cache-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should create");
        dir.join("config.json")
    }

    fn set_modified(path: &Path, modified: SystemTime) {
        fs::File::options()
            .write(true)
            .open(path)
            .expect("config should open for writing")
            .set_modified(modified)
            .expect("modified time should set");
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path.parent().expect("temp path should have parent"));
    }

    #[test]
    fn config_cache_serves_unchanged_file_from_memory() {
        let path = temp_config_path("hit");
        let config = reset_config_with_detected_browsers(Vec::new(), true);
        write_config_file(&path, &config).expect("config should write");

        let mut cache = CachedConfigState::default();
        let loaded = cache
            .load(&path, false)
            .expect("first load reads the file")
            .0;
        assert!(loaded.onboarding_completed);

        // Same size and modified time, but unreadable content: only a cache hit
        // can still return the config, which proves the file was not re-read.
        let stamp = FileStamp::of(&path).expect("stamp should read");
        fs::write(&path, "x".repeat(stamp.len as usize)).expect("garbage should write");
        set_modified(&path, stamp.modified);

        let cached = cache
            .load(&path, false)
            .expect("second load hits the cache")
            .0;
        assert_eq!(cached, loaded);

        cleanup(&path);
    }

    #[test]
    fn config_cache_picks_up_hand_edits() {
        let path = temp_config_path("edit");
        let config = reset_config_with_detected_browsers(Vec::new(), false);
        write_config_file(&path, &config).expect("config should write");

        let mut cache = CachedConfigState::default();
        let first = cache
            .load(&path, false)
            .expect("first load reads the file")
            .0;
        assert!(!first.onboarding_completed);

        let stamp = FileStamp::of(&path).expect("stamp should read");
        let mut edited = config.clone();
        edited.onboarding_completed = true;
        let json = serde_json::to_string_pretty(&edited).expect("config should serialize");
        fs::write(&path, json).expect("hand edit should write");
        set_modified(&path, stamp.modified + Duration::from_secs(5));

        let reloaded = cache.load(&path, false).expect("changed file reloads").0;
        assert!(reloaded.onboarding_completed);

        cleanup(&path);
    }

    #[test]
    fn config_cache_save_updates_memory_and_disk() {
        let path = temp_config_path("save");
        let config = reset_config_with_detected_browsers(Vec::new(), false);
        write_config_file(&path, &config).expect("config should write");

        let mut cache = CachedConfigState::default();
        cache
            .load(&path, false)
            .expect("first load reads the file")
            .0;

        let mut updated = config.clone();
        updated.always_show_picker = true;
        cache.save(&path, &updated).expect("save should succeed");

        assert!(
            cache
                .load(&path, false)
                .expect("load after save")
                .0
                .always_show_picker
        );
        let on_disk: AppConfig =
            serde_json::from_str(&fs::read_to_string(&path).expect("config should read"))
                .expect("config should parse");
        assert!(on_disk.always_show_picker);

        cleanup(&path);
    }

    #[test]
    fn config_cache_forgets_entry_when_save_fails() {
        let path = temp_config_path("fail");
        let config = reset_config_with_detected_browsers(Vec::new(), false);
        write_config_file(&path, &config).expect("config should write");

        let mut cache = CachedConfigState::default();
        cache
            .load(&path, false)
            .expect("first load reads the file")
            .0;

        // A directory where the file should be makes the rename fail.
        let blocked = path.with_file_name("blocked");
        fs::create_dir_all(blocked.join("config.json")).expect("blocking dir should create");
        assert!(cache.save(&blocked.join("config.json"), &config).is_err());
        assert!(cache.entry.is_none());

        cleanup(&path);
    }

    #[test]
    fn validate_config_rejects_duplicate_browser_paths() {
        let path = temp_executable_path("custom-browser.exe");
        let path_string = path.to_string_lossy().to_string();
        let config = AppConfig {
            version: 1,
            always_show_picker: false,
            use_defaults_when_not_running: false,
            disable_transparency: false,
            theme_preference: ThemePreference::Light,
            onboarding_completed: true,
            default_browser_id: None,
            browsers: vec![
                BrowserConfig {
                    id: "manual-a".to_string(),
                    name: "Custom A".to_string(),
                    path: path_string.clone(),
                    private_flag: None,
                    icon_key: Some("generic".to_string()),
                    manual_trust: Some(ManualBrowserTrust::UserConfirmed),
                    source: BrowserSource::Manual,
                    is_hidden: false,
                },
                BrowserConfig {
                    id: "manual-b".to_string(),
                    name: "Custom B".to_string(),
                    path: path_string.clone(),
                    private_flag: None,
                    icon_key: Some("generic".to_string()),
                    manual_trust: Some(ManualBrowserTrust::UserConfirmed),
                    source: BrowserSource::Manual,
                    is_hidden: false,
                },
            ],
            rules: Vec::new(),
        };

        let error = validate_config(&config).expect_err("duplicate browser paths should reject");
        assert!(error.contains("duplicated"));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(path.parent().expect("temp path should have parent"));
    }

    fn backups_next_to(path: &Path) -> Vec<PathBuf> {
        let mut backups: Vec<PathBuf> = fs::read_dir(path.parent().expect("temp path has parent"))
            .expect("temp dir should list")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|candidate| {
                candidate
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("config.json.broken-"))
            })
            .collect();
        backups.sort();
        backups
    }

    #[test]
    fn validate_config_accepts_a_browser_whose_executable_was_removed() {
        let mut config = reset_config_with_detected_browsers(
            vec![build_detected_browser(
                r"C:\Hops Test\Uninstalled\Google\Chrome\Application\chrome.exe",
                Some("Google Chrome"),
                None,
            )],
            true,
        );
        config.default_browser_id = Some(config.browsers[0].id.clone());
        config.theme_preference = ThemePreference::Dark;

        validate_config(&config).expect("a missing exe must not block saving other settings");
    }

    #[test]
    fn invalid_json_is_backed_up_and_replaced_with_defaults() {
        let path = temp_config_path("invalid-json");
        let broken = "{ \"version\": 1, \"rules\": [,] }";
        fs::write(&path, broken).expect("broken config should write");

        let mut cache = CachedConfigState::default();
        let (config, recovery) = cache
            .load(&path, false)
            .expect("an invalid config is recovered, not an error");

        let recovery = recovery.expect("the load reports the recovery");
        assert_eq!(
            fs::read_to_string(&recovery.backup_path).expect("backup should read"),
            broken
        );
        assert_eq!(backups_next_to(&path), vec![recovery.backup_path.clone()]);
        assert!(config.onboarding_completed, "a recovering user is not new");
        assert!(config.rules.is_empty());

        let on_disk: AppConfig =
            serde_json::from_str(&fs::read_to_string(&path).expect("config should read"))
                .expect("the replacement config is valid");
        assert_eq!(on_disk, config);

        let notice = cache.recovery_notice.take().expect("a notice is kept");
        assert!(notice.contains(&recovery.backup_path.display().to_string()));

        // The replacement is a normal config from now on.
        assert!(cache
            .load(&path, false)
            .expect("second load succeeds")
            .1
            .is_none());

        cleanup(&path);
    }

    #[test]
    fn unknown_enum_values_and_empty_files_count_as_invalid() {
        let config = reset_config_with_detected_browsers(Vec::new(), false);
        let unknown_theme = serde_json::to_string(&config)
            .expect("config should serialize")
            .replace("\"light\"", "\"sepia\"");

        for contents in [unknown_theme.as_str(), ""] {
            let path = temp_config_path("semantic");
            fs::write(&path, contents).expect("config should write");

            let (_, recovery) =
                load_or_init_config_from_path(&path, false).expect("config is recovered");
            assert!(recovery.is_some(), "{contents:?} should be recovered");

            cleanup(&path);
        }
    }

    #[test]
    fn a_missing_config_file_is_created_without_a_recovery() {
        let path = temp_config_path("missing");

        let (_, recovery) = load_or_init_config_from_path(&path, false).expect("config is created");
        assert!(recovery.is_none());
        assert!(path.exists());
        assert!(backups_next_to(&path).is_empty());

        cleanup(&path);
    }

    #[test]
    fn a_second_backup_never_overwrites_the_first() {
        let path = temp_config_path("two-backups");

        fs::write(&path, "first").expect("config should write");
        let first = back_up_invalid_config(&path).expect("first backup");
        fs::write(&path, "second").expect("config should write");
        let second = back_up_invalid_config(&path).expect("second backup");

        assert_ne!(first, second);
        assert_eq!(fs::read_to_string(&first).expect("first backup"), "first");
        assert_eq!(
            fs::read_to_string(&second).expect("second backup"),
            "second"
        );

        cleanup(&path);
    }
}
