use gamepad_desktop::{
    control::{Button, ControlMode, GamepadFrame},
    onboarding::{Onboarding, Step},
};
use std::time::Duration;

const TICK: Duration = Duration::from_millis(16);

fn frame(button: Button) -> GamepadFrame {
    let mut frame = GamepadFrame::default();
    frame.buttons[button as usize] = true;
    frame
}

fn at(step: Step) -> Onboarding {
    let mut tour = Onboarding::lesson(step);
    tour.update(Some(&GamepadFrame::default()), TICK);
    tour.update(Some(&GamepadFrame::default()), TICK);
    tour
}

fn tap(tour: &mut Onboarding, button: Button) {
    tour.update(Some(&frame(button)), TICK);
    tour.update(Some(&GamepadFrame::default()), TICK);
}

#[test]
fn every_lesson_can_be_skipped_without_a_controller() {
    let mut tour = Onboarding::default();
    for _ in 1..Step::ALL.len() {
        tour.next();
    }
    assert_eq!(tour.step, Step::Ready);
    for _ in 1..Step::ALL.len() {
        tour.back();
    }
    assert_eq!(tour.step, Step::Welcome);
}

#[test]
fn holding_next_does_not_skip_multiple_lessons() {
    let mut tour = at(Step::Welcome);
    let next = frame(Button::RightShoulder);
    for _ in 0..30 {
        tour.update(Some(&next), TICK);
    }
    assert_eq!(tour.step, Step::Pointer);
    tour.update(Some(&GamepadFrame::default()), TICK);
    tap(&mut tour, Button::RightShoulder);
    assert_eq!(tour.step, Step::Click);
}

#[test]
fn practice_uses_the_actual_mouse_and_keyboard_bindings() {
    let mut tour = at(Step::Pointer);
    let moving = GamepadFrame {
        x: 0.8,
        ..GamepadFrame::default()
    };
    for _ in 0..15 {
        tour.update(Some(&moving), TICK);
    }
    assert!(tour.complete);

    tour.next();
    tour.update(Some(&GamepadFrame::default()), TICK);
    tap(&mut tour, Button::South);
    assert!(tour.complete);

    let mut tour = at(Step::OpenKeyboard);
    tap(&mut tour, Button::RightStick);
    assert!(tour.complete && tour.keyboard().open);
    tour.next();
    tour.update(Some(&GamepadFrame::default()), TICK);
    tap(&mut tour, Button::Right);
    tap(&mut tour, Button::South);
    assert!(tour.complete);
    assert!(!tour.typed.is_empty());
    tour.next();
    tour.update(Some(&GamepadFrame::default()), TICK);
    tap(&mut tour, Button::RightStick);
    assert!(tour.complete && !tour.keyboard().open);
}

#[test]
fn switching_modes_and_pausing_are_distinct_gestures() {
    for (step, mode) in [
        (Step::Game, ControlMode::Game),
        (Step::Desktop, ControlMode::Desktop),
    ] {
        let mut tour = at(step);
        tap(&mut tour, Button::Select);
        assert!(!tour.complete);
        tap(&mut tour, Button::Select);
        assert!(tour.complete);
        assert_eq!(tour.practice_settings().mode, mode);
    }
    let mut tour = at(Step::Pause);
    let mut chord = frame(Button::Start);
    chord.buttons[Button::Select as usize] = true;
    tour.update(Some(&chord), TICK);
    assert!(!tour.practice_settings().enabled);
    assert!(!tour.complete);
    tour.update(Some(&GamepadFrame::default()), TICK);
    tour.update(Some(&chord), TICK);
    assert!(tour.practice_settings().enabled && tour.complete);
    assert_eq!(tour.practice_settings().mode, ControlMode::Desktop);
}

#[test]
fn every_practice_has_a_working_demo_without_hardware() {
    for step in [
        Step::Pointer,
        Step::Click,
        Step::RightClick,
        Step::Drag,
        Step::Scroll,
        Step::OpenKeyboard,
        Step::Type,
        Step::CloseKeyboard,
        Step::Game,
        Step::Desktop,
        Step::WindowSwitcher,
        Step::DesktopShortcuts,
        Step::Pause,
    ] {
        let mut tour = at(step);
        tour.toggle_demo();
        for _ in 0..200 {
            tour.update_demo(TICK);
        }
        assert!(tour.complete, "demo failed for {step:?}");
        assert_eq!(tour.step, step, "a demo must not advance the lesson");
    }
}

#[test]
fn disconnecting_does_not_complete_a_lesson_or_exit_the_tour() {
    let mut tour = at(Step::Type);
    assert_eq!(tour.update(None, TICK), None);
    assert!(!tour.complete);
    assert_eq!(tour.step, Step::Type);
}

#[test]
fn demonstrations_freeze_at_the_successful_result() {
    let mut keyboard = at(Step::OpenKeyboard);
    let mut game = at(Step::Game);
    keyboard.toggle_demo();
    game.toggle_demo();
    for _ in 0..200 {
        keyboard.update_demo(TICK);
        game.update_demo(TICK);
    }
    assert!(keyboard.complete && keyboard.keyboard().open);
    assert!(game.complete);
    assert_eq!(game.practice_settings().mode, ControlMode::Game);
}

#[test]
fn demo_survives_disconnected_hardware_polling_between_ui_frames() {
    let mut tour = at(Step::Game);
    tour.toggle_demo();
    for _ in 0..200 {
        tour.update(None, TICK);
        tour.update_demo(TICK);
    }
    assert!(tour.complete);
    assert_eq!(tour.practice_settings().mode, ControlMode::Game);
}

