//! Practice uses the real bindings, but consumes actions locally instead of sending them to the OS.
use std::time::Duration;

use crate::{
    control::{
        Action, Button, ControlMode, ControlSettings, GamepadFrame, KeyboardInput, MouseButton,
        MouseController,
    },
    keyboard::KeyboardSession,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Step {
    #[default]
    Welcome,
    Modes,
    Pointer,
    Click,
    RightClick,
    Drag,
    Scroll,
    OpenKeyboard,
    Type,
    CloseKeyboard,
    Game,
    Desktop,
    Pause,
    Ready,
}

impl Step {
    pub const ALL: [Self; 14] = [
        Self::Welcome,
        Self::Modes,
        Self::Pointer,
        Self::Click,
        Self::RightClick,
        Self::Drag,
        Self::Scroll,
        Self::OpenKeyboard,
        Self::Type,
        Self::CloseKeyboard,
        Self::Game,
        Self::Desktop,
        Self::Pause,
        Self::Ready,
    ];

    pub fn chapter(self) -> (usize, usize, usize) {
        match self {
            Self::Welcome => (0, 1, 1),
            Self::Modes => (1, 1, 1),
            Self::Pointer => (2, 1, 5),
            Self::Click => (2, 2, 5),
            Self::RightClick => (2, 3, 5),
            Self::Drag => (2, 4, 5),
            Self::Scroll => (2, 5, 5),
            Self::OpenKeyboard => (3, 1, 3),
            Self::Type => (3, 2, 3),
            Self::CloseKeyboard => (3, 3, 3),
            Self::Game => (4, 1, 3),
            Self::Desktop => (4, 2, 3),
            Self::Pause => (4, 3, 3),
            Self::Ready => (5, 1, 1),
        }
    }

    pub fn is_practice(self) -> bool {
        !matches!(self, Self::Welcome | Self::Modes | Self::Ready)
    }
}

#[derive(Clone, Copy)]
pub enum Command {
    Next,
    Back,
    ToggleDemo,
    Skip,
    Finish(ControlMode),
    Language(crate::i18n::Locale),
    Scale(crate::preferences::UiScale),
}

#[derive(Clone)]
pub struct Onboarding {
    pub step: Step,
    pub complete: bool,
    pub demonstrating: bool,
    pub cursor: [f32; 2],
    pub scroll: f32,
    pub mouse_held: bool,
    pub item_position: [f32; 2],
    pub typed: String,
    pub start_mode: ControlMode,
    controller: MouseController,
    settings: ControlSettings,
    previous: Option<GamepadFrame>,
    moved_selection: bool,
    paused_once: bool,
    demo_time: f64,
}

impl Default for Onboarding {
    fn default() -> Self {
        let mut tour = Self {
            step: Step::Welcome,
            complete: false,
            demonstrating: false,
            cursor: [0.3, 0.5],
            scroll: 0.0,
            mouse_held: false,
            item_position: [0.3, 0.5],
            typed: String::new(),
            start_mode: ControlMode::Desktop,
            controller: MouseController::default(),
            settings: ControlSettings::default(),
            previous: None,
            moved_selection: false,
            paused_once: false,
            demo_time: 0.0,
        };
        tour.reset_lesson();
        tour
    }
}

impl Onboarding {
    pub fn next(&mut self) {
        self.step = Step::ALL[(self.step as usize + 1).min(Step::ALL.len() - 1)];
        self.reset_lesson();
    }

    pub fn back(&mut self) {
        self.step = Step::ALL[(self.step as usize).saturating_sub(1)];
        self.reset_lesson();
    }

    pub fn toggle_demo(&mut self) {
        self.demonstrating = !self.demonstrating;
        self.previous = None;
        self.reset_lesson();
    }

    pub fn keyboard(&self) -> &KeyboardSession {
        &self.controller.keyboard
    }
    pub fn practice_settings(&self) -> ControlSettings {
        self.settings
    }

    fn reset_lesson(&mut self) {
        let locale = self.controller.keyboard.locale;
        self.controller = MouseController::default();
        self.controller.set_system_keyboard_uses_gamepad(true);
        self.controller.keyboard.locale = locale;
        self.controller.keyboard.open = matches!(self.step, Step::Type | Step::CloseKeyboard);
        self.controller.keyboard.selected_row = 2;
        self.controller.keyboard.selected_column = 1;
        self.settings = ControlSettings {
            mode: if self.step == Step::Desktop {
                ControlMode::Game
            } else {
                ControlMode::Desktop
            },
            ..ControlSettings::default()
        };
        self.complete = false;
        self.cursor = if matches!(self.step, Step::Click | Step::RightClick) {
            [0.72, 0.5]
        } else {
            [0.3, 0.5]
        };
        self.scroll = 0.0;
        self.mouse_held = false;
        self.item_position = [0.3, 0.5];
        if self.step != Step::CloseKeyboard {
            self.typed.clear();
        }
        self.moved_selection = false;
        self.paused_once = false;
        self.demo_time = 0.0;
    }

    pub fn update(
        &mut self,
        frame: Option<&GamepadFrame>,
        elapsed: Duration,
    ) -> Option<ControlMode> {
        let Some(frame) = frame else {
            self.previous = None;
            if !self.demonstrating {
                self.mouse_held = false;
                self.controller.update(None, elapsed, &mut self.settings);
                self.controller.keyboard.open =
                    matches!(self.step, Step::Type | Step::CloseKeyboard);
            }
            return None;
        };
        let edge = |button| {
            self.previous
                .is_some_and(|previous| frame.pressed(button) && !previous.pressed(button))
        };
        let back = edge(Button::LeftShoulder);
        let next = edge(Button::RightShoulder);
        let confirm = edge(Button::South);
        if self.step == Step::Ready {
            if edge(Button::Left) {
                self.start_mode = ControlMode::Desktop;
            }
            if edge(Button::Right) {
                self.start_mode = ControlMode::Game;
            }
        }
        self.previous = Some(*frame);
        if back {
            self.back();
            return None;
        }
        if self.step == Step::Ready {
            return confirm.then_some(self.start_mode);
        }
        if next || confirm && matches!(self.step, Step::Welcome | Step::Modes) {
            self.next();
        } else if !self.demonstrating && self.step.is_practice() {
            self.practice(frame, elapsed);
        }
        None
    }

    pub fn update_demo(&mut self, elapsed: Duration) {
        if !self.demonstrating || !self.step.is_practice() || self.complete {
            return;
        }
        self.demo_time += elapsed.as_secs_f64().min(0.05);
        let time = self.demo_time % 2.4;
        let mut frame = GamepadFrame::default();
        if time > 0.25 {
            match self.step {
                Step::Pointer => {
                    if self.cursor[0] < 0.72 {
                        frame.x = 0.7;
                    }
                }
                Step::Click => frame.buttons[Button::South as usize] = (0.5..0.7).contains(&time),
                Step::RightClick => {
                    frame.buttons[Button::East as usize] = (0.5..0.7).contains(&time)
                }
                Step::Drag => {
                    frame.buttons[Button::South as usize] = (0.4..1.8).contains(&time);
                    if self.mouse_held && self.cursor[0] < 0.72 {
                        frame.x = 0.8;
                    }
                }
                Step::Scroll => frame.scroll = 0.8,
                Step::OpenKeyboard | Step::CloseKeyboard => {
                    frame.buttons[Button::RightStick as usize] = (0.5..0.7).contains(&time)
                }
                Step::Type => {
                    if (0.4..0.5).contains(&time) {
                        frame.x = 0.8;
                    }
                    frame.buttons[Button::South as usize] = (1.0..1.1).contains(&time);
                }
                Step::Game | Step::Desktop => {
                    frame.buttons[Button::Select as usize] =
                        (0.4..0.48).contains(&time) || (0.65..0.73).contains(&time)
                }
                Step::Pause => {
                    let pressed = (0.4..0.6).contains(&time) || (1.0..1.2).contains(&time);
                    frame.buttons[Button::Select as usize] = pressed;
                    frame.buttons[Button::Start as usize] = pressed;
                }
                _ => {}
            }
        }
        self.practice(&frame, elapsed);
    }

    fn practice(&mut self, frame: &GamepadFrame, elapsed: Duration) {
        let selection = (
            self.controller.keyboard.selected_row,
            self.controller.keyboard.selected_column,
        );
        let actions = self
            .controller
            .update(Some(frame), elapsed, &mut self.settings);
        self.moved_selection |= selection
            != (
                self.controller.keyboard.selected_row,
                self.controller.keyboard.selected_column,
            );
        for action in actions {
            match action {
                Action::Move(x, y) => {
                    self.cursor[0] = (self.cursor[0] + x as f32 / 900.0).clamp(0.04, 0.96);
                    self.cursor[1] = (self.cursor[1] + y as f32 / 600.0).clamp(0.08, 0.92);
                    if self.step == Step::Drag && self.mouse_held {
                        self.item_position = self.cursor;
                    }
                    if self.step == Step::Pointer {
                        self.complete |= (self.cursor[0] - 0.3).abs() > 0.12
                            || (self.cursor[1] - 0.5).abs() > 0.12;
                    }
                }
                Action::Mouse(button, pressed)
                    if matches!(
                        (self.step, button),
                        (Step::Click | Step::Drag, MouseButton::Left)
                            | (Step::RightClick, MouseButton::Right)
                    ) =>
                {
                    if pressed {
                        let target = if self.step == Step::Drag {
                            self.item_position
                        } else {
                            [0.72, 0.5]
                        };
                        self.mouse_held = near(self.cursor, target);
                    } else {
                        let position = if self.step == Step::Drag {
                            self.item_position
                        } else {
                            self.cursor
                        };
                        self.complete |= self.mouse_held && near(position, [0.72, 0.5]);
                        self.mouse_held = false;
                    }
                }
                Action::Scroll(amount) if self.step == Step::Scroll => {
                    self.scroll = (self.scroll + amount as f32 * 0.12).clamp(-1.0, 1.0);
                    self.complete |= self.scroll.abs() >= 0.24;
                }
                Action::OnScreenKeyboard(open) => {
                    self.complete |= matches!(
                        (self.step, open),
                        (Step::OpenKeyboard, true) | (Step::CloseKeyboard, false)
                    );
                }
                Action::Keyboard(KeyboardInput::Character(character))
                    if self.step == Step::Type =>
                {
                    if self.typed.chars().count() < 24 {
                        self.typed.push(character);
                    }
                    self.complete |= self.moved_selection;
                }
                Action::Keyboard(KeyboardInput::Backspace) if self.step == Step::Type => {
                    self.typed.pop();
                }
                _ => {}
            }
        }
        self.paused_once |= !self.settings.enabled;
        self.complete |= match self.step {
            Step::Game => self.settings.mode == ControlMode::Game,
            Step::Desktop => self.settings.mode == ControlMode::Desktop,
            Step::Pause => self.paused_once && self.settings.enabled,
            _ => false,
        };
    }
}

fn near(position: [f32; 2], target: [f32; 2]) -> bool {
    (position[0] - target[0]).abs() < 0.14 && (position[1] - target[1]).abs() < 0.22
}
