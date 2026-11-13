use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use sdl3::{
    EventPump, GamepadSubsystem, JoystickSubsystem, Sdl,
    gamepad::{Axis, Button as PadButton, Gamepad, GamepadType},
    joystick::JoystickId,
};

use crate::control::{Button, GamepadFrame};

#[cfg(test)]
#[path = "gamepads_tests.rs"]
mod tests;

#[derive(Clone, Debug)]
pub struct GamepadInfo {
    pub id: u32,
    pub name: String,
    pub guid: String,
    pub mapped: bool,
}

pub fn select_device(devices: &[GamepadInfo], selected: Option<u32>) -> Option<u32> {
    devices
        .iter()
        .find(|device| device.mapped && selected.is_none_or(|id| device.id == id))
        .map(|device| device.id)
}

pub fn validate_mapping(input: &str) -> Result<&str, String> {
    let line = input.trim();
    let mut parts = line.splitn(3, ',');
    let guid = parts.next().unwrap_or_default();
    let valid_guid =
        guid == "xinput" || guid.len() == 32 && guid.bytes().all(|byte| byte.is_ascii_hexdigit());
    let name = parts.next().unwrap_or_default();
    let bindings = parts.next().unwrap_or_default();
    if line
        .chars()
        .any(|character| matches!(character, '\n' | '\r' | '\0'))
        || !valid_guid
        || name.trim().is_empty()
        || !bindings.contains(':')
    {
        return Err("Cole uma única linha de perfil SDL: GUID,nome,botões/eixos.".to_owned());
    }
    Ok(line)
}

pub fn save_mapping(path: &Path, input: &str) -> Result<(), String> {
    let line = validate_mapping(input)?;
    let guid = line.split(',').next().expect("validated GUID");
    let existing = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.to_string()),
    };
    let mut lines: Vec<&str> = existing
        .lines()
        .filter(|old| old.split(',').next() != Some(guid))
        .collect();
    lines.push(line);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("pending");
    fs::write(&temporary, format!("{}\n", lines.join("\n"))).map_err(|error| error.to_string())?;
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

pub struct Gamepads {
    active: Option<Gamepad>,
    events: EventPump,
    gamepads: GamepadSubsystem,
    joysticks: JoystickSubsystem,
    _context: Sdl,
    next_scan: Instant,
    pub devices: Vec<GamepadInfo>,
    profile_path: Option<PathBuf>,
    pub warning: Option<String>,
}

impl Gamepads {
    pub fn new() -> Result<Self, String> {
        sdl3::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
        let context = sdl3::init().map_err(|error| error.to_string())?;
        let gamepads = context.gamepad().map_err(|error| error.to_string())?;
        let joysticks = context.joystick().map_err(|error| error.to_string())?;
        let events = context.event_pump().map_err(|error| error.to_string())?;
        let profile_path = profile_path();
        let mut warning = None;
        if let Some(path) = &profile_path {
            match fs::read_to_string(path) {
                Ok(contents) => {
                    for line in contents
                        .lines()
                        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
                    {
                        if let Err(error) = gamepads.add_mapping(line) {
                            warning = Some(format!("Erro ao carregar perfil SDL: {error}"));
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => warning = Some(format!("Erro ao ler os perfis: {error}")),
            }
        }
        Ok(Self {
            active: None,
            events,
            gamepads,
            joysticks,
            _context: context,
            next_scan: Instant::now(),
            devices: Vec::new(),
            profile_path,
            warning,
        })
    }

    pub fn name(&self) -> Option<String> {
        self.active.as_ref().and_then(Gamepad::name)
    }

    pub fn active_type(&self) -> Option<GamepadType> {
        self.active.as_ref().map(Gamepad::r#type)
    }

    pub fn import_mapping(&mut self, input: &str) -> Result<(), String> {
        let line = validate_mapping(input)?;
        self.gamepads
            .add_mapping(line)
            .map_err(|error| error.to_string())?;
        self.active = None;
        self.next_scan = Instant::now();
        let path = self.profile_path.as_ref().ok_or(
            "Perfil aplicado nesta sessão; pasta de configuração indisponível para salvar.",
        )?;
        save_mapping(path, line)
            .map_err(|error| format!("Perfil aplicado nesta sessão, mas não foi salvo: {error}"))
    }

    pub fn poll(&mut self, selected: Option<u32>) -> Result<Option<GamepadFrame>, String> {
        for _ in self.events.poll_iter() {}
        self.gamepads.update();
        if Instant::now() >= self.next_scan {
            self.next_scan = Instant::now() + Duration::from_millis(500);
            self.devices = self
                .joysticks
                .joysticks()
                .map_err(|error| error.to_string())?
                .into_iter()
                .filter_map(|id| {
                    let joystick = self.joysticks.open(id).ok()?;
                    Some(GamepadInfo {
                        id: id.raw(),
                        name: joystick.name(),
                        guid: joystick.guid().to_string(),
                        mapped: self.gamepads.is_gamepad(id),
                    })
                })
                .collect();
        }
        let target = select_device(&self.devices, selected);
        if self.active.as_ref().is_some_and(|gamepad| {
            !gamepad.connected() || gamepad.id().ok().map(|id| id.raw()) != target
        }) {
            self.active = None;
            self.next_scan = Instant::now();
            return Ok(None);
        }
        let Some(id) = target else {
            return Ok(None);
        };
        if self.active.is_none() {
            match self.gamepads.open(JoystickId::new(id)) {
                Ok(gamepad) => self.active = Some(gamepad),
                Err(_) => {
                    self.next_scan = Instant::now();
                    return Ok(None);
                }
            }
        }
        let gamepad = self.active.as_ref().expect("connected gamepad");
        let mut frame = GamepadFrame {
            x: normalize(gamepad.axis(Axis::LeftX)),
            y: normalize(gamepad.axis(Axis::LeftY)),
            scroll: normalize(gamepad.axis(Axis::RightY)),
            ..Default::default()
        };
        for (button, native) in [
            (Button::South, PadButton::South),
            (Button::East, PadButton::East),
            (Button::West, PadButton::West),
            (Button::North, PadButton::North),
            (Button::Up, PadButton::DPadUp),
            (Button::Down, PadButton::DPadDown),
            (Button::Left, PadButton::DPadLeft),
            (Button::Right, PadButton::DPadRight),
            (Button::LeftShoulder, PadButton::LeftShoulder),
            (Button::RightShoulder, PadButton::RightShoulder),
            (Button::RightStick, PadButton::RightStick),
            (Button::LeftStick, PadButton::LeftStick),
            (Button::Start, PadButton::Start),
            (Button::Select, PadButton::Back),
        ] {
            frame.buttons[button as usize] = gamepad.button(native);
        }
        Ok(Some(frame))
    }
}

fn normalize(value: i16) -> f64 {
    f64::from(value) / if value < 0 { 32768.0 } else { 32767.0 }
}

fn profile_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "linux")]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.map(|base| base.join("gamepad-navigator/gamecontrollerdb.txt"))
}
