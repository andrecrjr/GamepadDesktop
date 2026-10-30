use evdev::{
    AttributeSet, EventType, InputEvent, KeyCode, RelativeAxisCode as Relative,
    uinput::VirtualDevice,
};
use std::env;

#[path = "cosmic_keyboard.rs"]
mod cosmic_keyboard;
use cosmic_keyboard::CosmicKeyboard;
#[path = "linux_cursor_scale.rs"]
mod linux_cursor_scale;
use linux_cursor_scale::LinuxCursorScale;

use crate::{
    control::{Action, DpadDirection, KeyboardInput, MouseButton, SystemKey},
    cursor_scale::{CursorScale, CursorScaleRange},
    keyboard::{KeyboardModifiers, KeyboardPresentation, KeyboardSession},
    wayland_keyboard::WaylandKeyboardOverlay,
};

#[cfg(test)]
#[path = "linux_tests.rs"]
mod tests;

pub struct Platform {
    pointer: VirtualDevice,
    keyboard: VirtualDevice,
    keyboard_overlay: Option<WaylandKeyboardOverlay>,
    cosmic_keyboard: Option<CosmicKeyboard>,
    keyboard_presentation: KeyboardPresentation,
    keyboard_overlay_error: Option<String>,
    cursor_scale: LinuxCursorScale,
}

impl Platform {
    pub fn new() -> Result<Self, String> {
        let create = || -> std::io::Result<Self> {
            let mouse_keys: AttributeSet<KeyCode> = [KeyCode::BTN_LEFT, KeyCode::BTN_RIGHT]
                .into_iter()
                .collect();
            let relative: AttributeSet<Relative> =
                [Relative::REL_X, Relative::REL_Y, Relative::REL_WHEEL]
                    .into_iter()
                    .collect();
            let keys: AttributeSet<KeyCode> = (KeyCode::KEY_ESC.0..=KeyCode::KEY_MICMUTE.0)
                .map(KeyCode)
                .collect();
            let pointer = VirtualDevice::builder()?
                .name("Gamepad Desktop Mouse")
                .with_keys(&mouse_keys)?
                .with_relative_axes(&relative)?
                .build()?;
            let keyboard = VirtualDevice::builder()?
                .name("Gamepad Desktop Keyboard")
                .with_keys(&keys)?
                .build()?;
            let is_wayland = env::var_os("WAYLAND_DISPLAY").is_some()
                || env::var("XDG_SESSION_TYPE")
                    .is_ok_and(|session| session.eq_ignore_ascii_case("wayland"));
            let (keyboard_overlay, keyboard_presentation, keyboard_overlay_error) = if is_wayland {
                match WaylandKeyboardOverlay::new() {
                    Ok(overlay) => (Some(overlay), KeyboardPresentation::WaylandLayer, None),
                    Err(error) => (None, KeyboardPresentation::Unavailable, Some(error)),
                }
            } else {
                (None, KeyboardPresentation::Viewport, None)
            };
            let cosmic_keyboard = CosmicKeyboard::connect(
                is_wayland,
                &env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
            );
            let keyboard_presentation = if cosmic_keyboard.is_some() {
                KeyboardPresentation::System
            } else {
                keyboard_presentation
            };
            Ok(Self {
                pointer,
                keyboard,
                keyboard_overlay,
                cosmic_keyboard,
                keyboard_presentation,
                keyboard_overlay_error,
                cursor_scale: LinuxCursorScale::new(),
            })
        };
        create().map_err(|error| format!("Não foi possível criar mouse/teclado virtual em /dev/uinput: {error}. Verifique as permissões da sua sessão."))
    }

    pub fn system_keyboard_uses_gamepad(&self) -> bool {
        true
    }

    pub fn keyboard_presentation(&self) -> KeyboardPresentation {
        self.keyboard_presentation
    }

    pub fn keyboard_overlay_error(&self) -> Option<&str> {
        self.keyboard_overlay_error.as_deref()
    }

    pub(crate) fn update_native_dpad_target(&mut self) {}

    pub fn cursor_scale_range(&self) -> CursorScaleRange {
        self.cursor_scale.range()
    }

