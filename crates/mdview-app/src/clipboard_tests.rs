//! Build-configuration checks that frame tests cannot observe.
//!
//! egui always exposes copy and paste events, but without eframe's `clipboard` feature it
//! swaps the OS clipboard for an in-process simulation: copy works inside the app and is
//! invisible everywhere else, and text copied elsewhere never reaches the paste event.

use toml::Value;

const WORKSPACE_MANIFEST: &str = include_str!("../../../Cargo.toml");

#[test]
fn eframe_keeps_the_system_clipboard() {
    let parsed: Result<Value, _> = toml::from_str(WORKSPACE_MANIFEST);
    assert!(parsed.is_ok(), "the workspace Cargo.toml must parse");
    let Ok(manifest) = parsed else {
        return;
    };
    let eframe = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(|dependencies| dependencies.get("eframe"));
    assert!(eframe.is_some(), "the workspace manifest must declare eframe");
    let Some(eframe) = eframe else {
        return;
    };
    let default_features = eframe.get("default-features").and_then(Value::as_bool).unwrap_or(true);
    let clipboard_feature = eframe
        .get("features")
        .and_then(Value::as_array)
        .is_some_and(|features| features.iter().any(|feature| feature.as_str() == Some("clipboard")));
    assert!(
        default_features || clipboard_feature,
        "eframe must keep the system clipboard: with default-features = false, list \"clipboard\" in features"
    );
}
