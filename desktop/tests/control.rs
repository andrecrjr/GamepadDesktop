use std::time::Duration;

use gamepad_desktop::control::{
    Action, Button, ControlSettings, DpadDirection, GamepadFrame, MouseButton, MouseController,
    SystemKey,
};

const TICK: Duration = Duration::from_millis(10);

fn ready() -> (MouseController, ControlSettings) {
    let mut controller = MouseController::default();
    let mut settings = ControlSettings::default();
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    (controller, settings)
}

fn pressed(button: Button) -> GamepadFrame {
    let mut frame = GamepadFrame::default();
    frame.buttons[button as usize] = true;
    frame
}

#[test]
fn dpad_sends_arrow_keys_while_mouse_and_drag_work() {
    let (mut controller, mut settings) = ready();
    let mut frame = pressed(Button::Down);
    frame.x = 1.0;
    frame.buttons[Button::South as usize] = true;
    let actions = controller.update(Some(&frame), TICK, &mut settings);
    assert!(actions.contains(&Action::Mouse(MouseButton::Left, true)));
    assert!(actions.contains(&Action::Dpad(DpadDirection::Down)));
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, Action::Move(x, 0) if *x > 0))
    );
}

#[test]
fn x_sends_enter_and_y_sends_escape() {
    let (mut controller, mut settings) = ready();
    assert_eq!(
        controller.update(Some(&pressed(Button::West)), TICK, &mut settings),
        vec![Action::Key(SystemKey::Enter)]
    );
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    assert_eq!(
        controller.update(Some(&pressed(Button::North)), TICK, &mut settings),
        vec![Action::Key(SystemKey::Escape)]
    );
}

#[test]
fn swapped_stick_bindings_open_keyboard_and_launcher() {
    let (mut controller, mut settings) = ready();
    assert_eq!(
        controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings),
        vec![Action::OnScreenKeyboard(true)]
    );
    assert!(controller.keyboard.open);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    assert_eq!(
        controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings),
        vec![Action::OnScreenKeyboard(false)]
    );
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    assert_eq!(
        controller.update(Some(&pressed(Button::LeftStick)), TICK, &mut settings),
        vec![Action::Key(SystemKey::Meta)]
    );
    assert!(!controller.keyboard.open);
}

#[test]
fn dpad_moves_pointer_when_system_keyboard_does_not_use_gamepad_focus() {
    let (mut controller, mut settings) = ready();
    controller.set_system_keyboard_uses_gamepad(false);
    controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);

    assert_eq!(
        controller.update(Some(&pressed(Button::Right)), TICK, &mut settings),
        vec![Action::Dpad(DpadDirection::Right)]
    );
}

#[test]
fn swapped_keyboard_shortcuts_keep_space_separate_from_close() {
    use gamepad_desktop::control::KeyboardInput;
    let (mut controller, mut settings) = ready();
    controller.set_system_keyboard_uses_gamepad(true);
    controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    assert_eq!(
        controller.update(Some(&pressed(Button::LeftStick)), TICK, &mut settings),
        vec![Action::Keyboard(KeyboardInput::Space)]
    );
    assert!(controller.keyboard.open);
}

#[test]
fn stick_drift_and_invalid_axes_do_not_move_the_pointer() {
    let (mut controller, mut settings) = ready();
    let frame = GamepadFrame {
        x: 0.1,
        y: -0.1,
        scroll: f64::NAN,
        ..Default::default()
    };
    assert!(
        controller
            .update(Some(&frame), TICK, &mut settings)
            .is_empty()
    );
}

#[test]
fn movement_is_independent_of_poll_rate_and_keeps_small_movements() {
    fn distance(ticks: usize, tick_ms: u64, x: f64) -> i32 {
        let (mut controller, mut settings) = ready();
        let frame = GamepadFrame {
            x,
            ..Default::default()
        };
        (0..ticks)
            .flat_map(|_| {
                controller.update(Some(&frame), Duration::from_millis(tick_ms), &mut settings)
            })
            .map(|action| match action {
                Action::Move(x, _) => x,
                _ => 0,
            })
            .sum()
    }
    assert!((distance(100, 10, 1.0) - distance(50, 20, 1.0)).abs() <= 1);
    assert!(distance(100, 10, 0.3) > 0);
    assert!(distance(100, 10, 0.3) < distance(100, 10, 1.0));
}

