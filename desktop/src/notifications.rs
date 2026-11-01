use std::{
    sync::mpsc::{self, Sender},
    thread,
};

use crate::{
    control::ControlMode,
    i18n::{Locale, Text},
};

const APP_NAME: &str = "Gamepad Desktop";

#[cfg(target_os = "windows")]
const APP_USER_MODEL_ID: &str = "GamepadNavigator.Desktop";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Notification {
    ControlState {
        enabled: bool,
        locale: Locale,
    },
    InputModeChanged {
        mode: ControlMode,
        locale: Locale,
    },
    ControllerConnected {
        locale: Locale,
        name: String,
    },
    ControllerDisconnected(Locale),
    NoUsableProfile(Locale),
    KeyboardUnavailable {
        locale: Locale,
        detail: Option<String>,
    },
    RuntimeFailure {
        locale: Locale,
        detail: String,
    },
}

impl Notification {
    pub(crate) fn mode_change(
        previous: ControlMode,
        current: ControlMode,
        locale: Locale,
    ) -> Option<Self> {
        (previous != current).then_some(Self::InputModeChanged {
            mode: current,
            locale,
        })
    }

    fn content(&self) -> (String, String) {
        let locale = match self {
            Self::ControlState { locale, .. }
            | Self::InputModeChanged { locale, .. }
            | Self::ControllerConnected { locale, .. }
            | Self::KeyboardUnavailable { locale, .. }
            | Self::RuntimeFailure { locale, .. } => *locale,
            Self::ControllerDisconnected(locale) | Self::NoUsableProfile(locale) => *locale,
        };
        let body = match self {
            Self::ControlState { enabled, .. } => locale.text(if *enabled {
                Text::ControlResumedNotice
            } else {
                Text::ControlPausedNotice
            }),
            Self::InputModeChanged { mode, .. } => locale.text(match mode {
                ControlMode::Desktop => Text::DesktopModeNotice,
                ControlMode::Game => Text::GameModeNotice,
            }),
            Self::ControllerConnected { name, .. } => {
                return (
                    locale.text(Text::NotificationTitle).to_owned(),
                    format!("{}{name}", locale.text(Text::ControllerConnectedPrefix)),
                );
            }
            Self::ControllerDisconnected(_) => locale.text(Text::ControllerDisconnectedNotice),
            Self::NoUsableProfile(_) => locale.text(Text::NoUsableProfileNotice),
            Self::KeyboardUnavailable { detail, .. } => match detail {
                Some(detail) if !detail.trim().is_empty() => {
                    return (
                        locale.text(Text::NotificationTitle).to_owned(),
                        format!(
                            "{} {}",
                            locale.text(Text::KeyboardUnavailableNotice),
                            locale.translate(detail)
                        ),
                    );
                }
                _ => locale.text(Text::KeyboardUnavailableNotice),
            },
            Self::RuntimeFailure { detail, .. } => {
                return (
                    locale.text(Text::NotificationTitle).to_owned(),
                    format!(
                        "{}{}",
                        locale.text(Text::RuntimeStoppedPrefix),
                        locale.translate(detail)
                    ),
                );
            }
        };
        (
            locale.text(Text::NotificationTitle).to_owned(),
            body.to_owned(),
        )
    }
}

#[derive(Clone)]
pub(crate) struct NotificationService {
    sender: Sender<Notification>,
}

impl NotificationService {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<Notification>();
        thread::spawn(move || {
            #[cfg(target_os = "windows")]
            if let Err(error) = windows_shortcut::ensure_start_menu_shortcut() {
                eprintln!("Could not register Gamepad Desktop notifications: {error}");
            }

            for event in receiver {
                let (title, body) = event.content();
                let mut notification = notify_rust::Notification::new();
                notification.summary(&title).body(&body).appname(APP_NAME);
                #[cfg(target_os = "windows")]
                notification.app_id(APP_USER_MODEL_ID);
                if let Err(error) = notification.show() {
                    eprintln!("Could not show Gamepad Desktop notification: {error}");
                }
            }
        });
        Self { sender }
    }

    pub(crate) fn send(&self, notification: Notification) {
        let _ = self.sender.send(notification);
    }
}

#[cfg(target_os = "windows")]
mod windows_shortcut {
    use std::{
        mem::{ManuallyDrop, size_of},
        os::windows::ffi::OsStrExt,
        path::PathBuf,
    };

    use windows::{
        Win32::{
            Foundation::PROPERTYKEY,
            System::{
                Com::{
                    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance,
                    CoInitializeEx, CoTaskMemAlloc, CoUninitialize, IPersistFile,
                    StructuredStorage::{
                        PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
                        PropVariantClear,
                    },
                },
                Variant::VT_LPWSTR,
            },
            UI::Shell::{IShellLinkW, PropertiesSystem::IPropertyStore, ShellLink},
        },
        core::{GUID, Interface, PCWSTR, PWSTR},
    };

    const APP_USER_MODEL_ID_KEY: PROPERTYKEY = PROPERTYKEY {
        fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
        pid: 5,
    };

