#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyboardPresentation {
    #[default]
    Viewport,
    WaylandLayer,
    System,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardKey {
    Character(char),
    Function(u8),
    Backspace,
    Enter,
    Escape,
    Shift,
    CapsLock,
    Control,
    Alt,
    Meta,
    Menu,
    Space,
    Tab,
}

pub use crate::i18n::{Locale, Text};
use KeyboardKey::{
    Alt, Backspace, CapsLock, Character as C, Control, Enter, Function as F, Menu, Meta, Shift,
    Space, Tab,
};

const FUNCTION_ROW: [KeyboardKey; 12] = [
    F(1),
    F(2),
    F(3),
    F(4),
    F(5),
    F(6),
    F(7),
    F(8),
    F(9),
    F(10),
    F(11),
    F(12),
];

/// ANSI QWERTY typing block with a function-key row. Every row spans 15 key units.
pub const KEY_ROWS: &[&[KeyboardKey]] = &[
    &FUNCTION_ROW,
    &[
        C('`'),
        C('1'),
        C('2'),
        C('3'),
        C('4'),
        C('5'),
        C('6'),
        C('7'),
        C('8'),
        C('9'),
        C('0'),
        C('-'),
        C('='),
        Backspace,
    ],
    &[
        Tab,
        C('q'),
        C('w'),
        C('e'),
        C('r'),
        C('t'),
        C('y'),
        C('u'),
        C('i'),
        C('o'),
        C('p'),
        C('['),
        C(']'),
        C('\\'),
    ],
    &[
        CapsLock,
        C('a'),
        C('s'),
        C('d'),
        C('f'),
        C('g'),
        C('h'),
        C('j'),
        C('k'),
        C('l'),
        C(';'),
        C('\''),
        Enter,
    ],
    &[
        Shift,
        C('z'),
        C('x'),
        C('c'),
        C('v'),
        C('b'),
        C('n'),
        C('m'),
        C(','),
        C('.'),
        C('/'),
        Shift,
    ],
    &[Control, Meta, Alt, Space, Alt, Meta, Menu, Control],
];

/// Width in quarter-key units; used by rendering and spatial navigation.
pub fn key_width_quarters(row: usize, column: usize) -> u16 {
    match (row, column) {
        (0, _) => 5,
        (1, 13) => 8,
        (2, 0 | 13) => 6,
        (3, 0) => 7,
        (3, 12) | (4, 0) => 9,
        (4, 11) => 11,
        (5, 3) => 25,
        (5, _) => 5,
        _ => 4,
    }
}

