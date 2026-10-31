use std::time::Duration;

use crate::keyboard::{KeyboardKey, KeyboardModifiers, KeyboardSession};

const REPEAT_INITIAL_DELAY: f64 = 0.4;
const DIRECTION_REPEAT_INTERVAL: f64 = 0.08;
const SELECT_DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(400);
const NEXT_WINDOW_PRESS_GUARD: Duration = Duration::from_millis(32);

pub const MOUSE_SHORTCUTS: &[(&str, &str)] = &[
    (
        "R3 · pressionar analógico direito",
        if cfg!(target_os = "windows") {
            "Abrir / fechar teclado do Windows no modo Desktop"
        } else {
            "Abrir / fechar teclado visual próprio no modo Desktop"
        },
    ),
    (
        "L3 · pressionar analógico esquerdo",
        "Lançador do sistema · Super / Windows",
    ),
    ("Analógico esquerdo", "Mover ponteiro"),
    ("Analógico direito ↕", "Rolar página"),
    (
        "Inferior · A / ✕ / B no Switch",
        "Clique esquerdo · segurar para arrastar",
    ),
    ("Direito · B / ○ / A no Switch", "Clique direito"),
    (
        "Esquerdo · X / □ / Y no Switch",
        "Enter · abrir item selecionado",
    ),
    (
        "Superior · Y / △ / X no Switch",
        "Escape · fechar menu / cancelar",
    ),
    (
        "Direcional",
        if cfg!(target_os = "windows") {
            "Mover a seleção pelo teclado"
        } else {
            "Setas · segurar para repetir"
        },
    ),
    ("LB / L1", "Tab · próximo campo"),
    ("RB / R1", "Alt+Tab · alternar janela"),
    (
        "Start + Select / Menu + View",
        "Pausar / retomar · solte e centralize para retomar",
    ),
    (
        "Select · dois toques rápidos",
        "Alternar entre Desktop e Jogo",
    ),
    ("Select/Back + R3 · modo Jogo", "Abrir / fechar teclado"),
    ("Start/Menu + LB/L1", "Print Screen · iniciar captura"),
    ("Start/Menu + RB/R1", "F11 · tela cheia"),
];

pub const KEYBOARD_SHORTCUTS: &[(&str, &str)] = &[
    (
        "Direcional / analógico esquerdo",
        "Mover a seleção pelo teclado",
    ),
    ("A / botão inferior", "Ativar a tecla selecionada"),
    ("B / botão direito", "Apagar um caractere"),
    ("X / botão esquerdo", "Inserir Enter"),
    ("Y / botão superior", "Inserir Esc"),
    ("L3", "Inserir espaço"),
    ("L1", "Inserir Tab"),
    ("R1", "Ativar ou desativar Shift"),
    ("R3", "Fechar o teclado visual"),
    (
        "Start / Menu / +",
        "Ajustes do teclado · direcional escolhe e altera",
    ),
];

