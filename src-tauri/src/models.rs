use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BrowserSource {
    Detected,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RulePatternType {
    Hostname,
    HostnameSubdomains,
    Prefix,
    Contains,
    FullUrl,
    Glob,
    Regex,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ThemePreference {
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowserConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) private_flag: Option<String>,
    #[serde(default)]
    pub(crate) icon_key: Option<String>,
    #[serde(default)]
    pub(crate) manual_trust: Option<ManualBrowserTrust>,
    pub(crate) source: BrowserSource,
    pub(crate) is_hidden: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ManualBrowserTrust {
    Verified,
    UserConfirmed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BrowserRecognition {
    Known,
    RecognizedFamily,
    UnverifiedManual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuleConfig {
    pub(crate) id: String,
    pub(crate) pattern: String,
    pub(crate) pattern_type: RulePatternType,
    pub(crate) browser_id: String,
    pub(crate) private_mode: bool,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppConfig {
    pub(crate) version: u32,
    pub(crate) always_show_picker: bool,
    pub(crate) use_defaults_when_not_running: bool,
    pub(crate) disable_transparency: bool,
    #[serde(default = "default_theme_preference")]
    pub(crate) theme_preference: ThemePreference,
    #[serde(default)]
    pub(crate) onboarding_completed: bool,
    pub(crate) default_browser_id: Option<String>,
    pub(crate) browsers: Vec<BrowserConfig>,
    pub(crate) rules: Vec<RuleConfig>,
}

fn default_theme_preference() -> ThemePreference {
    ThemePreference::Light
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RouteAction {
    OpenBrowser,
    ShowPicker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteDecision {
    pub(crate) action: RouteAction,
    pub(crate) reason: String,
    pub(crate) browser_id: Option<String>,
    pub(crate) browser_name: Option<String>,
    pub(crate) private_mode: bool,
    pub(crate) matched_rule_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenUrlRequest {
    pub(crate) browser_id: String,
    pub(crate) url: String,
    pub(crate) private_mode: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManualBrowserValidationRequest {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) private_flag: Option<String>,
    #[serde(default)]
    pub(crate) allow_user_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManualBrowserValidationResult {
    pub(crate) recognition: BrowserRecognition,
    pub(crate) manual_trust: Option<ManualBrowserTrust>,
    pub(crate) browser_name: String,
    pub(crate) private_flag: Option<String>,
    pub(crate) family: Option<String>,
    pub(crate) icon_key: Option<String>,
    pub(crate) requires_confirmation: bool,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowserRegistrationStatus {
    pub(crate) registered: bool,
    pub(crate) is_default_http: bool,
    pub(crate) is_default_https: bool,
    pub(crate) is_fully_default: bool,
    pub(crate) current_http_prog_id: Option<String>,
    pub(crate) current_https_prog_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PickerLaunchSource {
    Route,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PickerBrowserEntry {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) private_flag: Option<String>,
    pub(crate) icon_key: Option<String>,
    pub(crate) is_default: bool,
    pub(crate) is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PickerSession {
    pub(crate) url: String,
    pub(crate) reason: String,
    pub(crate) source: PickerLaunchSource,
    pub(crate) preferred_browser_id: Option<String>,
    pub(crate) preferred_private_mode: bool,
    pub(crate) disable_transparency: bool,
    pub(crate) theme_preference: ThemePreference,
    pub(crate) always_show_picker: bool,
    pub(crate) alt_pressed: bool,
    pub(crate) browsers: Vec<PickerBrowserEntry>,
}
