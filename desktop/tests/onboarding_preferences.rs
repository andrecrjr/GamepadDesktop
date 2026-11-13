use gamepad_desktop::{control::ControlMode, preferences::DesktopPreferences};

#[test]
fn first_run_requires_the_welcome_tour() {
    let value = serde_json::to_value(DesktopPreferences::default()).unwrap();
    assert_eq!(value["onboarding_completed"], false);
}

#[test]
fn old_preferences_keep_their_settings_and_offer_the_tour_once() {
    let preferences: DesktopPreferences = serde_json::from_value(serde_json::json!({
        "locale": "pt-BR",
        "controls": { "mode": "game", "enabled": false },
        "start_on_login": true
    }))
    .unwrap();
    assert_eq!(preferences.controls.mode, ControlMode::Game);
    assert!(!preferences.controls.enabled);
    assert!(preferences.start_on_login);
    assert_eq!(
        serde_json::to_value(preferences).unwrap()["onboarding_completed"],
        false
    );
}

#[test]
fn completion_is_remembered_after_reloading_preferences() {
    let preferences: DesktopPreferences = serde_json::from_value(serde_json::json!({
        "onboarding_completed": true
    }))
    .unwrap();
    assert_eq!(
        serde_json::to_value(preferences).unwrap()["onboarding_completed"],
        true
    );
}

#[test]
fn interface_scale_defaults_larger_and_survives_preferences_roundtrip() {
    let defaults = serde_json::to_value(DesktopPreferences::default()).unwrap();
    assert_eq!(defaults["ui_scale"], "comfortable");
    for scale in ["standard", "comfortable", "large", "extra_large"] {
        let preferences: DesktopPreferences =
            serde_json::from_value(serde_json::json!({"ui_scale": scale})).unwrap();
        assert_eq!(
            serde_json::to_value(preferences).unwrap()["ui_scale"],
            scale
        );
    }
}

#[test]
fn interface_scale_steps_are_bounded() {
    use gamepad_desktop::preferences::UiScale;
    assert_eq!(UiScale::Standard.step(false), UiScale::Standard);
    assert_eq!(UiScale::ExtraLarge.step(true), UiScale::ExtraLarge);
    assert_eq!(UiScale::default().factor(), 1.25);
    for pair in UiScale::ALL.windows(2) {
        assert_eq!(pair[0].step(true), pair[1]);
        assert_eq!(pair[1].step(false), pair[0]);
    }
}