#[test]
fn holding_a_drags_and_release_ends_the_click() {
    let (mut controller, mut settings) = ready();
    let mut frame = pressed(Button::South);
    assert_eq!(
        controller.update(Some(&frame), TICK, &mut settings),
        vec![Action::Mouse(MouseButton::Left, true)]
    );
    frame.x = 1.0;
    let actions = controller.update(Some(&frame), TICK, &mut settings);
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, Action::Move(x, 0) if *x > 0))
    );
    assert!(
        !actions
            .iter()
            .any(|action| matches!(action, Action::Mouse(_, _)))
    );
    assert_eq!(
        controller.update(Some(&GamepadFrame::default()), TICK, &mut settings),
        vec![Action::Mouse(MouseButton::Left, false)]
    );
}

#[test]
fn disconnect_releases_buttons_and_reconnect_requires_neutral() {
    let (mut controller, mut settings) = ready();
    let frame = pressed(Button::South);
    controller.update(Some(&frame), TICK, &mut settings);
    assert_eq!(
        controller.update(None, TICK, &mut settings),
        vec![Action::Mouse(MouseButton::Left, false)]
    );
    assert!(
        controller
            .update(Some(&frame), TICK, &mut settings)
            .is_empty()
    );
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    assert_eq!(
        controller.update(Some(&frame), TICK, &mut settings),
        vec![Action::Mouse(MouseButton::Left, true)]
    );
}

#[test]
fn start_alone_does_not_pause_mouse_controls() {
    let (mut controller, mut settings) = ready();
    controller.update(Some(&pressed(Button::Start)), TICK, &mut settings);
    assert!(settings.enabled);
}

#[test]
fn start_and_select_pause_resume_once_and_release_drag() {
    let (mut controller, mut settings) = ready();
    controller.update(Some(&pressed(Button::South)), TICK, &mut settings);
    let mut frame = pressed(Button::Start);
    frame.buttons[Button::Select as usize] = true;
    frame.buttons[Button::South as usize] = true;
    assert_eq!(
        controller.update(Some(&frame), TICK, &mut settings),
        vec![Action::Mouse(MouseButton::Left, false)]
    );
    assert!(!settings.enabled);
    assert!(
        controller
            .update(Some(&frame), TICK, &mut settings)
            .is_empty()
    );
    assert!(!settings.enabled);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    controller.update(Some(&frame), TICK, &mut settings);
    assert!(settings.enabled);
    assert!(
        controller
            .update(Some(&frame), TICK, &mut settings)
            .is_empty()
    );
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    assert_eq!(
        controller.update(Some(&pressed(Button::South)), TICK, &mut settings),
        vec![Action::Mouse(MouseButton::Left, true)]
    );
}

#[test]
fn gui_pause_also_releases_a_held_button() {
    let (mut controller, mut settings) = ready();
    let frame = pressed(Button::East);
    controller.update(Some(&frame), TICK, &mut settings);
    settings.enabled = false;
    assert_eq!(
        controller.update(Some(&frame), TICK, &mut settings),
        vec![Action::Mouse(MouseButton::Right, false)]
    );
}

#[test]
fn l3_opens_launcher_once_and_shoulders_navigate_apps() {
    let (mut controller, mut settings) = ready();
    let frame = pressed(Button::LeftStick);
    assert_eq!(
        controller.update(Some(&frame), TICK, &mut settings),
        vec![Action::Key(SystemKey::Meta)]
    );
    assert!(
        controller
            .update(Some(&frame), TICK, &mut settings)
            .is_empty()
    );
    assert_eq!(
        controller.update(Some(&pressed(Button::West)), TICK, &mut settings),
        vec![Action::Key(SystemKey::Enter)]
    );
    assert_eq!(
        controller.update(Some(&pressed(Button::RightShoulder)), TICK, &mut settings),
        vec![Action::NextWindow]
    );
    assert_eq!(
        controller.update(Some(&pressed(Button::LeftShoulder)), TICK, &mut settings),
        vec![Action::Key(SystemKey::Tab)]
    );
}