#[test]
fn left_click_requires_press_and_release_over_the_target() {
    let mut tour = at(Step::Click);
    tap(&mut tour, Button::East);
    assert!(
        !tour.complete,
        "right click cannot complete the left-click lesson"
    );
    tour.update(Some(&frame(Button::South)), TICK);
    assert!(
        !tour.complete,
        "holding the button is not a completed click"
    );
    tour.update(Some(&GamepadFrame::default()), TICK);
    assert!(tour.complete);
}

#[test]
fn right_click_uses_the_right_face_button_and_opens_only_on_release() {
    let mut tour = at(Step::RightClick);
    tap(&mut tour, Button::South);
    assert!(!tour.complete);
    tour.update(Some(&frame(Button::East)), TICK);
    assert!(!tour.complete);
    tour.update(Some(&GamepadFrame::default()), TICK);
    assert!(tour.complete);
}

#[test]
fn dragging_requires_picking_up_moving_and_releasing_at_the_destination() {
    let mut tour = at(Step::Drag);
    tap(&mut tour, Button::South);
    assert!(!tour.complete);
    let mut held = frame(Button::South);
    tour.update(Some(&held), TICK);
    assert!(tour.mouse_held);
    held.x = 0.8;
    for _ in 0..100 {
        if tour.cursor[0] >= 0.72 {
            break;
        }
        tour.update(Some(&held), TICK);
    }
    assert!(!tour.complete, "must release at the destination");
    assert!(tour.item_position[0] > 0.6);
    tour.update(Some(&GamepadFrame::default()), TICK);
    assert!(tour.complete);
    assert!(!tour.mouse_held);
}

#[test]
fn moving_without_holding_or_with_the_wrong_button_cannot_drag() {
    let mut tour = at(Step::Drag);
    let mut moving = frame(Button::East);
    moving.x = 0.8;
    for _ in 0..50 {
        tour.update(Some(&moving), TICK);
    }
    assert_eq!(tour.item_position, [0.3, 0.5]);
    tap(&mut tour, Button::South);
    assert!(!tour.complete);
    assert!(!tour.mouse_held);
}

#[test]
fn disconnecting_releases_the_practice_drag_without_completing_it() {
    let mut tour = at(Step::Drag);
    tour.update(Some(&frame(Button::South)), TICK);
    assert!(tour.mouse_held);
    tour.update(None, TICK);
    assert!(!tour.mouse_held && !tour.complete);
}

#[test]
fn releasing_a_drag_before_the_destination_does_not_complete_it() {
    let mut tour = at(Step::Drag);
    let mut held = frame(Button::South);
    tour.update(Some(&held), TICK);
    held.x = 0.8;
    for _ in 0..8 {
        tour.update(Some(&held), TICK);
    }
    tour.update(Some(&GamepadFrame::default()), TICK);
    assert!(!tour.complete && !tour.mouse_held);
}

#[test]
fn clicking_outside_the_target_does_not_complete_the_lesson() {
    for step in [Step::Click, Step::RightClick] {
        let mut tour = at(step);
        tour.cursor = [0.1, 0.1];
        let button = if step == Step::Click {
            Button::South
        } else {
            Button::East
        };
        tour.update(Some(&frame(button)), TICK);
        tour.cursor = [0.72, 0.5];
        tour.update(Some(&GamepadFrame::default()), TICK);
        assert!(
            !tour.complete,
            "a click must start over the target: {step:?}"
        );
    }
}

#[test]
fn quick_start_follows_only_the_core_lessons() {
    let mut tour = Onboarding::default();
    for expected in [
        Step::Welcome,
        Step::Pointer,
        Step::Click,
        Step::Scroll,
        Step::OpenKeyboard,
        Step::Type,
        Step::CloseKeyboard,
        Step::Game,
        Step::Desktop,
        Step::Ready,
    ] {
        assert_eq!(tour.step, expected);
        tour.next();
    }
    assert_eq!(tour.step, Step::Ready);
}

#[test]
fn optional_lessons_have_their_own_route_and_can_be_revisited() {
    let mut tour = Onboarding::lesson(Step::RightClick);
    for expected in [
        Step::RightClick,
        Step::Drag,
        Step::WindowSwitcher,
        Step::DesktopShortcuts,
        Step::Pause,
    ] {
        assert_eq!(tour.step, expected);
        assert!(tour.extra_lessons);
        tour.next();
    }
    assert_eq!(tour.step, Step::Ready);
    tour.back();
    assert_eq!(tour.step, Step::Pause);
    for _ in 0..10 {
        tour.back();
    }
    assert_eq!(tour.step, Step::RightClick);
}

#[test]
fn quick_start_groups_related_practices_into_five_stages() {
    let mut tour = Onboarding::default();
    for (step, stage) in [
        (Step::Welcome, 0),
        (Step::Pointer, 0),
        (Step::Click, 0),
        (Step::Scroll, 1),
        (Step::OpenKeyboard, 2),
        (Step::Type, 2),
        (Step::CloseKeyboard, 2),
        (Step::Game, 3),
        (Step::Desktop, 3),
        (Step::Ready, 4),
    ] {
        assert_eq!(tour.step, step);
        assert_eq!(tour.progress(), (stage, 5));
        tour.next();
    }
}
