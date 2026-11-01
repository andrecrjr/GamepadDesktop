use gamepad_desktop::keyboard::{KEY_ROWS, KeyboardKey, KeyboardSession, key_width_quarters};

#[test]
fn typing_rows_follow_the_standard_qwerty_keyboard() {
    let characters: Vec<String> = KEY_ROWS[1..5]
        .iter()
        .map(|row| row.iter().filter_map(|key| key.character(false)).collect())
        .collect();
    assert_eq!(
        characters,
        [
            "`1234567890-=",
            "qwertyuiop[]\\",
            "asdfghjkl;'",
            "zxcvbnm,./"
        ]
    );
    assert_eq!(KEY_ROWS[0].first(), Some(&KeyboardKey::Function(1)));
    assert_eq!(KEY_ROWS[0].last(), Some(&KeyboardKey::Function(12)));
    assert_eq!(KEY_ROWS[2][0], KeyboardKey::Tab);
    assert_eq!(KEY_ROWS[3][0].label(false), "Caps");
    assert_eq!(KEY_ROWS[4][0], KeyboardKey::Shift);
    assert_eq!(KEY_ROWS[4].last(), Some(&KeyboardKey::Shift));
}

#[test]
fn qwerty_rows_use_physical_key_proportions() {
    for (index, row) in KEY_ROWS.iter().enumerate() {
        assert_eq!(
            (0..row.len())
                .map(|column| key_width_quarters(index, column))
                .sum::<u16>(),
            60
        );
    }
    assert_eq!(key_width_quarters(2, 0), 6); // Tab: 1.5 keys
    assert_eq!(key_width_quarters(4, 0), 9); // Left Shift: 2.25 keys
    assert_eq!(key_width_quarters(5, 3), 25); // Space: 6.25 keys
}

#[test]
fn vertical_navigation_enters_space_and_returns_to_the_original_key() {
    let mut keyboard = KeyboardSession::default();
    keyboard.selected_row = 2;
    keyboard.selected_column = 4;
    assert_eq!(keyboard.selected_key(), KeyboardKey::Character('r'));
    for _ in 0..3 {
        keyboard.move_selection(1, 0);
    }
    assert_eq!(keyboard.selected_key(), KeyboardKey::Space);
    for _ in 0..3 {
        keyboard.move_selection(-1, 0);
    }
    assert_eq!(keyboard.selected_key(), KeyboardKey::Character('r'));
}

#[test]
fn saved_settings_accept_missing_fields_and_bound_extreme_values() {
    use gamepad_desktop::keyboard::KeyboardSettings;
    let settings: KeyboardSettings =
        serde_json::from_str(r#"{"width":5000,"repeat_interval_ms":0}"#).unwrap();
    let settings = settings.normalized();
    assert_eq!(settings.width, 1000);
    assert_eq!(settings.repeat_interval_ms, 100);
    assert_eq!(
        settings.repeat_delay_ms,
        KeyboardSettings::default().repeat_delay_ms
    );
    assert_eq!(
        serde_json::from_slice::<KeyboardSettings>(&serde_json::to_vec(&settings).unwrap())
            .unwrap(),
        settings
    );
}