#[test]
fn dpad_arrow_keys_repeat_after_delay_without_repeating_confirm() {
    let (mut controller, mut settings) = ready();
    let mut frame = pressed(Button::Down);
    frame.buttons[Button::West as usize] = true;
    assert_eq!(
        controller.update(Some(&frame), TICK, &mut settings),
        vec![
            Action::Key(SystemKey::Enter),
            Action::Dpad(DpadDirection::Down),
        ]
    );
    for _ in 0..20 {
        assert!(
            controller
                .update(Some(&frame), TICK, &mut settings)
                .is_empty()
        );
    }
    let repeated: Vec<_> = (0..30)
        .flat_map(|_| controller.update(Some(&frame), TICK, &mut settings))
        .collect();
    assert!(!repeated.is_empty());
    assert!(
        repeated
            .iter()
            .all(|action| *action == Action::Dpad(DpadDirection::Down))
    );
}

#[test]
fn dpad_route_ends_only_after_all_directions_are_released() {
    let (mut controller, mut settings) = ready();
    let up = pressed(Button::Up);
    assert!(
        controller
            .update(Some(&up), TICK, &mut settings)
            .contains(&Action::Dpad(DpadDirection::Up))
    );

    let mut up_and_right = up;
    up_and_right.buttons[Button::Right as usize] = true;
    assert!(
        controller
            .update(Some(&up_and_right), TICK, &mut settings)
            .contains(&Action::Dpad(DpadDirection::Right))
    );

    let right_only = pressed(Button::Right);
    assert!(
        !controller
            .update(Some(&right_only), TICK, &mut settings)
            .contains(&Action::DpadReleased)
    );
    assert_eq!(
        controller.update(Some(&GamepadFrame::default()), TICK, &mut settings),
        vec![Action::DpadReleased]
    );
    assert!(
        controller
            .update(Some(&GamepadFrame::default()), TICK, &mut settings)
            .is_empty()
    );
}

#[test]
fn r3_opens_system_keyboard_without_disabling_mouse_control() {
    let (mut controller, mut settings) = ready();
    let mut frame = GamepadFrame {
        x: 1.0,
        ..Default::default()
    };
    frame.buttons[Button::RightStick as usize] = true;

    let actions = controller.update(Some(&frame), TICK, &mut settings);

    assert!(controller.keyboard.open);
    assert!(actions.contains(&Action::OnScreenKeyboard(true)));
    assert!(settings.enabled);
    assert!(
        !actions
            .iter()
            .any(|action| matches!(action, Action::Move(_, _)))
    );
}

#[test]
fn native_gamepad_keyboard_mode_does_not_inject_duplicate_mouse_movement() {
    let (mut controller, mut settings) = ready();
    controller.set_system_keyboard_uses_gamepad(true);
    assert!(
        controller
            .update(Some(&pressed(Button::RightStick)), TICK, &mut settings)
            .contains(&Action::OnScreenKeyboard(true))
    );
    let frame = GamepadFrame {
        x: 1.0,
        ..pressed(Button::Start)
    };

    assert!(
        controller
            .update(Some(&frame), TICK, &mut settings)
            .is_empty()
    );
    assert!(settings.enabled);
}

#[test]
fn scroll_and_pointer_do_not_jump_after_a_stall() {
    let (mut controller, mut settings) = ready();
    let frame = GamepadFrame {
        x: 1.0,
        scroll: 1.0,
        ..Default::default()
    };
    let actions = controller.update(Some(&frame), Duration::from_secs(60), &mut settings);
    assert!(
        actions
            .iter()
            .all(|action| !matches!(action, Action::Move(x, _) if *x > 100))
    );
    let actions: Vec<_> = (0..20)
        .flat_map(|_| controller.update(Some(&frame), TICK, &mut settings))
        .collect();
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, Action::Scroll(amount) if *amount > 0))
    );
}

#[test]
fn r3_toggles_keyboard_once_and_releases_drag() {
    let (mut controller, mut settings) = ready();
    controller.update(Some(&pressed(Button::South)), TICK, &mut settings);
    let frame = pressed(Button::RightStick);
    let actions = controller.update(Some(&frame), TICK, &mut settings);
    assert!(actions.contains(&Action::OnScreenKeyboard(true)));
    assert!(actions.contains(&Action::Mouse(MouseButton::Left, false)));
    assert!(controller.keyboard.open);
    assert!(
        controller
            .update(Some(&frame), TICK, &mut settings)
            .is_empty()
    );
    assert!(controller.keyboard.open);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    assert_eq!(
        controller.update(Some(&frame), TICK, &mut settings),
        vec![Action::OnScreenKeyboard(false)]
    );
    assert!(!controller.keyboard.open);
}

