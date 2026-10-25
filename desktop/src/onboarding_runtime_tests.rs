use super::*;
use crate::control::{Button, MouseButton};
use crate::onboarding::Step;

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
    assert!(
        actions.iter().all(|action| matches!(
            action,
            Action::Mouse(_, false) | Action::OnScreenKeyboard(false) | Action::DpadReleased
        )),
        "unexpected desktop actions: {actions:?}"
    );
    assert!(!state.keyboard.open);
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
    let mut tour = Onboarding::default();
    while tour.step != Step::Game {
        tour.next();
    }
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