    pub fn set_cursor_scale(&mut self, scale: CursorScale) -> Result<(), String> {
        self.cursor_scale.set(scale).map(|_| ())
    }

    pub(crate) fn update_mouse_mode(&mut self, _: bool, _: bool) -> Result<(), String> {
        Ok(())
    }

    pub fn update_keyboard_overlay(&mut self, session: &mut KeyboardSession) -> Result<(), String> {
        if let Some(keyboard) = &mut self.cosmic_keyboard {
            match keyboard.update(session.open) {
                Ok(()) => return Ok(()),
                Err(error) => {
                    // Release native ownership before restoring the existing Linux path.
                    let _ = keyboard.update(false);
                    self.cosmic_keyboard = None;
                    self.keyboard_presentation = if self.keyboard_overlay.is_some() {
                        KeyboardPresentation::WaylandLayer
                    } else {
                        KeyboardPresentation::Unavailable
                    };
                    session.presentation = self.keyboard_presentation;
                    session.status = Some(format!(
                        "COSMIC indisponível; usando teclado próprio: {error}"
                    ));
                    if self.keyboard_overlay.is_none() {
                        return Err(error);
                    }
                }
            }
        }
        let Some(overlay) = &mut self.keyboard_overlay else {
            return Ok(());
        };
        if let Err(error) = overlay.update(session) {
            self.keyboard_overlay = None;
            self.keyboard_presentation = KeyboardPresentation::Unavailable;
            self.keyboard_overlay_error = Some(error.clone());
            return Err(error);
        }
        Ok(())
    }

    pub fn apply(&mut self, action: Action) -> Result<(), String> {
        let key_event = |key: KeyCode, value| InputEvent::new(EventType::KEY.0, key.0, value);
        let result = match action {
            Action::OnScreenKeyboard(_) => return Ok(()),
            Action::Move(x, y) => self.pointer.emit(&[
                InputEvent::new(EventType::RELATIVE.0, Relative::REL_X.0, x),
                InputEvent::new(EventType::RELATIVE.0, Relative::REL_Y.0, y),
            ]),
            Action::Scroll(amount) => self.pointer.emit(&[InputEvent::new(
                EventType::RELATIVE.0,
                Relative::REL_WHEEL.0,
                -amount,
            )]),
            Action::Mouse(button, pressed) => self.pointer.emit(&[key_event(
                match button {
                    MouseButton::Left => KeyCode::BTN_LEFT,
                    MouseButton::Right => KeyCode::BTN_RIGHT,
                },
                i32::from(pressed),
            )]),
            Action::Key(key) => {
                let code = match key {
                    SystemKey::Enter => KeyCode::KEY_ENTER,
                    SystemKey::Escape => KeyCode::KEY_ESC,
                    SystemKey::Up => KeyCode::KEY_UP,
                    SystemKey::Down => KeyCode::KEY_DOWN,
                    SystemKey::Left => KeyCode::KEY_LEFT,
                    SystemKey::Right => KeyCode::KEY_RIGHT,
                    SystemKey::Tab => KeyCode::KEY_TAB,
                    SystemKey::Meta => KeyCode::KEY_LEFTMETA,
                };
                self.emit_key(code)
            }
            Action::Dpad(direction) => {
                let code = match direction {
                    DpadDirection::Up => KeyCode::KEY_UP,
                    DpadDirection::Down => KeyCode::KEY_DOWN,
                    DpadDirection::Left => KeyCode::KEY_LEFT,
                    DpadDirection::Right => KeyCode::KEY_RIGHT,
                };
                self.emit_key(code)
            }
            Action::DpadReleased => return Ok(()),
            Action::Keyboard(input) => {
                self.emit_keyboard_input(input, KeyboardModifiers::default())
            }
            Action::KeyboardChord(input, modifiers) => self.emit_keyboard_input(input, modifiers),
            Action::NextWindow => {
                let press = self.keyboard.emit(&[
                    key_event(KeyCode::KEY_LEFTALT, 1),
                    key_event(KeyCode::KEY_TAB, 1),
                ]);
                let release = self.keyboard.emit(&[
                    key_event(KeyCode::KEY_TAB, 0),
                    key_event(KeyCode::KEY_LEFTALT, 0),
                ]);
                press.and(release)
            }
        };
        result.map_err(|error| format!("Erro enviando entrada ao desktop: {error}"))
    }