#[test]
fn pause_and_disconnect_request_system_keyboard_close() {
    for disconnect in [false, true] {
        let (mut controller, mut settings) = ready();
        let open_actions =
            controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
        assert!(open_actions.contains(&Action::OnScreenKeyboard(true)));
        let mut pause = pressed(Button::Start);
        pause.buttons[Button::Select as usize] = true;
        let frame = if disconnect { None } else { Some(&pause) };
        let actions = controller.update(frame, TICK, &mut settings);
        assert!(actions.contains(&Action::OnScreenKeyboard(false)));
        assert!(!controller.keyboard.open);
    }
}

#[test]
fn r3_cannot_open_keyboard_when_paused_or_connecting_with_it_held() {
    let mut controller = MouseController::default();
    let mut settings = ControlSettings::default();
    controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
    assert!(!controller.keyboard.open);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    settings.enabled = false;
    controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
    assert!(!controller.keyboard.open);
}

#[test]
fn game_mode_can_operate_keyboard_and_stop_pointer_after_closing() {
    use gamepad_desktop::control::ControlMode;

    for keyboard_uses_gamepad in [false, true] {
        let mut controller = MouseController::default();
        controller.set_system_keyboard_uses_gamepad(keyboard_uses_gamepad);
        let mut settings = ControlSettings {
            mode: ControlMode::Game,
            ..Default::default()
        };
        let neutral = GamepadFrame::default();
        controller.update(Some(&neutral), TICK, &mut settings);
        controller.update(Some(&neutral), TICK, &mut settings);
        assert_eq!(
            controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings),
            vec![Action::OnScreenKeyboard(true)]
        );
        assert!(controller.keyboard.open);
        assert!(
            controller
                .update(Some(&pressed(Button::RightStick)), TICK, &mut settings)
                .is_empty()
        );
        controller.update(Some(&neutral), TICK, &mut settings);
        let navigation = GamepadFrame {
            x: 0.8,
            scroll: 0.8,
            ..pressed(Button::Right)
        };
        for _ in 0..50 {
            let actions = controller.update(Some(&navigation), TICK, &mut settings);
            if keyboard_uses_gamepad {
                assert!(actions.is_empty());
            } else {
                assert!(
                    actions
                        .iter()
                        .any(|action| matches!(action, Action::Move(x, 0) if *x > 0))
                );
                assert!(
                    actions
                        .iter()
                        .all(|action| matches!(action, Action::Move(_, _) | Action::Dpad(_)))
                );
            }
            assert!(controller.keyboard.open);
        }
        controller.update(Some(&neutral), TICK, &mut settings);
        assert_eq!(
            controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings),
            vec![Action::OnScreenKeyboard(false)]
        );
        assert_eq!(settings.mode, ControlMode::Game);
        assert!(
            controller
                .update(
                    Some(&GamepadFrame {
                        x: 1.0,
                        ..pressed(Button::South)
                    }),
                    TICK,
                    &mut settings
                )
                .is_empty()
        );
    }
}

#[test]
fn game_keyboard_is_available_even_when_desktop_controls_were_paused() {
    use gamepad_desktop::control::ControlMode;

    let mut controller = MouseController::default();
    let mut settings = ControlSettings {
        enabled: false,
        mode: ControlMode::Game,
        ..Default::default()
    };
    let neutral = GamepadFrame::default();
    controller.update(Some(&neutral), TICK, &mut settings);
    controller.update(Some(&neutral), TICK, &mut settings);
    assert_eq!(
        controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings),
        vec![Action::OnScreenKeyboard(true)]
    );
    assert!(controller.keyboard.open);
    assert!(!settings.enabled);
}

