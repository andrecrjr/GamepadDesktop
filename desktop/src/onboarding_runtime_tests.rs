use super::*;
use crate::control::{Button, KeyboardInput, MouseButton};
use crate::onboarding::Step;

fn update_buttons(
    state: &mut RuntimeState,
    controller: &mut MouseController,
    buttons: &[Button],
) -> Vec<Action> {
    let mut frame = GamepadFrame::default();
    for button in buttons {
        frame.buttons[*button as usize] = true;
    }
    update_control(state, controller, Some(&frame), Duration::from_millis(16))
}

fn assert_actions_stay_local(actions: &[Action]) {
    assert!(actions.iter().all(|action| matches!(
        action,
        Action::Mouse(_, false) | Action::OnScreenKeyboard(false) | Action::DpadReleased
    )));
}

#[test]
fn tutorial_keeps_the_native_pointer_visible_even_in_game_mode() {
    let mut state = RuntimeState::default();
    state.settings.mode = ControlMode::Game;
    state.onboarding = Some(Onboarding::default());
    assert_eq!(state.native_mouse_state(true), (false, false));
    state.onboarding = None;
    assert_eq!(state.native_mouse_state(true), (false, true));
}

#[test]
fn skip_preserves_settings_and_finish_remembers_the_chosen_mode() {
    let mut state = RuntimeState::default();
    state.settings.enabled = false;
    state.settings.mode = ControlMode::Game;
    state.onboarding = Some(Onboarding::default());
    let settings = state.settings;
    state.finish_onboarding(None);
    assert_eq!(state.settings, settings);
    assert!(state.onboarding_completed && state.onboarding.is_none());
    state.onboarding = Some(Onboarding::default());
    state.finish_onboarding(Some(ControlMode::Desktop));
    assert_eq!(state.settings.mode, ControlMode::Desktop);
    assert!(state.settings.enabled);
}

#[test]
fn practice_never_sends_movement_keyboard_or_mode_changes_to_the_desktop() {
    let mut state = RuntimeState::default();
    let mut controller = MouseController::default();
    let neutral = GamepadFrame::default();
    update_control(&mut state, &mut controller, Some(&neutral), Duration::ZERO);
    state.onboarding = Some(Onboarding::default());
    let mut frame = GamepadFrame {
        x: 1.0,
        scroll: 1.0,
        ..neutral
    };
    frame.buttons[Button::South as usize] = true;
    let actions = update_control(
        &mut state,
        &mut controller,
        Some(&frame),
        Duration::from_millis(32),
    );
    assert_actions_stay_local(&actions);
    assert!(!state.keyboard.open);
    assert_eq!(state.settings.mode, ControlMode::Desktop);
}

#[test]
fn window_and_desktop_shortcut_lessons_only_change_the_local_preview() {
    let mut state = RuntimeState::default();
    let mut controller = MouseController::default();
    update_buttons(&mut state, &mut controller, &[]);

    let tour = Onboarding::lesson(Step::WindowSwitcher);
    state.onboarding = Some(tour);

    for expected in [1, 2] {
        update_buttons(&mut state, &mut controller, &[]);
        let actions = update_buttons(&mut state, &mut controller, &[Button::RightShoulder]);
        assert_actions_stay_local(&actions);
        let tour = state.onboarding.as_ref().unwrap();
        assert_eq!(tour.step, Step::WindowSwitcher);
        assert!(tour.window_switcher_open);
        assert_eq!(tour.selected_window, expected);
        assert!(!tour.complete, "moving the preview is not the final action");
    }

    update_buttons(&mut state, &mut controller, &[]);
    let actions = update_buttons(&mut state, &mut controller, &[Button::West]);
    assert_actions_stay_local(&actions);
    assert!(!state.onboarding.as_ref().unwrap().window_switcher_open);
    assert!(state.onboarding.as_ref().unwrap().complete);

    update_buttons(&mut state, &mut controller, &[]);
    update_buttons(&mut state, &mut controller, &[Button::South]);
    assert_eq!(
        state.onboarding.as_ref().unwrap().step,
        Step::DesktopShortcuts
    );

    for (index, (shoulder, expected)) in [
        (Button::LeftShoulder, KeyboardInput::PrintScreen),
        (Button::RightShoulder, KeyboardInput::Function(11)),
    ]
    .into_iter()
    .enumerate()
    {
        update_buttons(&mut state, &mut controller, &[]);
        let actions = update_buttons(&mut state, &mut controller, &[Button::Start, shoulder]);
        assert_actions_stay_local(&actions);
        let tour = state.onboarding.as_ref().unwrap();
        assert_eq!(tour.step, Step::DesktopShortcuts);
        assert_eq!(tour.preview_shortcut, Some(expected));
        assert_eq!(tour.complete, index == 1);
    }
    assert_eq!(state.settings.mode, ControlMode::Desktop);
}

#[test]
fn reopening_the_tour_releases_a_drag_and_requires_neutral_before_resuming() {
    let mut state = RuntimeState::default();
    let mut controller = MouseController::default();
    let neutral = GamepadFrame::default();
    update_control(&mut state, &mut controller, Some(&neutral), Duration::ZERO);
    let mut held = neutral;
    held.buttons[Button::South as usize] = true;
    let actions = update_control(&mut state, &mut controller, Some(&held), Duration::ZERO);
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, Action::Mouse(MouseButton::Left, true)))
    );
    state.onboarding = Some(Onboarding::default());
    let actions = update_control(&mut state, &mut controller, Some(&held), Duration::ZERO);
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, Action::Mouse(MouseButton::Left, false)))
    );
    state.onboarding = None;
    let actions = update_control(&mut state, &mut controller, Some(&held), Duration::ZERO);
    assert!(
        !actions
            .iter()
            .any(|action| matches!(action, Action::Mouse(_, true)))
    );
}

#[test]
fn switching_modes_in_a_lesson_preserves_the_real_mode_and_pause_setting() {
    let mut state = RuntimeState::default();
    let tour = Onboarding::lesson(Step::Game);
    state.onboarding = Some(tour);
    let settings = state.settings;
    let mut controller = MouseController::default();
    let neutral = GamepadFrame::default();
    let mut select = neutral;
    select.buttons[Button::Select as usize] = true;
    for frame in [neutral, neutral, select, neutral, select, neutral] {
        update_control(
            &mut state,
            &mut controller,
            Some(&frame),
            Duration::from_millis(16),
        );
    }
    assert_eq!(state.settings, settings);
    assert!(state.onboarding.as_ref().unwrap().complete);
}
