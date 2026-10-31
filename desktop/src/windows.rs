use std::time::{Duration, Instant};

use enigo::{Axis as ScrollAxis, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

use crate::control::{Action, DpadDirection, KeyboardInput, MouseButton, SystemKey};
use crate::keyboard::{KeyboardModifiers, KeyboardPresentation, KeyboardSession};
use crate::window_switcher::{WindowSwitcherState, WindowSwitcherStep};
use crate::windows_gamepad_hook::GamepadNavigationHook;
use crate::windows_navigation::{
    NativeDpadRouting, NativeDpadTargetCache, PausedCursor, filter_desktop_action,
    is_native_windows_target, is_touch_keyboard_process, resolve_native_target,
    resolve_native_windows,
};
use crate::windows_touch_keyboard::WindowsTouchKeyboard;
#[path = "windows_cursor_scale.rs"]
mod windows_cursor_scale;
use crate::cursor_scale::{CursorScale, CursorScaleRange};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HWND, LPARAM, POINT},
        System::Threading::{
            IsImmersiveProcess, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
        UI::WindowsAndMessaging::{
            CreateCursor, EnumChildWindows, GA_ROOT, GUITHREADINFO, GetAncestor, GetClassNameW,
            GetForegroundWindow, GetGUIThreadInfo, GetPhysicalCursorPos, GetWindowThreadProcessId,
            IsWindowVisible, OCR_APPSTARTING, OCR_CROSS, OCR_HAND, OCR_HELP, OCR_IBEAM, OCR_NO,
            OCR_NORMAL, OCR_SIZEALL, OCR_SIZENESW, OCR_SIZENS, OCR_SIZENWSE, OCR_SIZEWE, OCR_UP,
            OCR_WAIT, SPI_SETCURSORS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetPhysicalCursorPos,
            SetSystemCursor, SystemParametersInfoW, WindowFromPhysicalPoint,
        },
    },
    core::{BOOL, PWSTR},
};
use windows_cursor_scale::WindowsCursorScale;

const NAVIGATION_TARGET_CHECK_INTERVAL: Duration = Duration::from_millis(50);

pub struct Platform {
    input: Enigo,
    navigation_hook: GamepadNavigationHook,
    touch_keyboard: WindowsTouchKeyboard,
    desktop_controls_active: bool,
    controller_connected: bool,
    native_dpad: NativeDpadRouting,
    target_cache: [NativeDpadTargetCache; 4],
    paused_cursor: PausedCursor,
    cursor_scale: WindowsCursorScale,
    last_target_check: Option<Instant>,
    window_switcher: WindowSwitcherState,
}

