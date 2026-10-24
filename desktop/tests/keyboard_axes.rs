use gamepad_desktop::control::{Button, ControlSettings, GamepadFrame, MouseController};
use gamepad_desktop::onboarding::{Onboarding, Step};
use std::time::Duration;

const TICK: Duration = Duration::from_millis(10);

fn keyboard() -> (MouseController, ControlSettings) {
    let mut controller = MouseController::default();
    let mut settings = ControlSettings::default();
    controller.set_system_keyboard_uses_gamepad(true);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    let mut open = GamepadFrame::default();
    open.buttons[Button::RightStick as usize] = true;
    controller.update(Some(&open), TICK, &mut settings);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    controller.keyboard.selected_row = 3;
    controller.keyboard.selected_column = 5;
    (controller, settings)
}

fn position(controller: &MouseController) -> (usize, usize) {
    (
        controller.keyboard.selected_row,
        controller.keyboard.selected_column,
    )
}

#[test]
fn left_stick_navigates_in_all_four_directions_without_desktop_actions() {
    for (x, y, row, column) in [
        (0.8, 0.0, 3, 6),
        (-0.8, 0.0, 3, 4),
        (0.0, -0.8, 2, 5),
        (0.0, 0.8, 4, 5),
    ] {
        let (mut controller, mut settings) = keyboard();
        let actions = controller.update(
            Some(&GamepadFrame {
                x,
                y,
                ..GamepadFrame::default()
            }),
            TICK,
            &mut settings,
        );
        assert!(actions.is_empty(), "navigation leaked actions: {actions:?}");
        assert_eq!(controller.keyboard.selected_row, row);
        if x != 0.0 {
            assert_eq!(controller.keyboard.selected_column, column);
        }
    }
}

#[test]
fn stick_drift_invalid_axes_and_configured_deadzone_do_not_move_selection() {
    let (mut controller, mut settings) = keyboard();
    settings.deadzone = 0.5;
    for x in [
        0.0,
        0.1,
        -0.49,
        0.49,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        controller.update(
            Some(&GamepadFrame {
                x,
                ..GamepadFrame::default()
            }),
            TICK,
            &mut settings,
        );
        assert_eq!(position(&controller), (3, 5));
    }
    controller.update(
        Some(&GamepadFrame {
            x: 0.8,
            ..GamepadFrame::default()
        }),
        TICK,
        &mut settings,
    );
    assert_eq!(position(&controller), (3, 6));
}

#[test]
fn stick_uses_the_keyboard_repeat_settings() {
    let (mut controller, mut settings) = keyboard();
    controller.keyboard.settings.repeat_delay_ms = 400;
    controller.keyboard.settings.repeat_interval_ms = 150;
    let held = GamepadFrame {
        x: 0.8,
        ..GamepadFrame::default()
    };
    controller.update(Some(&held), TICK, &mut settings);
    assert_eq!(position(&controller), (3, 6));
    for _ in 0..35 {
        controller.update(Some(&held), TICK, &mut settings);
    }
    assert_eq!(position(&controller), (3, 6));
    for _ in 0..10 {
        controller.update(Some(&held), TICK, &mut settings);
    }
    assert_eq!(position(&controller), (3, 7));
    for _ in 0..15 {
        controller.update(Some(&held), TICK, &mut settings);
    }
    assert_eq!(position(&controller), (3, 8));
}

#[test]
fn disabling_repeat_requires_centering_the_stick_before_the_next_move() {
    let (mut controller, mut settings) = keyboard();
    controller.keyboard.settings.repeat_enabled = false;
    let held = GamepadFrame {
        x: 0.8,
        ..GamepadFrame::default()
    };
    for _ in 0..100 {
        controller.update(Some(&held), TICK, &mut settings);
    }
    assert_eq!(position(&controller), (3, 6));
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    controller.update(Some(&held), TICK, &mut settings);
    assert_eq!(position(&controller), (3, 7));
}

#[test]
fn dominant_axis_and_dpad_priority_prevent_double_steps() {
    let (mut controller, mut settings) = keyboard();
    controller.keyboard.settings.repeat_enabled = false;
    controller.update(
        Some(&GamepadFrame {
            x: 0.8,
            y: 0.4,
            ..GamepadFrame::default()
        }),
        TICK,
        &mut settings,
    );
    assert_eq!(position(&controller), (3, 6));
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    let mut mixed = GamepadFrame {
        x: 0.8,
        ..GamepadFrame::default()
    };
    mixed.buttons[Button::Left as usize] = true;
    controller.update(Some(&mixed), TICK, &mut settings);
    assert_eq!(position(&controller), (3, 5));
}

#[test]
fn stick_also_navigates_and_adjusts_keyboard_settings() {
    let (mut controller, mut settings) = keyboard();
    controller.keyboard.customizing = true;
    controller.update(
        Some(&GamepadFrame {
            y: 0.8,
            ..GamepadFrame::default()
        }),
        TICK,
        &mut settings,
    );
    assert_eq!(controller.keyboard.selected_setting, 1);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    controller.update(
        Some(&GamepadFrame {
            y: -0.8,
            ..GamepadFrame::default()
        }),
        TICK,
        &mut settings,
    );
    assert_eq!(controller.keyboard.selected_setting, 0);
    controller.update(Some(&GamepadFrame::default()), TICK, &mut settings);
    let width = controller.keyboard.settings.width;
    controller.update(
        Some(&GamepadFrame {
            x: 0.8,
            ..GamepadFrame::default()
        }),
        TICK,
        &mut settings,
    );
    assert!(controller.keyboard.settings.width > width);
}

#[test]
fn welcome_practice_accepts_analog_selection_before_typing() {
    let mut tour = Onboarding::default();
    while tour.step != Step::Type {
        tour.next();
    }
    tour.update(Some(&GamepadFrame::default()), TICK);
    tour.update(
        Some(&GamepadFrame {
            x: 0.8,
            ..GamepadFrame::default()
        }),
        TICK,
    );
    tour.update(Some(&GamepadFrame::default()), TICK);
    let mut confirm = GamepadFrame::default();
    confirm.buttons[Button::South as usize] = true;
    tour.update(Some(&confirm), TICK);
    assert!(tour.complete && !tour.typed.is_empty());
}