    fn emit_key(&mut self, key: KeyCode) -> std::io::Result<()> {
        self.keyboard.emit(&[
            InputEvent::new(EventType::KEY.0, key.0, 1),
            InputEvent::new(EventType::KEY.0, key.0, 0),
        ])
    }

    fn emit_keyboard_input(
        &mut self,
        input: KeyboardInput,
        modifiers: KeyboardModifiers,
    ) -> std::io::Result<()> {
        let (key, shifted) = match input {
            KeyboardInput::Character(character) => ascii_key(character)
                .ok_or_else(|| std::io::Error::other("caractere não suportado"))?,
            KeyboardInput::Function(number) => (
                match number {
                    1 => KeyCode::KEY_F1,
                    2 => KeyCode::KEY_F2,
                    3 => KeyCode::KEY_F3,
                    4 => KeyCode::KEY_F4,
                    5 => KeyCode::KEY_F5,
                    6 => KeyCode::KEY_F6,
                    7 => KeyCode::KEY_F7,
                    8 => KeyCode::KEY_F8,
                    9 => KeyCode::KEY_F9,
                    10 => KeyCode::KEY_F10,
                    11 => KeyCode::KEY_F11,
                    12 => KeyCode::KEY_F12,
                    _ => return Err(std::io::Error::other("tecla de função inválida")),
                },
                false,
            ),
            KeyboardInput::Backspace => (KeyCode::KEY_BACKSPACE, false),
            KeyboardInput::Enter => (KeyCode::KEY_ENTER, false),
            KeyboardInput::Escape => (KeyCode::KEY_ESC, false),
            KeyboardInput::Space => (KeyCode::KEY_SPACE, false),
            KeyboardInput::Tab => (KeyCode::KEY_TAB, false),
            KeyboardInput::Menu => (KeyCode::KEY_COMPOSE, false),
        };
        let press = |code: KeyCode| InputEvent::new(EventType::KEY.0, code.0, 1);
        let release = |code: KeyCode| InputEvent::new(EventType::KEY.0, code.0, 0);

        let modifiers: Vec<_> = [
            (modifiers.control, KeyCode::KEY_LEFTCTRL),
            (modifiers.alt, KeyCode::KEY_LEFTALT),
            (modifiers.meta, KeyCode::KEY_LEFTMETA),
            (shifted, KeyCode::KEY_LEFTSHIFT),
        ]
        .into_iter()
        .filter_map(|(enabled, key)| enabled.then_some(key))
        .collect();
        let events: Vec<_> = modifiers
            .iter()
            .copied()
            .map(press)
            .chain([press(key), release(key)])
            .chain(modifiers.iter().rev().copied().map(release))
            .collect();
        self.keyboard.emit(&events)
    }
}