impl Platform {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            input: Enigo::new(&Settings::default())
                .map_err(|error| format!("Falha ao conectar entrada do Windows: {error}"))?,
            navigation_hook: GamepadNavigationHook::new().map_err(|error| {
                format!("Falha filtrando navegação do gamepad no Windows: {error}")
            })?,
            touch_keyboard: WindowsTouchKeyboard::default(),
            desktop_controls_active: false,
            controller_connected: false,
            native_dpad: NativeDpadRouting::default(),
            target_cache: Default::default(),
            paused_cursor: PausedCursor::default(),
            cursor_scale: WindowsCursorScale::new(),
            last_target_check: None,
            window_switcher: WindowSwitcherState::default(),
        })
    }

    pub(crate) fn update_native_dpad_target(&mut self) {
        if !self.navigation_hook.is_active() {
            self.refresh_native_dpad_target(false);
        }
    }

    pub fn cursor_scale_range(&self) -> CursorScaleRange {
        self.cursor_scale.range()
    }

    pub fn set_cursor_scale(&mut self, scale: CursorScale) -> Result<(), String> {
        if self.cursor_scale.set(scale)?
            && !self.desktop_controls_active
            && !self.touch_keyboard.visible()
            && self.controller_connected
        {
            set_cursor_hidden(true)
                .map_err(|error| format!("Falha restaurando cursor oculto do Windows: {error}"))?;
        }
        Ok(())
    }

    pub(crate) fn update_mouse_mode(
        &mut self,
        enabled: bool,
        connected: bool,
    ) -> Result<(), String> {
        if !enabled || !connected {
            self.finish_window_switcher(true)?;
        }
        self.desktop_controls_active = enabled && connected;
        self.controller_connected = connected;
        self.navigation_hook.set_active(
            self.desktop_controls_active && !self.touch_keyboard.uses_native_gamepad(),
        )?;
        self.paused_cursor
            .sync(
                enabled || self.touch_keyboard.visible(),
                connected,
                set_cursor_hidden,
            )
            .map_err(|error| format!("Falha atualizando cursor do Windows: {error}"))
    }

    pub fn system_keyboard_uses_gamepad(&self) -> bool {
        false
    }

    pub fn keyboard_presentation(&self) -> KeyboardPresentation {
        KeyboardPresentation::System
    }

    pub fn keyboard_overlay_error(&self) -> Option<&str> {
        None
    }

    pub fn update_keyboard_overlay(&mut self, session: &mut KeyboardSession) -> Result<(), String> {
        let was_visible = self.touch_keyboard.visible();
        let result = self.touch_keyboard.update(session);
        if self.controller_connected
            && !was_visible
            && self.touch_keyboard.visible()
            && !self.touch_keyboard.uses_native_gamepad()
            && let Some(bounds) = self.touch_keyboard.bounds
        {
            let (x, y) = bounds.center();
            unsafe { SetPhysicalCursorPos(x, y) }.map_err(|error| error.to_string())?;
        }
        self.navigation_hook.set_active(
            self.desktop_controls_active && !self.touch_keyboard.uses_native_gamepad(),
        )?;
        self.paused_cursor
            .sync(
                self.desktop_controls_active || self.touch_keyboard.visible(),
                self.controller_connected,
                set_cursor_hidden,
            )
            .map_err(|error| error.to_string())?;
        result
    }

    fn refresh_native_dpad_target(&mut self, force: bool) {
        if !force
            && self
                .last_target_check
                .is_some_and(|last| last.elapsed() < NAVIGATION_TARGET_CHECK_INTERVAL)
        {
            return;
        }
        self.last_target_check = Some(Instant::now());

        let Some(windows) = Self::navigation_windows() else {
            self.target_cache = Default::default();
            self.native_dpad.set_target_active(false);
            return;
        };
        let identities = windows.map(|window| {
            let mut process_id = 0;
            unsafe {
                GetWindowThreadProcessId(window, Some(&mut process_id));
            }
            (window.0 as isize, process_id)
        });
        let active = resolve_native_windows(&mut self.target_cache, identities, |handle| {
            Self::is_native_system_window(HWND(handle as *mut _))
        });
        self.native_dpad.set_target_active(active);
    }

    pub fn apply(&mut self, action: Action) -> Result<(), String> {
        if self.window_switcher.is_open() {
            match action {
                Action::Key(SystemKey::Enter) => {
                    self.finish_window_switcher(false)?;
                    return Ok(());
                }
                Action::Key(SystemKey::Escape) => {
                    self.finish_window_switcher(true)?;
                    return Ok(());
                }
                _ => {}
            }
        }
        if self.touch_keyboard.active()
            && self.touch_keyboard.gamepad_session_active()
            && !self.touch_keyboard.uses_native_gamepad()
            && let Action::Dpad(direction) = action
        {
            if let Some(bounds) = self.touch_keyboard.bounds {
                let mut cursor = POINT::default();
                unsafe { GetPhysicalCursorPos(&mut cursor) }.map_err(|error| error.to_string())?;
                let (x, y) = bounds.move_cursor_by_key(cursor.x, cursor.y, direction);
                unsafe { SetPhysicalCursorPos(x, y) }.map_err(|error| error.to_string())?;
            }
            return Ok(());
        }
        let Some(action) = filter_desktop_action(
            action,
            self.navigation_hook.is_active(),
            self.touch_keyboard.uses_native_gamepad(),
        ) else {
            return Ok(());
        };
        if self.touch_keyboard.active() {
            match action {
                Action::Move(dx, dy) => {
                    if let Some(bounds) = self.touch_keyboard.bounds {
                        let mut cursor = POINT::default();
                        unsafe { GetPhysicalCursorPos(&mut cursor) }
                            .map_err(|error| error.to_string())?;
                        let (x, y) = bounds.constrain(cursor.x, cursor.y, dx, dy);
                        unsafe { SetPhysicalCursorPos(x, y) }.map_err(|error| error.to_string())?;
                    }
                    return Ok(());
                }
                Action::Mouse(MouseButton::Left, true) if self.keyboard_click_target() => {}
                Action::Mouse(_, false) | Action::DpadReleased => {}
                _ => return Ok(()),
            }
        }
        let result = match action {
            Action::OnScreenKeyboard(_) => return Ok(()),
            Action::Dpad(direction) => {
                self.refresh_native_dpad_target(true);
                if !self.native_dpad.should_synthesize() {
                    return Ok(());
                }
                self.input.key(
                    match direction {
                        DpadDirection::Up => Key::UpArrow,
                        DpadDirection::Down => Key::DownArrow,
                        DpadDirection::Left => Key::LeftArrow,
                        DpadDirection::Right => Key::RightArrow,
                    },
                    Direction::Click,
                )
            }
            Action::DpadReleased => {
                self.native_dpad.release_dpad();
                return Ok(());
            }
            Action::Move(x, y) => self.input.move_mouse(x, y, Coordinate::Rel),
            Action::Scroll(amount) => self.input.scroll(amount, ScrollAxis::Vertical),
            Action::Mouse(button, pressed) => self.input.button(
                match button {
                    MouseButton::Left => enigo::Button::Left,
                    MouseButton::Right => enigo::Button::Right,
                },
                if pressed {
                    Direction::Press
                } else {
                    Direction::Release
                },
            ),
            Action::Key(key) => self.input.key(
                match key {
                    SystemKey::Enter => Key::Return,
                    SystemKey::Escape => Key::Escape,
                    SystemKey::Up => Key::UpArrow,
                    SystemKey::Down => Key::DownArrow,
                    SystemKey::Left => Key::LeftArrow,
                    SystemKey::Right => Key::RightArrow,
                    SystemKey::Tab => Key::Tab,
                    SystemKey::Meta => Key::Meta,
                },
                Direction::Click,
            ),
            Action::Keyboard(input) => {
                self.emit_keyboard_input(input, KeyboardModifiers::default())
            }
            Action::KeyboardChord(input, modifiers) => self.emit_keyboard_input(input, modifiers),
            Action::NextWindow => {
                let step = self.window_switcher.next();
                let pressed = if step == WindowSwitcherStep::Open {
                    let result = self.input.key(Key::Alt, Direction::Press);
                    if result.is_err() {
                        self.window_switcher.finish();
                    }
                    result
                } else {
                    Ok(())
                };
                if pressed.is_ok() {
                    self.input.key(Key::Tab, Direction::Click)
                } else {
                    pressed
                }
            }
        };
        result.map_err(|error| format!("Falha enviando entrada ao Windows: {error}"))
    }

    fn finish_window_switcher(&mut self, cancel: bool) -> Result<(), String> {
        if !self.window_switcher.is_open() {
            return Ok(());
        }
        let cancel_result = if cancel {
            self.input
                .key(Key::Escape, Direction::Click)
                .map_err(|error| error.to_string())
        } else {
            Ok(())
        };
        let release = self
            .input
            .key(Key::Alt, Direction::Release)
            .map_err(|error| error.to_string());
        if release.is_ok() {
            self.window_switcher.finish();
        }
        cancel_result.and(release)
    }

    fn keyboard_click_target(&self) -> bool {
        let mut cursor = POINT::default();
        if unsafe { GetPhysicalCursorPos(&mut cursor) }.is_err()
            || !self
                .touch_keyboard
                .bounds
                .is_some_and(|bounds| bounds.contains(cursor.x, cursor.y))
        {
            return false;
        }
        let window = unsafe { WindowFromPhysicalPoint(cursor) };
        let mut process_id = 0;
        unsafe { GetWindowThreadProcessId(window, Some(&mut process_id)) };
        let Ok(process) =
            (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) })
        else {
            return false;
        };
        let mut path = [0_u16; 32_768];
        let mut length = path.len() as u32;
        let queried = unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(path.as_mut_ptr()),
                &mut length,
            )
        };
        let _ = unsafe { CloseHandle(process) };
        if queried.is_err() {
            return false;
        }
        let path = String::from_utf16_lossy(&path[..length as usize]);
        ["WINDIR", "CommonProgramFiles"].into_iter().any(|name| {
            std::env::var(name).is_ok_and(|directory| is_touch_keyboard_process(&path, &directory))
        })
    }

    fn emit_keyboard_input(
        &mut self,
        input: KeyboardInput,
        modifiers: KeyboardModifiers,
    ) -> Result<(), enigo::InputError> {
        let key = match input {
            KeyboardInput::Character(character) => Key::Unicode(character),
            KeyboardInput::Function(number) => match number {
                1 => Key::F1,
                2 => Key::F2,
                3 => Key::F3,
                4 => Key::F4,
                5 => Key::F5,
                6 => Key::F6,
                7 => Key::F7,
                8 => Key::F8,
                9 => Key::F9,
                10 => Key::F10,
                11 => Key::F11,
                12 => Key::F12,
                _ => return Ok(()),
            },
            KeyboardInput::PrintScreen => Key::PrintScr,
            KeyboardInput::Backspace => Key::Backspace,
            KeyboardInput::Enter => Key::Return,
            KeyboardInput::Escape => Key::Escape,
            KeyboardInput::Space => Key::Space,
            KeyboardInput::Tab => Key::Tab,
            KeyboardInput::Menu => Key::Apps,
        };
        let modifiers: Vec<_> = [
            (modifiers.control, Key::Control),
            (modifiers.alt, Key::Alt),
            (modifiers.meta, Key::Meta),
        ]
        .into_iter()
        .filter_map(|(enabled, key)| enabled.then_some(key))
        .collect();
        let mut result = Ok(());
        for modifier in &modifiers {
            result = result.and_then(|()| self.input.key(*modifier, Direction::Press));
        }
        result = result.and_then(|()| self.input.key(key, Direction::Click));
        for modifier in modifiers.iter().rev() {
            let released = self.input.key(*modifier, Direction::Release);
            result = result.and(released);
        }
        result
    }

    fn is_native_system_window(window: HWND) -> Option<bool> {
        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(window, Some(&mut process_id));
        }
        if process_id == 0 {
            return None;
        }

        let process =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };
        // A positive API result is sufficient; a negative result or error uses
        // the known shell/app rules. Neither package identity nor path is required
        // for the automatic branch.
        let native = resolve_native_target(unsafe { IsImmersiveProcess(process).is_ok() }, || {
            let mut path = [0_u16; 32_768];
            let mut length = path.len() as u32;
            unsafe {
                QueryFullProcessImageNameW(
                    process,
                    PROCESS_NAME_WIN32,
                    PWSTR(path.as_mut_ptr()),
                    &mut length,
                )
                .ok()?;
            }
            let windows_directory = std::env::var("WINDIR").ok()?;
            let class = Self::window_class(window);
            if class.is_empty() {
                return None;
            }
            let image_path = String::from_utf16_lossy(&path[..length as usize]);
            Some(is_native_windows_target(
                &image_path,
                &windows_directory,
                &class,
            ))
        });
        let _ = unsafe { CloseHandle(process) };
        native
    }

    fn window_class(window: HWND) -> String {
        let mut name = [0_u16; 128];
        let length = unsafe { GetClassNameW(window, &mut name) };
        if length <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&name[..length as usize])
    }

    fn hosted_content_window(frame: HWND) -> Option<HWND> {
        if !Self::window_class(frame).eq_ignore_ascii_case("ApplicationFrameWindow") {
            return None;
        }

        unsafe extern "system" fn find_content(window: HWND, parameter: LPARAM) -> BOOL {
            if unsafe { IsWindowVisible(window) }.as_bool()
                && Platform::window_class(window).eq_ignore_ascii_case("Windows.UI.Core.CoreWindow")
            {
                // EnumChildWindows invokes this synchronously while `content` is alive.
                unsafe { *(parameter.0 as *mut HWND) = window };
                return false.into();
            }
            true.into()
        }

        let mut content = HWND::default();
        unsafe {
            let _ = EnumChildWindows(
                Some(frame),
                Some(find_content),
                LPARAM((&mut content as *mut HWND) as isize),
            );
        }
        (!content.is_invalid()).then_some(content)
    }

    fn navigation_windows() -> Option<[HWND; 4]> {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.is_invalid() {
                return None;
            }

            let mut thread = GUITHREADINFO {
                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            let focus = GetGUIThreadInfo(0, &mut thread)
                .ok()
                .map(|()| thread.hwndFocus)
                .filter(|window| !window.is_invalid())
                .unwrap_or(foreground);
            let root = GetAncestor(focus, GA_ROOT);
            let root = if root.is_invalid() { foreground } else { root };
            let content = [foreground, root]
                .into_iter()
                .find_map(Self::hosted_content_window)
                .unwrap_or_default();
            Some([content, focus, foreground, root])
        }
    }
}