    pub(super) fn ensure_start_menu_shortcut() -> Result<(), String> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .map_err(|error| error.to_string())?;
        let result = create_shortcut();
        unsafe { CoUninitialize() };
        result
    }

    fn create_shortcut() -> Result<(), String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let working_directory = executable
            .parent()
            .ok_or_else(|| "The executable has no parent directory".to_owned())?;
        let programs = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| "APPDATA is not available".to_owned())?
            .join("Microsoft/Windows/Start Menu/Programs");
        std::fs::create_dir_all(&programs).map_err(|error| error.to_string())?;
        let shortcut_path = programs.join("Gamepad Desktop.lnk");
        let executable_wide = wide(executable.as_os_str());
        let working_directory_wide = wide(working_directory.as_os_str());
        let description_wide = wide(std::ffi::OsStr::new("Gamepad Desktop"));
        let shortcut_path_wide = wide(shortcut_path.as_os_str());

        let link: IShellLinkW = unsafe {
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|error| error.to_string())?
        };
        unsafe {
            link.SetPath(PCWSTR(executable_wide.as_ptr()))
                .map_err(|error| error.to_string())?;
            link.SetWorkingDirectory(PCWSTR(working_directory_wide.as_ptr()))
                .map_err(|error| error.to_string())?;
            link.SetDescription(PCWSTR(description_wide.as_ptr()))
                .map_err(|error| error.to_string())?;
            link.SetIconLocation(PCWSTR(executable_wide.as_ptr()), 0)
                .map_err(|error| error.to_string())?;
        }

        let property_store: IPropertyStore = link.cast().map_err(|error| error.to_string())?;
        let mut app_id = app_user_model_id_variant()?;
        let set_result = unsafe { property_store.SetValue(&APP_USER_MODEL_ID_KEY, &app_id) }
            .map_err(|error| error.to_string());
        let clear_result =
            unsafe { PropVariantClear(&mut app_id) }.map_err(|error| error.to_string());
        set_result?;
        clear_result?;
        unsafe { property_store.Commit() }.map_err(|error| error.to_string())?;

        let persistence: IPersistFile = link.cast().map_err(|error| error.to_string())?;
        unsafe { persistence.Save(PCWSTR(shortcut_path_wide.as_ptr()), true) }
            .map_err(|error| error.to_string())
    }

    fn app_user_model_id_variant() -> Result<PROPVARIANT, String> {
        let encoded = super::APP_USER_MODEL_ID
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let pointer = unsafe { CoTaskMemAlloc(encoded.len() * size_of::<u16>()) }.cast::<u16>();
        if pointer.is_null() {
            return Err("Could not allocate the notification app identity".to_owned());
        }
        unsafe { std::ptr::copy_nonoverlapping(encoded.as_ptr(), pointer, encoded.len()) };

        Ok(PROPVARIANT {
            Anonymous: PROPVARIANT_0 {
                Anonymous: ManuallyDrop::new(PROPVARIANT_0_0 {
                    vt: VT_LPWSTR,
                    wReserved1: 0,
                    wReserved2: 0,
                    wReserved3: 0,
                    Anonymous: PROPVARIANT_0_0_0 {
                        pwszVal: PWSTR(pointer),
                    },
                }),
            },
        })
    }

    fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_changes_notify_both_directions_without_repeating_unchanged_modes() {
        for previous in [ControlMode::Desktop, ControlMode::Game] {
            for current in [ControlMode::Desktop, ControlMode::Game] {
                let notice = Notification::mode_change(previous, current, Locale::PortugueseBrazil);
                if previous == current {
                    assert_eq!(notice, None);
                } else {
                    assert_eq!(
                        notice,
                        Some(Notification::InputModeChanged {
                            mode: current,
                            locale: Locale::PortugueseBrazil,
                        })
                    );
                    assert_eq!(
                        notice.unwrap().content().1,
                        Locale::PortugueseBrazil.text(match current {
                            ControlMode::Desktop => Text::DesktopModeNotice,
                            ControlMode::Game => Text::GameModeNotice,
                        })
                    );
                }
            }
        }
    }

    #[test]
    fn mode_notices_include_the_keyboard_command_for_each_mode() {
        for locale in Locale::ALL {
            let desktop = Notification::InputModeChanged {
                mode: ControlMode::Desktop,
                locale,
            }
            .content()
            .1;
            let game = Notification::InputModeChanged {
                mode: ControlMode::Game,
                locale,
            }
            .content()
            .1;

            assert!(desktop.contains("R3"), "desktop notice: {desktop}");
            assert!(game.contains("Select/Back + R3"), "game notice: {game}");
        }
    }

    #[test]
    fn pause_and_resume_notices_are_localized_and_explain_the_rearm() {
        let paused = Notification::ControlState {
            enabled: false,
            locale: Locale::English,
        }
        .content()
        .1;
        let resumed = Notification::ControlState {
            enabled: true,
            locale: Locale::PortugueseBrazil,
        }
        .content()
        .1;

        assert!(paused.contains("START + SELECT"));
        assert!(resumed.contains("centralize"));
    }

    #[test]
    fn runtime_error_notice_includes_the_actionable_detail() {
        let (_, body) = Notification::RuntimeFailure {
            locale: Locale::English,
            detail: "SDL3: device unavailable".to_owned(),
        }
        .content();

        assert!(body.contains("device unavailable"));
    }
}