#[test]
fn native_keyboard_allows_only_pointer_and_left_click_in_both_modes() {
    use gamepad_desktop::control::ControlMode;

    for mode in [ControlMode::Desktop, ControlMode::Game] {
        let mut controller = MouseController::default();
        let mut settings = ControlSettings {
            mode,
            ..Default::default()
        };
        let neutral = GamepadFrame::default();
        controller.update(Some(&neutral), TICK, &mut settings);
        controller.update(Some(&neutral), TICK, &mut settings);
        controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
        controller.update(Some(&neutral), TICK, &mut settings);
        for button in [
            Button::South,
            Button::East,
            Button::West,
            Button::North,
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
            Button::LeftShoulder,
            Button::RightShoulder,
            Button::LeftStick,
            Button::Start,
            Button::Select,
        ] {
            let frame = GamepadFrame {
                x: 1.0,
                y: 1.0,
                scroll: 1.0,
                ..pressed(button)
            };
            let actions = controller.update(Some(&frame), TICK, &mut settings);
            assert!(
                actions
                    .iter()
                    .any(|action| matches!(action, Action::Move(x, y) if *x > 0 && *y > 0))
            );
            assert!(
                actions.iter().all(|action| matches!(
                    action,
                    Action::Move(_, _) | Action::Mouse(MouseButton::Left, true) | Action::Dpad(_)
                )),
                "native keyboard leaked {button:?}: {actions:?}"
            );
            assert_eq!(
                actions
                    .iter()
                    .any(|action| matches!(action, Action::Dpad(_))),
                matches!(
                    button,
                    Button::Up | Button::Down | Button::Left | Button::Right
                )
            );
            assert_eq!(
                actions.contains(&Action::Mouse(MouseButton::Left, true)),
                matches!(button, Button::South)
            );
            let released = controller.update(Some(&neutral), TICK, &mut settings);
            assert_eq!(
                released,
                if matches!(button, Button::South) {
                    vec![Action::Mouse(MouseButton::Left, false)]
                } else {
                    vec![]
                }
            );
            assert!(controller.keyboard.open);
        }
        assert_eq!(settings.mode, mode);
    }
}

#[test]
fn keyboard_navigation_honors_repeat_delay_at_different_poll_rates() {
    for tick_ms in [8, 20] {
        let (mut controller, mut settings) = ready();
        controller.keyboard.settings.repeat_delay_ms = 600;
        controller.set_system_keyboard_uses_gamepad(true);
        controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
        controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
        let right = pressed(Button::Right);
        let tick = Duration::from_millis(tick_ms);
        controller.update(Some(&right), tick, &mut settings);
        let selected = controller.keyboard.selected_key();

        for _ in 0..(300 / tick_ms) {
            controller.update(Some(&right), tick, &mut settings);
            assert_eq!(
                controller.keyboard.selected_key(),
                selected,
                "holding D-pad must not move every {tick_ms} ms"
            );
        }
    }
}

#[test]
fn ctrl_enter_uses_a_single_chord_and_releases_the_virtual_modifier() {
    use gamepad_desktop::{control::KeyboardInput, keyboard::KeyboardModifiers};
    let (mut controller, mut settings) = ready();
    controller.set_system_keyboard_uses_gamepad(true);
    controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    controller.keyboard.modifiers.control = true;
    controller.keyboard.selected_row = 3;
    controller.keyboard.selected_column = 12; // Enter
    let actions = controller.update(Some(&pressed(Button::South)), TICK, &mut settings);
    assert_eq!(
        actions,
        vec![Action::KeyboardChord(
            KeyboardInput::Enter,
            KeyboardModifiers {
                control: true,
                ..Default::default()
            }
        )]
    );
    assert_eq!(controller.keyboard.modifiers, KeyboardModifiers::default());
}

#[test]
fn keyboard_repeat_can_be_disabled_without_disabling_single_presses() {
    let (mut controller, mut settings) = ready();
    controller.set_system_keyboard_uses_gamepad(true);
    controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    controller.keyboard.settings.repeat_enabled = false;
    let right = pressed(Button::Right);
    controller.update(Some(&right), TICK, &mut settings);
    let column = controller.keyboard.selected_column;
    for _ in 0..200 {
        controller.update(Some(&right), TICK, &mut settings);
    }
    assert_eq!(controller.keyboard.selected_column, column);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    controller.update(Some(&right), TICK, &mut settings);
    assert_eq!(controller.keyboard.selected_column, column + 1);
}

