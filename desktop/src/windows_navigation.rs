#[derive(Clone, Copy)]
pub(crate) struct KeyboardBounds {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl KeyboardBounds {
    pub(crate) fn new(left: i32, top: i32, right: i32, bottom: i32) -> Option<Self> {
        (right > left && bottom > top).then_some(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    pub(crate) fn center(self) -> (i32, i32) {
        (
            ((i64::from(self.left) + i64::from(self.right)) / 2) as i32,
            ((i64::from(self.top) + i64::from(self.bottom)) / 2) as i32,
        )
    }

    pub(crate) fn contains(self, x: i32, y: i32) -> bool {
        (self.left..self.right).contains(&x) && (self.top..self.bottom).contains(&y)
    }

    pub(crate) fn constrain(self, x: i32, y: i32, dx: i32, dy: i32) -> (i32, i32) {
        (
            x.saturating_add(dx).clamp(self.left, self.right - 1),
            y.saturating_add(dy).clamp(self.top, self.bottom - 1),
        )
    }

    pub(crate) fn move_cursor_by_key(self, x: i32, y: i32, direction: DpadDirection) -> (i32, i32) {
        let step = |start: i32, end: i32, divisions: i64| {
            ((i64::from(end) - i64::from(start)) / divisions).clamp(1, i64::from(i32::MAX)) as i32
        };
        let (dx, dy) = match direction {
            DpadDirection::Up => (0, -step(self.top, self.bottom, 5)),
            DpadDirection::Down => (0, step(self.top, self.bottom, 5)),
            DpadDirection::Left => (-step(self.left, self.right, 18), 0),
            DpadDirection::Right => (step(self.left, self.right, 18), 0),
        };
        self.constrain(x, y, dx, dy)
    }
}

pub(crate) fn is_touch_keyboard_process(path: &str, directory: &str) -> bool {
    let path = path.to_ascii_lowercase();
    let root = format!(
        "{}\\",
        directory.trim_end_matches('\\').to_ascii_lowercase()
    );
    path.starts_with(&root)
        && (path.ends_with("\\textinputhost.exe") || path.ends_with("\\tabtip.exe"))
}

use crate::control::{Action, DpadDirection};

#[derive(Default)]
pub(crate) struct TouchKeyboardState {
    pub requested: bool,
    pub visible: bool,
    pub gamepad_session: bool,
    pending: Option<bool>,
}

impl TouchKeyboardState {
    pub(crate) fn observe(&mut self, requested: bool, visible: bool) -> Option<bool> {
        let changed = requested != self.requested;
        if changed && requested {
            self.gamepad_session = true;
        }
        let completed = self.pending == Some(visible);
        if completed {
            self.pending = None;
        }
        if !changed && !completed && self.pending.is_none() && self.visible != visible {
            self.requested = visible;
        } else {
            self.requested = requested;
        }
        self.visible = visible;
        if !self.requested && !visible && self.pending.is_none() {
            self.gamepad_session = false;
        }
        (self.pending.is_none() && self.requested != visible).then_some(self.requested)
    }

    pub(crate) fn close_after_visibility_error(&mut self, requested: bool) -> Option<bool> {
        if requested || !self.visible {
            return None;
        }
        self.observe(requested, self.visible)
    }

    pub(crate) fn command_sent(&mut self, visible: bool) {
        self.pending = Some(visible);
    }

    pub(crate) fn waiting(&self) -> bool {
        self.pending.is_some()
    }
}

pub(crate) fn filter_desktop_action(
    action: Action,
    filter_active: bool,
    native_keyboard_active: bool,
) -> Option<Action> {
    if (native_keyboard_active && !matches!(action, Action::Mouse(_, false) | Action::DpadReleased))
        || (filter_active && matches!(action, Action::Dpad(_)))
    {
        None
    } else {
        Some(action)
    }
}

#[derive(Default)]
pub(crate) struct GamepadKeyFilter {
    held: [Option<bool>; 24],
}

impl GamepadKeyFilter {
    pub(crate) fn process(&mut self, key: u32, down: bool, enabled: bool) -> bool {
        // VK_GAMEPAD_A through VK_GAMEPAD_RIGHT_THUMBSTICK_LEFT (Winuser.h).
        let Some(held) = key
            .checked_sub(0xc3)
            .and_then(|index| self.held.get_mut(index as usize))
        else {
            return false;
        };
        if down {
            *held.get_or_insert(enabled)
        } else {
            held.take().unwrap_or(false)
        }
    }
}

#[derive(Default)]
pub(crate) struct PausedCursor {
    hidden: bool,
}

impl PausedCursor {
    pub(crate) fn sync<E>(
        &mut self,
        enabled: bool,
        connected: bool,
        mut apply: impl FnMut(bool) -> Result<(), E>,
    ) -> Result<(), E> {
        let hidden = !enabled && connected;
        if self.hidden != hidden {
            // A failed hide can have changed some system cursors already.
            self.hidden = true;
            apply(hidden)?;
            self.hidden = hidden;
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct NativeDpadTargetCache {
    resolved: Option<((isize, u32), bool)>,
}

impl NativeDpadTargetCache {
    pub(crate) fn resolve(
        &mut self,
        identity: (isize, u32),
        identify: impl FnOnce() -> Option<bool>,
    ) -> bool {
        if let Some((cached_identity, active)) = self.resolved
            && cached_identity == identity
        {
            return active;
        }
        let active = identify();
        self.resolved = active.map(|active| (identity, active));
        active.unwrap_or(false)
    }
}

pub(crate) fn resolve_native_windows<const N: usize>(
    caches: &mut [NativeDpadTargetCache; N],
    windows: [(isize, u32); N],
    mut identify: impl FnMut(isize) -> Option<bool>,
) -> bool {
    caches.iter_mut().zip(windows).any(|(cache, identity)| {
        identity.0 != 0 && cache.resolve(identity, || identify(identity.0))
    })
}

pub(crate) fn resolve_native_target(
    immersive: bool,
    fallback: impl FnOnce() -> Option<bool>,
) -> Option<bool> {
    if immersive { Some(true) } else { fallback() }
}

#[derive(Default)]
pub(crate) struct NativeDpadRouting {
    target_active: bool,
    dpad_latched: bool,
}

impl NativeDpadRouting {
    pub(crate) fn set_target_active(&mut self, active: bool) {
        self.target_active = active;
    }

    pub(crate) fn should_synthesize(&mut self) -> bool {
        if self.target_active {
            self.dpad_latched = true;
            false
        } else {
            !self.dpad_latched
        }
    }

    pub(crate) fn release_dpad(&mut self) {
        self.dpad_latched = false;
    }
}

pub(crate) fn is_native_windows_target(
    process_image_path: &str,
    windows_directory: &str,
    window_class: &str,
) -> bool {
    let image_path = normalize_path(process_image_path);
    let windows_directory = normalize_path(windows_directory);
    if image_path.is_empty() || windows_directory.is_empty() {
        return false;
    }

    if image_path.ends_with("/playnite.fullscreenapp.exe") {
        return true;
    }

    if image_path == format!("{windows_directory}/system32/applicationframehost.exe")
        && window_class.eq_ignore_ascii_case("ApplicationFrameWindow")
    {
        return true;
    }

    if is_child_path(&image_path, &format!("{windows_directory}/systemapps"))
        || is_child_path(
            &image_path,
            &format!("{windows_directory}/immersivecontrolpanel"),
        )
        || image_path == format!("{windows_directory}/system32/shellhost.exe")
            && window_class.eq_ignore_ascii_case("ControlCenterWindow")
    {
        return true;
    }

    image_path == format!("{windows_directory}/explorer.exe")
        && matches!(
            window_class.to_ascii_lowercase().as_str(),
            "shell_traywnd"
                | "shell_secondarytraywnd"
                | "notifyiconoverflowwindow"
                | "xamlexplorerhostislandwindow"
                | "windows.ui.core.corewindow"
                | "windows.ui.composition.desktopwindowcontentbridge"
        )
}

fn normalize_path(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn is_child_path(path: &str, directory: &str) -> bool {
    path.strip_prefix(directory)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_explicit_keyboard_request_starts_native_gamepad_navigation() {
        let mut keyboard = TouchKeyboardState::default();
        keyboard.observe(false, true);
        assert!(!keyboard.gamepad_session, "automatic opening is not R3");
        keyboard.observe(true, false);
        assert!(!keyboard.gamepad_session);

        assert_eq!(keyboard.observe(true, false), Some(true));
        assert!(keyboard.gamepad_session);
        keyboard.command_sent(true);
        keyboard.observe(true, true);
        assert!(keyboard.gamepad_session);
        keyboard.observe(false, true);
        assert!(keyboard.gamepad_session, "keep native input during closing");
        keyboard.command_sent(false);
        keyboard.observe(false, false);
        assert!(!keyboard.gamepad_session);

        keyboard.observe(true, false);
        keyboard.command_sent(true);
        keyboard.observe(false, false);
        assert!(keyboard.gamepad_session, "opening has not finished yet");
        keyboard.observe(false, true);
        keyboard.command_sent(false);
        keyboard.observe(false, false);
        assert!(!keyboard.gamepad_session);

        keyboard.observe(true, false);
        keyboard.command_sent(true);
        keyboard.observe(true, true);
        keyboard.observe(true, false);
        assert!(
            !keyboard.gamepad_session,
            "native dismissal ends the session"
        );
    }

    #[test]
    fn native_keyboard_receives_no_synthetic_pointer_clicks_or_shortcuts() {
        use crate::control::{DpadDirection, KeyboardInput, MouseButton, SystemKey};

        for action in [
            Action::Move(20, -20),
            Action::Scroll(1),
            Action::Mouse(MouseButton::Left, true),
            Action::Mouse(MouseButton::Right, true),
            Action::Key(SystemKey::Meta),
            Action::Dpad(DpadDirection::Right),
            Action::Keyboard(KeyboardInput::Enter),
            Action::NextWindow,
        ] {
            assert_eq!(
                filter_desktop_action(action, false, true),
                None,
                "{action:?}"
            );
        }
        for action in [
            Action::Mouse(MouseButton::Left, false),
            Action::Mouse(MouseButton::Right, false),
            Action::DpadReleased,
        ] {
            assert_eq!(filter_desktop_action(action, false, true), Some(action));
        }
    }

    #[test]
    fn touch_keyboard_waits_for_visibility_and_tracks_native_dismissal() {
        let mut keyboard = TouchKeyboardState::default();
        assert_eq!(keyboard.observe(true, false), Some(true));
        keyboard.command_sent(true);
        assert!(keyboard.waiting());
        assert_eq!(keyboard.observe(true, false), None);
        assert_eq!(keyboard.observe(true, true), None);
        assert!(!keyboard.waiting());
        assert!(keyboard.visible && keyboard.requested);
        assert_eq!(keyboard.observe(true, false), None);
        assert!(!keyboard.requested);
    }

    #[test]
    fn touch_keyboard_can_request_close_from_last_known_visibility() {
        let mut keyboard = TouchKeyboardState::default();
        assert_eq!(keyboard.observe(true, false), Some(true));
        keyboard.command_sent(true);
        assert_eq!(keyboard.observe(true, true), None);

        assert_eq!(keyboard.close_after_visibility_error(false), Some(false));
        assert!(!keyboard.requested);
    }

    #[test]
    fn touch_keyboard_adopts_windows_automatic_open_without_toggling_it() {
        let mut keyboard = TouchKeyboardState::default();
        assert_eq!(keyboard.observe(false, true), None);
        assert!(keyboard.requested && keyboard.visible);
        assert_eq!(keyboard.observe(true, false), None);
        assert!(!keyboard.requested && !keyboard.visible);
    }

    #[test]
    fn touch_keyboard_serializes_a_close_requested_during_opening() {
        let mut keyboard = TouchKeyboardState::default();
        assert_eq!(keyboard.observe(true, false), Some(true));
        keyboard.command_sent(true);
        assert_eq!(keyboard.observe(false, false), None);
        assert_eq!(keyboard.observe(false, true), Some(false));
        keyboard.command_sent(false);
        assert_eq!(keyboard.observe(false, false), None);
        assert!(!keyboard.requested && !keyboard.visible);
    }

    #[test]
    fn touch_keyboard_reopens_after_a_hide_already_in_flight() {
        let mut keyboard = TouchKeyboardState::default();
        keyboard.observe(true, false);
        keyboard.command_sent(true);
        keyboard.observe(true, true);
        assert_eq!(keyboard.observe(false, true), Some(false));
        keyboard.command_sent(false);
        assert_eq!(keyboard.observe(true, true), None);
        assert_eq!(keyboard.observe(true, false), Some(true));
        assert!(keyboard.requested);
    }

    #[test]
    fn windows_mouse_mode_does_not_send_dpad_arrows_to_other_programs() {
        use crate::control::{
            Action, Button, ControlSettings, GamepadFrame, MouseButton, MouseController,
        };
        use std::time::Duration;

        let tick = Duration::from_millis(10);
        for direction in [Button::Up, Button::Down, Button::Left, Button::Right] {
            let mut controller = MouseController::default();
            let mut settings = ControlSettings::default();
            controller.update(Some(&GamepadFrame::default()), tick, &mut settings);
            let mut frame = GamepadFrame {
                x: 1.0,
                ..Default::default()
            };
            frame.buttons[direction as usize] = true;
            frame.buttons[Button::South as usize] = true;
            let delivered: Vec<_> = controller
                .update(Some(&frame), tick, &mut settings)
                .into_iter()
                .filter_map(|action| filter_desktop_action(action, true, false))
                .collect();
            assert!(delivered.contains(&Action::Mouse(MouseButton::Left, true)));
            assert!(
                delivered
                    .iter()
                    .any(|action| matches!(action, Action::Move(x, 0) if *x > 0))
            );
            assert!(
                !delivered
                    .iter()
                    .any(|action| matches!(action, Action::Dpad(_)))
            );
            for _ in 0..100 {
                assert!(
                    controller
                        .update(Some(&frame), tick, &mut settings)
                        .into_iter()
                        .filter_map(|action| filter_desktop_action(action, true, false))
                        .all(|action| !matches!(action, Action::Dpad(_)))
                );
            }
            assert!(
                controller
                    .update(Some(&GamepadFrame::default()), tick, &mut settings)
                    .into_iter()
                    .filter_map(|action| filter_desktop_action(action, true, false))
                    .any(|action| action == Action::Mouse(MouseButton::Left, false))
            );
        }
    }

    #[test]
    fn windows_mouse_filter_preserves_keyboard_selection_and_typing() {
        use crate::control::{Action, Button, ControlSettings, GamepadFrame, MouseController};
        use std::time::Duration;

        let tick = Duration::from_millis(10);
        let mut controller = MouseController::default();
        controller.set_system_keyboard_uses_gamepad(true);
        let mut settings = ControlSettings::default();
        let neutral = GamepadFrame::default();
        controller.update(Some(&neutral), tick, &mut settings);
        let mut frame = GamepadFrame::default();
        frame.buttons[Button::RightStick as usize] = true;
        controller.update(Some(&frame), tick, &mut settings);
        controller.update(Some(&neutral), tick, &mut settings);
        controller.keyboard.selected_row = 3;
        controller.keyboard.selected_column = 5;
        frame = neutral;
        frame.buttons[Button::Right as usize] = true;
        assert!(
            controller
                .update(Some(&frame), tick, &mut settings)
                .into_iter()
                .filter_map(|action| filter_desktop_action(action, true, false))
                .next()
                .is_none()
        );
        assert_eq!(controller.keyboard.selected_column, 6);
        controller.update(Some(&neutral), tick, &mut settings);
        frame = neutral;
        frame.buttons[Button::South as usize] = true;
        let typed: Vec<_> = controller
            .update(Some(&frame), tick, &mut settings)
            .into_iter()
            .filter_map(|action| filter_desktop_action(action, true, false))
            .collect();
        assert!(matches!(typed.as_slice(), [Action::Keyboard(_)]));
    }

    #[test]
    fn desktop_filter_blocks_gamepad_navigation_but_passes_keyboard_and_mouse_keys() {
        let mut filter = GamepadKeyFilter::default();
        for key in 0..=255 {
            let gamepad_key = (0xc3..=0xda).contains(&key);
            assert_eq!(
                filter.process(key, true, true),
                gamepad_key,
                "down {key:#x}"
            );
            assert_eq!(
                filter.process(key, true, true),
                gamepad_key,
                "repeat {key:#x}"
            );
            assert_eq!(filter.process(key, false, true), gamepad_key, "up {key:#x}");
        }
    }

    #[test]
    fn game_mode_pause_and_disconnect_allow_native_gamepad_navigation() {
        let mut filter = GamepadKeyFilter::default();
        for key in 0xc3..=0xda {
            assert!(!filter.process(key, true, false));
            assert!(!filter.process(key, false, false));
        }
    }

    #[test]
    fn switching_filter_mode_preserves_each_held_keys_press_and_release_pair() {
        let mut filter = GamepadKeyFilter::default();
        for key in 0xc3..=0xda {
            assert!(
                !filter.process(key, false, true),
                "unknown release must pass"
            );
            assert!(!filter.process(key, true, false));
            assert!(
                !filter.process(key, true, true),
                "forwarded press keeps repeating"
            );
            assert!(
                !filter.process(key, false, true),
                "forwarded press needs its release"
            );
            assert!(filter.process(key, true, true));
            assert!(
                filter.process(key, true, false),
                "blocked press stays blocked"
            );
            assert!(filter.process(key, false, false));
            assert!(
                !filter.process(key, true, false),
                "next press follows the new mode"
            );
            assert!(!filter.process(key, false, false));
        }
    }

    #[test]
    fn immersive_detection_suppresses_duplicates_without_reading_a_process_path() {
        let mut fallback_reads = 0;
        let mut cache = NativeDpadTargetCache::default();
        let mut routing = NativeDpadRouting::default();
        routing.set_target_active(cache.resolve((17, 42), || {
            resolve_native_target(true, || {
                fallback_reads += 1;
                None
            })
        }));
        for _ in 0..10 {
            assert!(
                !routing.should_synthesize(),
                "immersive app received a duplicate arrow"
            );
        }
        assert_eq!(fallback_reads, 0);
        assert!(cache.resolve((17, 42), || panic!("recognized process should be cached")));
        routing.set_target_active(false);
        assert!(
            !routing.should_synthesize(),
            "held D-pad must stay suppressed across windows"
        );
        routing.release_dpad();
        assert!(routing.should_synthesize());
    }

    #[test]
    fn hybrid_detection_preserves_shell_fallback_and_ordinary_app_arrows() {
        for (path, class, native) in [
            (
                r"C:\Windows\System32\ShellHost.exe",
                "ControlCenterWindow",
                true,
            ),
            (
                r"C:\Program Files\WindowsApps\Contoso.App\App.exe",
                "DesktopWindow",
                false,
            ),
            (r"C:\Windows\explorer.exe", "CabinetWClass", false),
        ] {
            let mut routing = NativeDpadRouting::default();
            routing.set_target_active(
                resolve_native_target(false, || {
                    Some(is_native_windows_target(path, r"C:\Windows", class))
                })
                .unwrap(),
            );
            assert_eq!(routing.should_synthesize(), !native, "{path}");
        }
    }

    #[test]
    fn hybrid_lookup_failure_is_retried_when_automatic_detection_becomes_available() {
        let mut cache = NativeDpadTargetCache::default();
        assert!(!cache.resolve((17, 42), || resolve_native_target(false, || None)));
        assert!(cache.resolve((17, 42), || resolve_native_target(true, || None)));
    }

    #[test]
    fn hosted_content_is_checked_before_the_host_and_rechecked_when_its_process_changes() {
        let mut caches = Default::default();
        let mut windows = [(0, 0), (10, 1), (11, 1), (12, 1)];
        assert!(!resolve_native_windows(&mut caches, windows, |_| Some(
            false
        )));

        windows[0] = (20, 2);
        let mut lookups = Vec::new();
        assert!(resolve_native_windows(&mut caches, windows, |handle| {
            lookups.push(handle);
            resolve_native_target(true, || panic!("native content needs no path fallback"))
        }));
        assert_eq!(lookups, [20]);

        windows[0] = (20, 3);
        assert!(!resolve_native_windows(&mut caches, windows, |handle| {
            assert_eq!(handle, 20, "unchanged host should stay cached");
            Some(false)
        }));
        windows[0] = (0, 0);
        assert!(!resolve_native_windows(&mut caches, windows, |_| panic!(
            "cached host"
        )));
    }

    #[test]
    fn quick_settings_shellhost_does_not_receive_duplicate_directional_repeats() {
        for (process, windows_directory) in [
            (r"C:\Windows\System32\ShellHost.exe", r"C:\Windows"),
            ("d:/WINDOWS/system32/SHELLHOST.EXE", "D:/Windows/"),
        ] {
            let mut routing = NativeDpadRouting::default();
            routing.set_target_active(is_native_windows_target(
                process,
                windows_directory,
                "ControlCenterWindow",
            ));
            for _ in 0..10 {
                assert!(
                    !routing.should_synthesize(),
                    "duplicate arrow for {process}"
                );
            }
            routing.release_dpad();
            assert!(!routing.should_synthesize());
            routing.release_dpad();
            routing.set_target_active(false);
            assert!(
                routing.should_synthesize(),
                "ordinary apps still need arrows"
            );
        }
    }

    #[test]
    fn shellhost_detection_requires_the_system_path_and_panel_class() {
        for (process, class) in [
            (r"C:\Other\ShellHost.exe", "ControlCenterWindow"),
            (r"C:\Windows\System32\Other.exe", "ControlCenterWindow"),
            (r"C:\Windows\System32\ShellHost.exe", "OtherWindow"),
        ] {
            assert!(!is_native_windows_target(process, r"C:\Windows", class));
        }
    }

    #[test]
    fn native_focused_panel_or_foreground_wins_over_non_native_root() {
        for native in [11, 22, 33] {
            let mut caches = Default::default();
            let windows = [(11, 1), (22, 2), (33, 3)];
            let mut routing = NativeDpadRouting::default();
            routing.set_target_active(resolve_native_windows(&mut caches, windows, |handle| {
                Some(handle == native)
            }));
            assert!(
                !routing.should_synthesize(),
                "native surface {native} was missed"
            );
            assert!(!routing.should_synthesize());
            routing.release_dpad();
            assert!(!routing.should_synthesize());
        }
    }

    #[test]
    fn paused_cursor_hides_once_and_restores_on_resume_or_disconnect() {
        let mut cursor = PausedCursor::default();
        let mut changes = Vec::new();
        for (enabled, connected) in [
            (true, true),
            (false, true),
            (false, true),
            (true, true),
            (false, true),
            (false, false),
            (false, false),
        ] {
            cursor
                .sync(enabled, connected, |hidden| {
                    changes.push(hidden);
                    Ok::<_, ()>(())
                })
                .unwrap();
        }
        assert_eq!(changes, [true, false, true, false]);
    }

    #[test]
    fn failed_cursor_hide_still_requires_restoration() {
        let mut cursor = PausedCursor::default();
        assert!(cursor.sync(false, true, |_| Err("partial hide")).is_err());
        let mut restored = false;
        cursor
            .sync(true, false, |hidden| {
                restored = !hidden;
                Ok::<_, ()>(())
            })
            .unwrap();
        assert!(restored);
    }

    #[test]
    fn failed_cursor_restore_can_be_retried() {
        let mut cursor = PausedCursor::default();
        cursor.sync(false, true, |_| Ok::<_, ()>(())).unwrap();
        assert!(cursor.sync(true, true, |_| Err(())).is_err());
        let mut retried = false;
        cursor
            .sync(true, true, |hidden| {
                retried = !hidden;
                Ok::<_, ()>(())
            })
            .unwrap();
        assert!(retried);
    }

    #[test]
    fn failed_target_lookup_is_retried_and_then_stops_synthetic_dpad() {
        let mut cache = NativeDpadTargetCache::default();
        let mut routing = NativeDpadRouting::default();
        let identity = (17, 42);

        routing.set_target_active(cache.resolve(identity, || None));
        assert!(routing.should_synthesize());

        routing.set_target_active(cache.resolve(identity, || Some(true)));
        assert!(!routing.should_synthesize());
    }

    #[test]
    fn resolved_targets_are_cached_and_window_changes_trigger_lookup() {
        let mut cache = NativeDpadTargetCache::default();
        assert!(!cache.resolve((17, 42), || Some(false)));
        assert!(!cache.resolve((17, 42), || panic!("resolved target should be cached")));
        assert!(cache.resolve((18, 42), || Some(true)));
        assert!(cache.resolve((18, 42), || panic!("resolved target should be cached")));
    }

    #[test]
    fn recognizes_windows_system_app_surfaces_but_not_other_packaged_apps() {
        assert!(is_native_windows_target(
            r"C:\Windows\SystemApps\ShellExperienceHost_cw5n1h2txyewy\ShellExperienceHost.exe",
            r"C:\Windows",
            "Windows.UI.Core.CoreWindow",
        ));
        assert!(is_native_windows_target(
            r"C:\Windows\ImmersiveControlPanel\SystemSettings.exe",
            r"C:\Windows",
            "ApplicationFrameWindow",
        ));
        assert!(!is_native_windows_target(
            r"C:\Program Files\WindowsApps\Contoso.App\App.exe",
            r"C:\Windows",
            "ApplicationFrameWindow",
        ));
    }

    #[test]
    fn recognizes_uwp_application_frame_host_windows() {
        assert!(is_native_windows_target(
            r"C:\WINDOWS\SYSTEM32\ApplicationFrameHost.exe",
            r"C:\Windows",
            "ApplicationFrameWindow",
        ));
        assert!(!is_native_windows_target(
            r"C:\Other\ApplicationFrameHost.exe",
            r"C:\Windows",
            "ApplicationFrameWindow",
        ));
        assert!(!is_native_windows_target(
            r"C:\Windows\System32\ApplicationFrameHost.exe",
            r"C:\Windows",
            "OtherWindow",
        ));
    }

    #[test]
    fn recognizes_taskbar_windows_without_classifying_file_explorer() {
        assert!(is_native_windows_target(
            r"C:\Windows\explorer.exe",
            r"C:\Windows",
            "Shell_TrayWnd",
        ));
        assert!(!is_native_windows_target(
            r"C:\Windows\explorer.exe",
            r"C:\Windows",
            "CabinetWClass",
        ));
    }

    #[test]
    fn native_dpad_suppresses_synthetic_repeats_until_release() {
        let mut routing = NativeDpadRouting::default();
        routing.set_target_active(true);

        assert!(!routing.should_synthesize());
        assert!(!routing.should_synthesize());

        routing.set_target_active(false);
        assert!(!routing.should_synthesize());
        routing.release_dpad();
        assert!(routing.should_synthesize());
    }

    #[test]
    fn native_navigation_suppression_recovers_after_reentering_the_system_ui() {
        let mut routing = NativeDpadRouting::default();
        routing.set_target_active(true);
        assert!(!routing.should_synthesize());
        routing.release_dpad();

        routing.set_target_active(false);
        routing.set_target_active(true);
        assert!(!routing.should_synthesize());
    }
}

#[cfg(test)]
mod touch_pointer_tests {
    use super::*;
    use crate::control::DpadDirection;

    #[test]
    fn keyboard_dpad_moves_one_key_and_stays_inside_the_keyboard() {
        let bounds = KeyboardBounds::new(-1000, 400, 0, 700).unwrap();
        let center = bounds.center();

        assert_eq!(
            bounds.move_cursor_by_key(center.0, center.1, DpadDirection::Right),
            (-445, 550)
        );
        assert_eq!(
            bounds.move_cursor_by_key(center.0, center.1, DpadDirection::Down),
            (-500, 610)
        );
        assert_eq!(
            bounds.move_cursor_by_key(-995, 695, DpadDirection::Down),
            (-995, 699)
        );
    }

    #[test]
    fn keyboard_pointer_stays_inside_negative_monitor_bounds() {
        let bounds = KeyboardBounds::new(-1600, 600, 0, 1000).unwrap();
        assert_eq!(bounds.center(), (-800, 800));
        assert_eq!(bounds.constrain(-800, 800, i32::MAX, i32::MIN), (-1, 600));
        assert_eq!(bounds.constrain(-800, 800, -900, 900), (-1600, 999));
        assert!(bounds.contains(-1600, 600));
        assert!(!bounds.contains(0, 800));
        assert!(!bounds.contains(-800, 1000));
        assert!(KeyboardBounds::new(0, 0, 0, 100).is_none());
    }

    #[test]
    fn keyboard_click_target_must_be_a_windows_keyboard_host() {
        assert!(is_touch_keyboard_process(
            r"C:\Windows\SystemApps\MicrosoftWindows.Client.CBS_cw5n1h2txyewy\TextInputHost.exe",
            r"C:\Windows"
        ));
        assert!(is_touch_keyboard_process(
            r"C:\Program Files\Common Files\microsoft shared\ink\TabTip.exe",
            r"C:\Program Files\Common Files"
        ));
        assert!(!is_touch_keyboard_process(
            r"C:\Windows\explorer.exe",
            r"C:\Windows"
        ));
        assert!(!is_touch_keyboard_process(
            r"C:\Downloads\TextInputHost.exe",
            r"C:\Windows"
        ));
    }
}
