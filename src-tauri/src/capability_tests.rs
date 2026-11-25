//! Checks that `app_commands.rs` and the capability files agree, so a command
//! nobody can call, or a picker that gains a settings command, fails the tests
//! instead of breaking at runtime.

use std::collections::{BTreeMap, BTreeSet};

include!("../app_commands.rs");

const PICKER_COMMANDS: &[&str] = &["get_picker_state", "hide_picker_window", "open_url"];

const CAPABILITIES: &[(&str, &str)] = &[
    ("default", include_str!("../capabilities/default.json")),
    ("picker", include_str!("../capabilities/picker.json")),
];

fn permission_for(command: &str) -> String {
    format!("allow-{}", command.replace('_', "-"))
}

/// App commands granted by each capability. Plugin and core permissions
/// (`core:default`, `process:allow-restart`, ...) are ignored.
fn granted_commands() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    CAPABILITIES
        .iter()
        .map(|(name, json)| {
            let capability: serde_json::Value = serde_json::from_str(json)
                .unwrap_or_else(|error| panic!("capability {name} is not valid JSON: {error}"));
            let permissions: BTreeSet<String> = capability["permissions"]
                .as_array()
                .unwrap_or_else(|| panic!("capability {name} has no permissions array"))
                .iter()
                .filter_map(|permission| permission.as_str().map(str::to_string))
                .collect();
            let commands = APP_COMMANDS
                .iter()
                .copied()
                .filter(|command| permissions.contains(&permission_for(command)))
                .collect();
            (*name, commands)
        })
        .collect()
}

#[test]
fn every_app_command_is_granted_to_some_window() {
    let granted = granted_commands();
    for command in APP_COMMANDS {
        assert!(
            granted.values().any(|commands| commands.contains(command)),
            "{command} is not granted to any window"
        );
    }
}

#[test]
fn picker_is_granted_only_its_own_commands() {
    let granted = granted_commands();
    let expected: BTreeSet<&str> = PICKER_COMMANDS.iter().copied().collect();
    let unexpected: Vec<_> = granted["picker"].difference(&expected).collect();
    assert!(
        unexpected.is_empty(),
        "picker grants unexpected commands {unexpected:?}"
    );
    let missing: Vec<_> = expected.difference(&granted["picker"]).collect();
    assert!(missing.is_empty(), "picker is missing commands {missing:?}");
}

#[test]
fn no_app_command_is_granted_to_more_than_one_window() {
    let granted = granted_commands();
    for command in APP_COMMANDS {
        let windows: Vec<&str> = granted
            .iter()
            .filter(|(_, commands)| commands.contains(command))
            .map(|(name, _)| *name)
            .collect();
        assert!(
            windows.len() <= 1,
            "{command} is granted to more than one window: {windows:?}"
        );
    }
}