#[test]
fn keyboard_repeat_uses_the_configured_delay_and_interval() {
    for tick_ms in [8, 20] {
        let (mut controller, mut settings) = ready();
        controller.keyboard.settings.repeat_delay_ms = 600;
        controller.keyboard.settings.repeat_interval_ms = 400;
        controller.set_system_keyboard_uses_gamepad(true);
        controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
        controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
        let right = pressed(Button::Right);
        let tick = Duration::from_millis(tick_ms);
        let start = controller.keyboard.selected_column;
        controller.update(Some(&right), tick, &mut settings);
        for _ in 0..(1200 / tick_ms) {
            controller.update(Some(&right), tick, &mut settings);
        }
        assert_eq!(controller.keyboard.selected_column, start + 3);
    }
}

#[test]
fn keyboard_settings_are_operated_by_gamepad_without_typing_into_the_app() {
    let (mut controller, mut settings) = ready();
    controller.set_system_keyboard_uses_gamepad(true);
    controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    controller.update(Some(&pressed(Button::Start)), TICK, &mut settings);
    assert!(controller.keyboard.customizing);
    let width = controller.keyboard.settings.width;
    assert!(
        controller
            .update(Some(&pressed(Button::Right)), TICK, &mut settings)
            .is_empty()
    );
    assert_eq!(controller.keyboard.settings.width, width + 20);
    for _ in 0..100 {
        assert!(
            controller
                .update(Some(&pressed(Button::Right)), TICK, &mut settings)
                .is_empty()
        );
    }
    assert_eq!(controller.keyboard.settings.width, width + 20);
    assert!(
        controller
            .update(Some(&pressed(Button::South)), TICK, &mut settings)
            .is_empty()
    );
    controller.update(Some(&pressed(Button::Start)), TICK, &mut settings);
    assert!(!controller.keyboard.customizing);
    assert!(settings.enabled);
}

#[test]
fn caps_and_shift_affect_letters_without_changing_number_keys() {
    use gamepad_desktop::control::KeyboardInput;
    let (mut controller, mut settings) = ready();
    controller.set_system_keyboard_uses_gamepad(true);
    controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    controller.keyboard.caps_lock = true;
    assert_eq!(
        controller.update(Some(&pressed(Button::South)), TICK, &mut settings),
        vec![Action::Keyboard(KeyboardInput::Character('A'))]
    );
    controller.update(Some(&pressed(Button::RightShoulder)), TICK, &mut settings);
    assert_eq!(
        controller.update(Some(&pressed(Button::South)), TICK, &mut settings),
        vec![Action::Keyboard(KeyboardInput::Character('a'))]
    );
    controller.keyboard.selected_row = 1;
    controller.keyboard.selected_column = 1;
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    assert_eq!(
        controller.update(Some(&pressed(Button::South)), TICK, &mut settings),
        vec![Action::Keyboard(KeyboardInput::Character('1'))]
    );
}

#[test]
fn disconnect_leaves_keyboard_settings_mode_and_clears_pending_modifiers() {
    let (mut controller, mut settings) = ready();
    controller.set_system_keyboard_uses_gamepad(true);
    controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    controller.update(Some(&pressed(Button::Start)), TICK, &mut settings);
    controller.keyboard.modifiers.alt = true;
    controller.update(None, TICK, &mut settings);
    assert!(!controller.keyboard.customizing);
    assert!(!controller.keyboard.modifiers.alt);
}

#[cfg(target_os = "linux")]
#[test]
fn cosmic_keyboard_owns_input_until_b_or_r3_closes_it() {
    let (mut controller, mut settings) = ready();
    controller.set_system_keyboard_uses_gamepad(true);
    controller.keyboard.presentation = gamepad_desktop::keyboard::KeyboardPresentation::System;
    controller.update(Some(&pressed(Button::RightStick)), TICK, &mut settings);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    let mut input = pressed(Button::South);
    input.buttons[Button::Right as usize] = true;
    input.x = 1.0;
    assert!(
        controller
            .update(Some(&input), TICK, &mut settings)
            .is_empty()
    );
    assert_eq!(
        controller.update(Some(&pressed(Button::East)), TICK, &mut settings),
        vec![Action::OnScreenKeyboard(false)]
    );
    assert!(!controller.keyboard.open);
}
