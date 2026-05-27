use crate::browsers::{
    detect_browsers, hydrate_browser_icon_keys, hydrate_detected_browser_defaults,
    merge_detected_browsers, normalize_manual_browser_entries, validate_browser_for_launch,
};
use crate::models::{AppConfig, BrowserConfig, RulePatternType, ThemePreference};
use crate::CONFIG_FILENAME;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

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

pub(crate) fn load_or_init_config(
    app: &AppHandle,
    auto_populate_browsers: bool,
) -> Result<AppConfig, String> {
    let path = config_file_path(app)?;

    let mut config = if path.exists() {
        let data = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read config from {:?}: {error}", path))?;
        serde_json::from_str::<AppConfig>(&data)
            .map_err(|error| format!("Config file {:?} is invalid JSON: {error}", path))?
    } else {
        let mut config = default_config();
        merge_detected_browsers(&mut config, detect_browsers());
        write_config_file(&path, &config)?;
        config
    };

    let original_config = config.clone();
    normalize_config(&mut config);

    if auto_populate_browsers && config.browsers.is_empty() {
        merge_detected_browsers(&mut config, detect_browsers());
    }

    if config != original_config {
        write_config_file(&path, &config)?;
    }

    Ok(config)
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
        validate_browser_for_launch(browser)?;

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
            Regex::new(&rule.pattern).map_err(|error| {
                format!(
                    "Rule '{}' has an invalid regex pattern '{}': {error}",
                    rule.id, rule.pattern
                )
            })?;
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

    let path = config_file_path(app)?;
    write_config_file(&path, &config)?;
    Ok(config)
}

pub(crate) fn write_config_file(path: &Path, config: &AppConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|error| format!("Could not serialize config JSON: {error}"))?;
    fs::write(path, json).map_err(|error| format!("Could not write config to {:?}: {error}", path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browsers::build_detected_browser;
    use crate::models::{
        AppConfig, BrowserConfig, BrowserSource, ManualBrowserTrust, ThemePreference,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

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
}
