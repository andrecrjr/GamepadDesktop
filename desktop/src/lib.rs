pub mod control;
pub mod focused_input;
pub mod gamepads;
pub mod i18n;
pub mod keyboard;
pub(crate) mod notifications;
pub mod onboarding;
pub mod placement;
pub mod preferences;
pub mod runtime;
pub(crate) mod startup;
pub mod tray;

#[cfg(any(target_os = "windows", test))]
mod windows_navigation;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
pub mod platform;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
pub mod platform;

#[cfg(target_os = "linux")]
mod wayland_keyboard;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("Gamepad Desktop currently supports Linux and Windows only");