#[derive(Clone, Copy, Debug)]
pub enum Button {
    South,
    East,
    West,
    North,
    Up,
    Down,
    Left,
    Right,
    LeftShoulder,
    RightShoulder,
    RightStick,
    Start,
    LeftStick,
    Select,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GamepadFrame {
    pub x: f64,
    pub y: f64,
    pub scroll: f64,
    pub buttons: [bool; 14],
}

impl GamepadFrame {
    pub fn pressed(&self, button: Button) -> bool {
        self.buttons[button as usize]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlMode {
    #[default]
    Desktop,
    Game,
}

impl ControlMode {
    fn toggled(self) -> Self {
        match self {
            Self::Desktop => Self::Game,
            Self::Game => Self::Desktop,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ControlSettings {
    pub enabled: bool,
    pub mode: ControlMode,
    pub speed: f64,
    pub deadzone: f64,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: ControlMode::Desktop,
            speed: 1800.0,
            deadzone: 0.2,
        }
    }
}

impl ControlSettings {
    pub fn normalized(mut self) -> Self {
        self.speed = self.speed.clamp(100.0, 2400.0);
        self.deadzone = self.deadzone.clamp(0.05, 0.5);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemKey {
    Enter,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Tab,
    Meta,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DpadDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardInput {
    Character(char),
    Function(u8),
    PrintScreen,
    Backspace,
    Enter,
    Escape,
    Space,
    Tab,
    Menu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move(i32, i32),
    Scroll(i32),
    Mouse(MouseButton, bool),
    Key(SystemKey),
    Dpad(DpadDirection),
    DpadReleased,
    Keyboard(KeyboardInput),
    KeyboardChord(KeyboardInput, KeyboardModifiers),
    NextWindow,
    OnScreenKeyboard(bool),
}

#[derive(Clone)]
pub struct MouseController {
    pub keyboard: KeyboardSession,
    previous: GamepadFrame,
    connected: bool,
    armed: bool,
    held: [bool; 2],
    dpad_routing_held: bool,
    remainder: [f64; 3],
    repeat: [f64; 4],
    system_keyboard_uses_gamepad: bool,
    select_tap_remaining: Option<Duration>,
    next_window_press_guard: Duration,
    mode: ControlMode,
}

impl Default for MouseController {
    fn default() -> Self {
        Self {
            keyboard: KeyboardSession::default(),
            previous: GamepadFrame::default(),
            connected: false,
            armed: false,
            held: [false; 2],
            dpad_routing_held: false,
            remainder: [0.0; 3],
            repeat: [0.0; 4],
            system_keyboard_uses_gamepad: false,
            select_tap_remaining: None,
            next_window_press_guard: Duration::ZERO,
            mode: ControlMode::Desktop,
        }
    }
}

impl MouseController {
    pub(crate) fn require_neutral(&mut self) {
        self.armed = false;
    }

    pub fn set_system_keyboard_uses_gamepad(&mut self, enabled: bool) {
        self.system_keyboard_uses_gamepad = enabled;
    }

    pub fn update(
        &mut self,
        frame: Option<&GamepadFrame>,
        elapsed: Duration,
        settings: &mut ControlSettings,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        self.next_window_press_guard = self.next_window_press_guard.saturating_sub(elapsed);
        let Some(frame) = frame else {
            self.disconnect(&mut actions);
            return actions;
        };
        if !self.connected {
            self.previous = *frame;
            self.connected = true;
        }
        if self.mode != settings.mode {
            self.reset_for_mode_change(frame, settings.mode, &mut actions);
            return actions;
        }
        if let Some(remaining) = self.select_tap_remaining {
            self.select_tap_remaining = remaining
                .checked_sub(elapsed)
                .filter(|remaining| !remaining.is_zero());
        }
        let pause_chord = frame.pressed(Button::Start) && frame.pressed(Button::Select);
        let previous_pause_chord =
            self.previous.pressed(Button::Start) && self.previous.pressed(Button::Select);
        let keyboard_chord = settings.mode == ControlMode::Game
            && !pause_chord
            && frame.pressed(Button::Select)
            && frame.pressed(Button::RightStick)
            && !(self.previous.pressed(Button::Select)
                && self.previous.pressed(Button::RightStick));
        if pause_chord {
            self.select_tap_remaining = None;
        }
        if keyboard_chord {
            self.select_tap_remaining = None;
        }
        if settings.mode == ControlMode::Desktop
            && !(self.keyboard.open && self.system_keyboard_uses_gamepad)
            && pause_chord
            && !previous_pause_chord
        {
            settings.enabled = !settings.enabled;
            self.armed = false;
        }
        let select_pressed =
            frame.pressed(Button::Select) && !self.previous.pressed(Button::Select);
        if !self.keyboard.open && !pause_chord && !keyboard_chord && select_pressed {
            if self.select_tap_remaining.is_some() {
                settings.mode = settings.mode.toggled();
                self.mode = settings.mode;
                self.select_tap_remaining = None;
                self.close_keyboard(&mut actions);
                self.release(&mut actions);
                self.armed = false;
                self.previous = *frame;
                return actions;
            }
            self.select_tap_remaining = Some(SELECT_DOUBLE_TAP_WINDOW);
        }
        let keyboard_toggled =
            frame.pressed(Button::RightStick) && !self.previous.pressed(Button::RightStick);
        let keyboard_toggle_requested = keyboard_chord
            || (keyboard_toggled && (settings.mode == ControlMode::Desktop || self.keyboard.open));
        if keyboard_toggle_requested && self.keyboard.open {
            actions.push(self.toggle_keyboard());
            self.release(&mut actions);
            self.require_neutral();
            self.previous = *frame;
            return actions;
        }
        let input_enabled = settings.mode == ControlMode::Game || settings.enabled;
        if !input_enabled || !self.armed {
            if !input_enabled {
                self.close_keyboard(&mut actions);
            }
            self.release(&mut actions);
            self.armed = input_enabled
                && !frame.buttons.iter().any(|pressed| *pressed)
                && [frame.x, frame.y, frame.scroll]
                    .iter()
                    .all(|axis| axis.is_finite() && axis.abs() <= settings.deadzone);
            self.previous = *frame;
            return actions;
        }

        if keyboard_toggle_requested {
            actions.push(self.toggle_keyboard());
            self.release(&mut actions);
            if !self.keyboard.open {
                self.require_neutral();
            }
            self.previous = *frame;
            return actions;
        }
        if self.keyboard.open {
            self.update_keyboard_controls(frame, elapsed, settings, &mut actions);
            self.previous = *frame;
            return actions;
        }
        if settings.mode == ControlMode::Game {
            self.release(&mut actions);
            self.previous = *frame;
            return actions;
        }

        self.update_desktop_controls(frame, elapsed, settings, &mut actions);
        self.previous = *frame;
        actions
    }

    fn disconnect(&mut self, actions: &mut Vec<Action>) {
        self.close_keyboard(actions);
        self.release(actions);
        self.connected = false;
        self.armed = false;
        self.select_tap_remaining = None;
        self.next_window_press_guard = Duration::ZERO;
        self.previous = GamepadFrame::default();
    }

    fn reset_for_mode_change(
        &mut self,
        frame: &GamepadFrame,
        mode: ControlMode,
        actions: &mut Vec<Action>,
    ) {
        self.mode = mode;
        self.select_tap_remaining = None;
        self.next_window_press_guard = Duration::ZERO;
        self.close_keyboard(actions);
        self.release(actions);
        self.armed = false;
        self.previous = *frame;
    }

    fn update_keyboard_controls(
        &mut self,
        frame: &GamepadFrame,
        elapsed: Duration,
        settings: &ControlSettings,
        actions: &mut Vec<Action>,
    ) {
        #[cfg(target_os = "linux")]
        if self.keyboard.presentation == crate::keyboard::KeyboardPresentation::System {
            // COSMIC reads the physical controller itself. Never inject duplicate input.
            self.release(actions);
            if frame.pressed(Button::East) && !self.previous.pressed(Button::East) {
                self.close_keyboard(actions);
                self.require_neutral();
            }
            return;
        }
        if self.system_keyboard_uses_gamepad {
            self.release_mouse(actions);
            self.update_keyboard(frame, elapsed, settings.deadzone, actions);
            return;
        }

        self.move_pointer(frame, elapsed, settings, actions);
        for (index, (button, direction)) in [
            (Button::Up, DpadDirection::Up),
            (Button::Down, DpadDirection::Down),
            (Button::Left, DpadDirection::Left),
            (Button::Right, DpadDirection::Right),
        ]
        .into_iter()
        .enumerate()
        {
            if self.repeat_direction(
                frame.pressed(button),
                self.previous.pressed(button),
                index,
                elapsed,
                REPEAT_INITIAL_DELAY,
                DIRECTION_REPEAT_INTERVAL,
            ) {
                actions.push(Action::Dpad(direction));
            }
        }
        let pressed = frame.pressed(Button::South);
        if pressed != self.held[0] {
            actions.push(Action::Mouse(MouseButton::Left, pressed));
            self.held[0] = pressed;
        }
    }

    fn update_desktop_controls(
        &mut self,
        frame: &GamepadFrame,
        elapsed: Duration,
        settings: &ControlSettings,
        actions: &mut Vec<Action>,
    ) {
        for (index, (button, mouse)) in [
            (Button::South, MouseButton::Left),
            (Button::East, MouseButton::Right),
        ]
        .into_iter()
        .enumerate()
        {
            let pressed = frame.pressed(button);
            if pressed != self.held[index] {
                actions.push(Action::Mouse(mouse, pressed));
                self.held[index] = pressed;
            }
        }
        let start_held = frame.pressed(Button::Start);
        let left_shoulder_pressed =
            frame.pressed(Button::LeftShoulder) && !self.previous.pressed(Button::LeftShoulder);
        let right_shoulder_pressed =
            frame.pressed(Button::RightShoulder) && !self.previous.pressed(Button::RightShoulder);
        if start_held {
            match (left_shoulder_pressed, right_shoulder_pressed) {
                (true, false) => actions.push(Action::Keyboard(KeyboardInput::PrintScreen)),
                (false, true) => actions.push(Action::Keyboard(KeyboardInput::Function(11))),
                _ => {}
            }
        }
        for (button, action) in [
            (Button::West, Action::Key(SystemKey::Enter)),
            (Button::North, Action::Key(SystemKey::Escape)),
            (Button::LeftStick, Action::Key(SystemKey::Meta)),
            (Button::LeftShoulder, Action::Key(SystemKey::Tab)),
            (Button::RightShoulder, Action::NextWindow),
        ] {
            let shoulder_chord =
                start_held && matches!(button, Button::LeftShoulder | Button::RightShoulder);
            if !shoulder_chord && frame.pressed(button) && !self.previous.pressed(button) {
                if action == Action::NextWindow {
                    if !self.next_window_press_guard.is_zero() {
                        continue;
                    }
                    self.next_window_press_guard = NEXT_WINDOW_PRESS_GUARD;
                }
                actions.push(action);
            }
        }
        let seconds = elapsed.as_secs_f64().min(0.05);
        for (index, (button, direction)) in [
            (Button::Up, DpadDirection::Up),
            (Button::Down, DpadDirection::Down),
            (Button::Left, DpadDirection::Left),
            (Button::Right, DpadDirection::Right),
        ]
        .into_iter()
        .enumerate()
        {
            if self.repeat_direction(
                frame.pressed(button),
                self.previous.pressed(button),
                index,
                elapsed,
                REPEAT_INITIAL_DELAY,
                DIRECTION_REPEAT_INTERVAL,
            ) {
                actions.push(Action::Dpad(direction));
                self.dpad_routing_held = true;
            }
        }
        if self.dpad_routing_held
            && [Button::Up, Button::Down, Button::Left, Button::Right]
                .into_iter()
                .all(|button| !frame.pressed(button))
        {
            actions.push(Action::DpadReleased);
            self.dpad_routing_held = false;
        }
        self.move_pointer(frame, elapsed, settings, actions);
        let scroll = accumulate(
            &mut self.remainder[2],
            axis_value(frame.scroll, settings.deadzone) * 10.0 * seconds,
        );
        if scroll != 0 {
            actions.push(Action::Scroll(scroll));
        }
    }

    fn move_pointer(
        &mut self,
        frame: &GamepadFrame,
        elapsed: Duration,
        settings: &ControlSettings,
        actions: &mut Vec<Action>,
    ) {
        let seconds = elapsed.as_secs_f64().min(0.05);
        let x = axis_value(frame.x, settings.deadzone);
        let y = axis_value(frame.y, settings.deadzone);
        let magnitude = x.hypot(y).max(1.0);
        let speed = settings.speed.clamp(100.0, 2400.0);
        let dx = accumulate(&mut self.remainder[0], x / magnitude * speed * seconds);
        let dy = accumulate(&mut self.remainder[1], y / magnitude * speed * seconds);
        if dx != 0 || dy != 0 {
            actions.push(Action::Move(dx, dy));
        }
    }

    fn release(&mut self, actions: &mut Vec<Action>) {
        self.release_mouse(actions);
        if self.dpad_routing_held {
            actions.push(Action::DpadReleased);
            self.dpad_routing_held = false;
        }
        self.remainder = [0.0; 3];
        self.repeat = [0.0; 4];
    }

    fn toggle_keyboard(&mut self) -> Action {
        self.keyboard.open = !self.keyboard.open;
        self.keyboard.status = None;
        self.keyboard.customizing = false;
        self.keyboard.modifiers = KeyboardModifiers::default();
        self.keyboard.shift = false;
        Action::OnScreenKeyboard(self.keyboard.open)
    }

    fn close_keyboard(&mut self, actions: &mut Vec<Action>) {
        if self.keyboard.open {
            actions.push(self.toggle_keyboard());
        }
    }

    fn release_mouse(&mut self, actions: &mut Vec<Action>) {
        for (index, button) in [MouseButton::Left, MouseButton::Right]
            .into_iter()
            .enumerate()
        {
            if self.held[index] {
                actions.push(Action::Mouse(button, false));
            }
        }
        self.held = [false; 2];
    }

    fn repeat_direction(
        &mut self,
        pressed: bool,
        was_pressed: bool,
        index: usize,
        elapsed: Duration,
        initial_delay: f64,
        repeat_interval: f64,
    ) -> bool {
        if !pressed {
            return false;
        }
        if !was_pressed {
            self.repeat[index] = initial_delay;
            return true;
        }
        self.repeat[index] -= elapsed.as_secs_f64().min(0.05);
        if self.repeat[index] > 0.0 {
            return false;
        }
        self.repeat[index] += repeat_interval;
        true
    }

    fn update_keyboard(
        &mut self,
        frame: &GamepadFrame,
        elapsed: Duration,
        deadzone: f64,
        actions: &mut Vec<Action>,
    ) {
        let directions = keyboard_directions(frame, deadzone);
        let previous_directions = keyboard_directions(&self.previous, deadzone);
        if frame.pressed(Button::Start) && !self.previous.pressed(Button::Start) {
            self.keyboard.customizing = !self.keyboard.customizing;
            self.repeat = [f64::from(self.keyboard.settings.repeat_delay_ms) / 1000.0; 4];
            return;
        }
        if self.keyboard.customizing {
            for (index, delta) in [(0, -1), (1, 1)] {
                if directions[index] && !previous_directions[index] {
                    self.keyboard.selected_setting =
                        (self.keyboard.selected_setting as isize + delta).rem_euclid(6) as usize;
                }
            }
            for (index, delta) in [(2, -1), (3, 1)] {
                if directions[index] && !previous_directions[index] {
                    self.keyboard
                        .settings
                        .adjust(self.keyboard.selected_setting, delta);
                }
            }
            return;
        }
        let settings = self.keyboard.settings.normalized();
        for (index, (row_delta, column_delta)) in
            [(-1, 0), (1, 0), (0, -1), (0, 1)].into_iter().enumerate()
        {
            let moved = if settings.repeat_enabled {
                self.repeat_direction(
                    directions[index],
                    previous_directions[index],
                    index,
                    elapsed,
                    f64::from(settings.repeat_delay_ms) / 1000.0,
                    f64::from(settings.repeat_interval_ms) / 1000.0,
                )
            } else {
                directions[index] && !previous_directions[index]
            };
            if moved {
                self.keyboard.move_selection(row_delta, column_delta);
            }
        }

        for (button, input) in [
            (Button::East, KeyboardInput::Backspace),
            (Button::West, KeyboardInput::Enter),
            (Button::North, KeyboardInput::Escape),
            (Button::LeftStick, KeyboardInput::Space),
            (Button::LeftShoulder, KeyboardInput::Tab),
        ] {
            if frame.pressed(button) && !self.previous.pressed(button) {
                actions.push(self.keyboard_action(input));
            }
        }

        if frame.pressed(Button::RightShoulder) && !self.previous.pressed(Button::RightShoulder) {
            self.keyboard.shift = !self.keyboard.shift;
        }
        if frame.pressed(Button::South) && !self.previous.pressed(Button::South) {
            match self.keyboard.selected_key() {
                key @ KeyboardKey::Character(_) => {
                    let character = key.character(self.keyboard.shifted(key)).unwrap();
                    actions.push(self.keyboard_action(KeyboardInput::Character(character)));
                    self.keyboard.shift = false;
                }
                KeyboardKey::Function(number) => {
                    actions.push(self.keyboard_action(KeyboardInput::Function(number)))
                }
                KeyboardKey::Backspace => {
                    actions.push(self.keyboard_action(KeyboardInput::Backspace))
                }
                KeyboardKey::Enter => actions.push(self.keyboard_action(KeyboardInput::Enter)),
                KeyboardKey::Escape => actions.push(self.keyboard_action(KeyboardInput::Escape)),
                KeyboardKey::Shift => self.keyboard.shift = !self.keyboard.shift,
                KeyboardKey::CapsLock => self.keyboard.caps_lock = !self.keyboard.caps_lock,
                KeyboardKey::Control => {
                    self.keyboard.modifiers.control = !self.keyboard.modifiers.control
                }
                KeyboardKey::Alt => self.keyboard.modifiers.alt = !self.keyboard.modifiers.alt,
                KeyboardKey::Meta => self.keyboard.modifiers.meta = !self.keyboard.modifiers.meta,
                KeyboardKey::Menu => actions.push(self.keyboard_action(KeyboardInput::Menu)),
                KeyboardKey::Space => actions.push(self.keyboard_action(KeyboardInput::Space)),
                KeyboardKey::Tab => actions.push(self.keyboard_action(KeyboardInput::Tab)),
            }
        }
    }

    fn keyboard_action(&mut self, input: KeyboardInput) -> Action {
        let modifiers = std::mem::take(&mut self.keyboard.modifiers);
        if modifiers == KeyboardModifiers::default() {
            Action::Keyboard(input)
        } else {
            Action::KeyboardChord(input, modifiers)
        }
    }
}

// Digital input wins; a stick tilt selects one dominant axis to avoid diagonal double steps.
fn keyboard_directions(frame: &GamepadFrame, deadzone: f64) -> [bool; 4] {
    let mut directions =
        [Button::Up, Button::Down, Button::Left, Button::Right].map(|button| frame.pressed(button));
    if directions.iter().any(|pressed| *pressed) {
        return directions;
    }
    let x = axis_value(frame.x, deadzone);
    let y = axis_value(frame.y, deadzone);
    if x != 0.0 || y != 0.0 {
        let index = if x.abs() >= y.abs() {
            if x > 0.0 { 3 } else { 2 }
        } else if y > 0.0 {
            1
        } else {
            0
        };
        directions[index] = true;
    }
    directions
}

fn axis_value(value: f64, deadzone: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    let deadzone = deadzone.clamp(0.05, 0.5);
    let value = value.clamp(-1.0, 1.0);
    value.signum() * ((value.abs() - deadzone).max(0.0) / (1.0 - deadzone)).powi(2)
}

fn accumulate(remainder: &mut f64, amount: f64) -> i32 {
    *remainder += amount;
    let whole = remainder.trunc() as i32;
    *remainder -= f64::from(whole);
    whole
}

#[cfg(test)]
mod tests {
    use super::{Action, Button, ControlSettings, GamepadFrame, MouseController};
    use std::time::Duration;

    #[test]
    fn r3_closes_an_open_keyboard_even_when_the_controller_requires_neutral() {
        let tick = Duration::from_millis(10);
        let mut controller = MouseController::default();
        let mut settings = ControlSettings::default();
        let neutral = GamepadFrame::default();
        controller.set_system_keyboard_uses_gamepad(true);
        controller.keyboard.presentation = crate::keyboard::KeyboardPresentation::System;
        controller.update(Some(&neutral), tick, &mut settings);

        let mut r3 = neutral;
        r3.buttons[Button::RightStick as usize] = true;
        assert_eq!(
            controller.update(Some(&r3), tick, &mut settings),
            vec![Action::OnScreenKeyboard(true)]
        );
        controller.update(Some(&neutral), tick, &mut settings);
        controller.require_neutral();

        assert_eq!(
            controller.update(Some(&r3), tick, &mut settings),
            vec![Action::OnScreenKeyboard(false)]
        );
        assert!(!controller.keyboard.open);
    }
}