pub fn key_center(row: usize, column: usize) -> u16 {
    (0..column)
        .map(|i| key_width_quarters(row, i) * 2)
        .sum::<u16>()
        + key_width_quarters(row, column)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KeyboardPosition {
    Top,
    #[default]
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct KeyboardSettings {
    pub width: u16,
    pub opacity: u8,
    pub position: KeyboardPosition,
    pub repeat_enabled: bool,
    pub repeat_delay_ms: u16,
    pub repeat_interval_ms: u16,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        Self {
            width: 640,
            opacity: 235,
            position: KeyboardPosition::Bottom,
            repeat_enabled: true,
            repeat_delay_ms: 300,
            repeat_interval_ms: 100,
        }
    }
}

impl KeyboardSettings {
    pub fn normalized(mut self) -> Self {
        self.width = self.width.clamp(520, 1000);
        self.opacity = self.opacity.max(160);
        self.repeat_delay_ms = self.repeat_delay_ms.clamp(200, 1200);
        self.repeat_interval_ms = self.repeat_interval_ms.clamp(100, 1000);
        self
    }

    pub fn surface_size(self) -> (u32, u32) {
        let width = u32::from(self.normalized().width);
        (width, width * 258 / 720)
    }

    pub fn load() -> Self {
        settings_path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .unwrap_or_default()
            .normalized()
    }

    pub fn save(self) -> Result<(), String> {
        let path = settings_path().ok_or("Pasta de configuração indisponível")?;
        std::fs::create_dir_all(path.parent().expect("settings directory"))
            .map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(&self.normalized()).map_err(|e| e.to_string())?;
        let pending = path.with_extension("pending");
        std::fs::write(&pending, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(pending, path).map_err(|e| e.to_string())
    }

    pub fn adjust(&mut self, index: usize, direction: i16) {
        fn step(value: u16, delta: i16) -> u16 {
            value.saturating_add_signed(delta)
        }
        match index {
            0 => self.width = step(self.width, direction * 20),
            1 => {
                self.position = match self.position {
                    KeyboardPosition::Top => KeyboardPosition::Bottom,
                    KeyboardPosition::Bottom => KeyboardPosition::Top,
                }
            }
            2 => self.opacity = (i16::from(self.opacity) + direction * 10).clamp(160, 255) as u8,
            3 => self.repeat_enabled = !self.repeat_enabled,
            4 => self.repeat_delay_ms = step(self.repeat_delay_ms, direction * 50),
            5 => self.repeat_interval_ms = step(self.repeat_interval_ms, direction * 50),
            _ => return,
        }
        *self = self.normalized();
    }

    pub fn description(self, index: usize) -> String {
        self.description_in(Locale::PortugueseBrazil, index)
    }

    pub fn description_in(self, locale: Locale, index: usize) -> String {
        match index {
            0 => format!("{}: {} px", locale.text(Text::KeyboardWidth), self.width),
            1 => format!(
                "{}: {}",
                locale.text(Text::KeyboardPosition),
                if self.position == KeyboardPosition::Top {
                    locale.text(Text::Top)
                } else {
                    locale.text(Text::Bottom)
                }
            ),
            2 => format!(
                "{}: {}%",
                locale.text(Text::KeyboardOpacity),
                u16::from(self.opacity) * 100 / 255
            ),
            3 => format!(
                "{}: {}",
                locale.text(Text::KeyboardRepeat),
                locale.text(if self.repeat_enabled {
                    Text::Yes
                } else {
                    Text::No
                })
            ),
            4 => format!(
                "{}: {} ms",
                locale.text(Text::KeyboardRepeatDelay),
                self.repeat_delay_ms
            ),
            5 => format!(
                "{}: {} ms",
                locale.text(Text::KeyboardRepeatInterval),
                self.repeat_interval_ms
            ),
            _ => String::new(),
        }
    }
}

fn settings_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(std::path::PathBuf::from);
    #[cfg(not(target_os = "windows"))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")));
    base.map(|p| p.join("gamepad-navigator/keyboard.json"))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyboardModifiers {
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardSession {
    pub locale: Locale,
    pub focused_input: Option<crate::focused_input::FocusedInput>,
    pub open: bool,
    pub status: Option<String>,
    pub presentation: KeyboardPresentation,
    pub selected_row: usize,
    pub selected_column: usize,
    pub shift: bool,
    pub caps_lock: bool,
    pub modifiers: KeyboardModifiers,
    pub settings: KeyboardSettings,
    pub customizing: bool,
    pub selected_setting: usize,
    navigation_x: Option<u16>,
}

impl Default for KeyboardSession {
    fn default() -> Self {
        Self {
            locale: Locale::default(),
            focused_input: None,
            open: false,
            status: None,
            presentation: KeyboardPresentation::default(),
            selected_row: 3,
            selected_column: 1,
            shift: false,
            caps_lock: false,
            modifiers: KeyboardModifiers::default(),
            settings: KeyboardSettings::default(),
            customizing: false,
            selected_setting: 0,
            navigation_x: None,
        }
    }
}

impl KeyboardSession {
    pub(crate) fn with_locale(locale: Locale, settings: KeyboardSettings) -> Self {
        Self {
            locale,
            settings,
            ..Self::default()
        }
    }

    pub fn shifted(&self, key: KeyboardKey) -> bool {
        self.shift ^ (self.caps_lock && matches!(key, KeyboardKey::Character('a'..='z')))
    }

    pub fn selected_index(&self) -> usize {
        KEY_ROWS[..self.selected_row]
            .iter()
            .map(|row| row.len())
            .sum::<usize>()
            + self.selected_column
    }

    pub fn selected_key(&self) -> KeyboardKey {
        KEY_ROWS[self.selected_row][self.selected_column]
    }

    pub fn move_selection(&mut self, row_delta: isize, column_delta: isize) {
        if column_delta != 0 {
            self.selected_column = (self.selected_column as isize + column_delta)
                .clamp(0, KEY_ROWS[self.selected_row].len() as isize - 1)
                as usize;
            self.navigation_x = None;
            return;
        }
        let x = *self
            .navigation_x
            .get_or_insert(key_center(self.selected_row, self.selected_column));
        self.selected_row =
            (self.selected_row as isize + row_delta).clamp(0, KEY_ROWS.len() as isize - 1) as usize;
        self.selected_column = (0..KEY_ROWS[self.selected_row].len())
            .min_by_key(|column| {
                let center = key_center(self.selected_row, *column);
                let distance = center.abs_diff(x);
                (
                    distance.saturating_sub(key_width_quarters(self.selected_row, *column)),
                    distance,
                )
            })
            .expect("nonempty QWERTY row");
    }
}

impl KeyboardKey {
    pub fn label(self, shift: bool) -> String {
        match self.character(shift) {
            Some(character) => character.to_string(),
            None => match self {
                Self::Function(number) => format!("F{number}"),
                Self::Backspace => "Backspace".to_owned(),
                Self::Enter => "Enter".to_owned(),
                Self::Escape => "Esc".to_owned(),
                Self::Shift => "Shift".to_owned(),
                Self::CapsLock => "Caps".to_owned(),
                Self::Control => "Ctrl".to_owned(),
                Self::Alt => "Alt".to_owned(),
                Self::Meta => "Super".to_owned(),
                Self::Menu => "Menu".to_owned(),
                Self::Space => "Espaço".to_owned(),
                Self::Tab => "Tab".to_owned(),
                Self::Character(_) => unreachable!(),
            },
        }
    }

    pub fn label_in(self, shift: bool, locale: Locale) -> String {
        if self == Self::Space {
            locale.text(Text::Space).to_owned()
        } else {
            self.label(shift)
        }
    }

    pub fn character(self, shift: bool) -> Option<char> {
        let Self::Character(character) = self else {
            return None;
        };
        Some(if shift {
            match character {
                'a'..='z' => character.to_ascii_uppercase(),
                '`' => '~',
                '1' => '!',
                '2' => '@',
                '3' => '#',
                '4' => '$',
                '5' => '%',
                '6' => '^',
                '7' => '&',
                '8' => '*',
                '9' => '(',
                '0' => ')',
                '-' => '_',
                '=' => '+',
                '[' => '{',
                ']' => '}',
                ';' => ':',
                '\'' => '"',
                '\\' => '|',
                ',' => '<',
                '.' => '>',
                '/' => '?',
                _ => character,
            }
        } else {
            character
        })
    }
}