impl Drop for Platform {
    fn drop(&mut self) {
        let _ = self.update_mouse_mode(true, false);
        let _ = self.input.button(enigo::Button::Left, Direction::Release);
        let _ = self.input.button(enigo::Button::Right, Direction::Release);
    }
}

// ShowCursor only affects the calling thread's windows. Replace the standard
// shapes temporarily so the cursor is hidden over other applications as well.
// No registry changes: restoring reloads the user's configured cursor scheme.
fn set_cursor_hidden(hidden: bool) -> windows::core::Result<()> {
    if !hidden {
        return unsafe {
            SystemParametersInfoW(
                SPI_SETCURSORS,
                0,
                None,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };
    }
    for id in [
        OCR_NORMAL,
        OCR_IBEAM,
        OCR_WAIT,
        OCR_CROSS,
        OCR_UP,
        OCR_SIZENWSE,
        OCR_SIZENESW,
        OCR_SIZEWE,
        OCR_SIZENS,
        OCR_SIZEALL,
        OCR_NO,
        OCR_HAND,
        OCR_APPSTARTING,
        OCR_HELP,
    ] {
        // Monochrome AND=1, XOR=0 preserves the screen underneath each pixel.
        let and_mask = [0xff_u8; 128];
        let xor_mask = [0_u8; 128];
        unsafe {
            let cursor = CreateCursor(
                None,
                0,
                0,
                32,
                32,
                and_mask.as_ptr().cast(),
                xor_mask.as_ptr().cast(),
            )?;
            SetSystemCursor(cursor, id)?;
        }
    }
    Ok(())
}