fn ascii_key(character: char) -> Option<(KeyCode, bool)> {
    let (code, shifted) = match character {
        '`' => (KeyCode::KEY_GRAVE, false),
        '~' => (KeyCode::KEY_GRAVE, true),
        'a' | 'A' => (KeyCode::KEY_A, character.is_ascii_uppercase()),
        'b' | 'B' => (KeyCode::KEY_B, character.is_ascii_uppercase()),
        'c' | 'C' => (KeyCode::KEY_C, character.is_ascii_uppercase()),
        'd' | 'D' => (KeyCode::KEY_D, character.is_ascii_uppercase()),
        'e' | 'E' => (KeyCode::KEY_E, character.is_ascii_uppercase()),
        'f' | 'F' => (KeyCode::KEY_F, character.is_ascii_uppercase()),
        'g' | 'G' => (KeyCode::KEY_G, character.is_ascii_uppercase()),
        'h' | 'H' => (KeyCode::KEY_H, character.is_ascii_uppercase()),
        'i' | 'I' => (KeyCode::KEY_I, character.is_ascii_uppercase()),
        'j' | 'J' => (KeyCode::KEY_J, character.is_ascii_uppercase()),
        'k' | 'K' => (KeyCode::KEY_K, character.is_ascii_uppercase()),
        'l' | 'L' => (KeyCode::KEY_L, character.is_ascii_uppercase()),
        'm' | 'M' => (KeyCode::KEY_M, character.is_ascii_uppercase()),
        'n' | 'N' => (KeyCode::KEY_N, character.is_ascii_uppercase()),
        'o' | 'O' => (KeyCode::KEY_O, character.is_ascii_uppercase()),
        'p' | 'P' => (KeyCode::KEY_P, character.is_ascii_uppercase()),
        'q' | 'Q' => (KeyCode::KEY_Q, character.is_ascii_uppercase()),
        'r' | 'R' => (KeyCode::KEY_R, character.is_ascii_uppercase()),
        's' | 'S' => (KeyCode::KEY_S, character.is_ascii_uppercase()),
        't' | 'T' => (KeyCode::KEY_T, character.is_ascii_uppercase()),
        'u' | 'U' => (KeyCode::KEY_U, character.is_ascii_uppercase()),
        'v' | 'V' => (KeyCode::KEY_V, character.is_ascii_uppercase()),
        'w' | 'W' => (KeyCode::KEY_W, character.is_ascii_uppercase()),
        'x' | 'X' => (KeyCode::KEY_X, character.is_ascii_uppercase()),
        'y' | 'Y' => (KeyCode::KEY_Y, character.is_ascii_uppercase()),
        'z' | 'Z' => (KeyCode::KEY_Z, character.is_ascii_uppercase()),
        '1' => (KeyCode::KEY_1, false),
        '2' => (KeyCode::KEY_2, false),
        '3' => (KeyCode::KEY_3, false),
        '4' => (KeyCode::KEY_4, false),
        '5' => (KeyCode::KEY_5, false),
        '6' => (KeyCode::KEY_6, false),
        '7' => (KeyCode::KEY_7, false),
        '8' => (KeyCode::KEY_8, false),
        '9' => (KeyCode::KEY_9, false),
        '0' => (KeyCode::KEY_0, false),
        '!' => (KeyCode::KEY_1, true),
        '@' => (KeyCode::KEY_2, true),
        '#' => (KeyCode::KEY_3, true),
        '$' => (KeyCode::KEY_4, true),
        '%' => (KeyCode::KEY_5, true),
        '^' => (KeyCode::KEY_6, true),
        '&' => (KeyCode::KEY_7, true),
        '*' => (KeyCode::KEY_8, true),
        '(' => (KeyCode::KEY_9, true),
        ')' => (KeyCode::KEY_0, true),
        '-' => (KeyCode::KEY_MINUS, false),
        '_' => (KeyCode::KEY_MINUS, true),
        '=' => (KeyCode::KEY_EQUAL, false),
        '+' => (KeyCode::KEY_EQUAL, true),
        '\\' => (KeyCode::KEY_BACKSLASH, false),
        '|' => (KeyCode::KEY_BACKSLASH, true),
        '[' => (KeyCode::KEY_LEFTBRACE, false),
        '{' => (KeyCode::KEY_LEFTBRACE, true),
        ']' => (KeyCode::KEY_RIGHTBRACE, false),
        '}' => (KeyCode::KEY_RIGHTBRACE, true),
        ';' => (KeyCode::KEY_SEMICOLON, false),
        ':' => (KeyCode::KEY_SEMICOLON, true),
        '\'' => (KeyCode::KEY_APOSTROPHE, false),
        '"' => (KeyCode::KEY_APOSTROPHE, true),
        ',' => (KeyCode::KEY_COMMA, false),
        '<' => (KeyCode::KEY_COMMA, true),
        '.' => (KeyCode::KEY_DOT, false),
        '>' => (KeyCode::KEY_DOT, true),
        '/' => (KeyCode::KEY_SLASH, false),
        '?' => (KeyCode::KEY_SLASH, true),
        _ => return None,
    };
    Some((code, shifted))
}
