use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use sdl3::gamepad::GamepadType;

use crate::{
    control::{Action, ControlMode, ControlSettings, GamepadFrame, MouseController},
    cursor_scale::{CursorScale, CursorScaleRange},
    gamepads::{GamepadInfo, Gamepads},
    i18n::Locale,
    keyboard::{KeyboardSession, KeyboardSettings},
    notifications::{Notification, NotificationService},
    onboarding::{Command as TourCommand, Onboarding},
    platform::Platform,
    preferences::{DesktopPreferences, UiScale},
};

const ACTIVE_POLL_INTERVAL: Duration = Duration::from_millis(8);
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(16);

#[cfg(test)]
#[path = "onboarding_runtime_tests.rs"]
mod onboarding_tests;

#[derive(Clone, Default)]
pub struct RuntimeState {
    pub locale: Locale,
    pub ui_scale: UiScale,
    pub cursor_scale: CursorScale,
    pub cursor_scale_range: CursorScaleRange,
    pub cursor_scale_error: Option<String>,
    pub settings: ControlSettings,
    pub start_on_login: bool,
    pub controller: Option<String>,
    pub controller_type: Option<GamepadType>,
    pub error: Option<String>,
    pub ready: bool,
    pub devices: Vec<GamepadInfo>,
    pub selected: Option<u32>,
    pub mapping_message: Option<String>,
    pub keyboard: KeyboardSession,
    pub preferences_error: Option<String>,
    pub onboarding_completed: bool,
    pub onboarding: Option<Onboarding>,
    pending_mapping: Option<String>,
}

pub struct DesktopRuntime {
    state: Arc<Mutex<RuntimeState>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    notifications: NotificationService,
}

impl RuntimeState {
    fn desktop_input_enabled(&self) -> bool {
        self.onboarding.is_none()
            && self.settings.enabled
            && self.settings.mode == ControlMode::Desktop
    }

    fn native_mouse_state(&self, connected: bool) -> (bool, bool) {
        (
            self.desktop_input_enabled(),
            connected && self.onboarding.is_none(),
        )
    }

    fn finish_onboarding(&mut self, mode: Option<ControlMode>) {
        self.onboarding = None;
        self.onboarding_completed = true;
        if let Some(mode) = mode {
            self.settings.mode = mode;
            self.settings.enabled = true;
        }
    }
}

impl Default for DesktopRuntime {
    fn default() -> Self {
        let preferences = DesktopPreferences::load();
        let startup_error = crate::startup::set_enabled(preferences.start_on_login).err();
        let initial = RuntimeState {
            locale: preferences.locale,
            ui_scale: preferences.ui_scale,
            cursor_scale: preferences.cursor_scale,
            settings: preferences.controls,
            start_on_login: preferences.start_on_login,
            onboarding_completed: preferences.onboarding_completed,
            onboarding: (!preferences.onboarding_completed).then(Onboarding::default),
            preferences_error: startup_error,
            keyboard: KeyboardSession::with_locale(preferences.locale, KeyboardSettings::load()),
            ..RuntimeState::default()
        };
        let state = Arc::new(Mutex::new(initial));
        let stop = Arc::new(AtomicBool::new(false));
        let notifications = NotificationService::new();
        let worker_state = Arc::clone(&state);
        let worker_stop = Arc::clone(&stop);
        let worker_notifications = notifications.clone();
        let worker = thread::spawn(move || {
            if let Err(error) = run(&worker_state, &worker_stop, &worker_notifications) {
                let locale = {
                    let mut state = worker_state.lock().expect("runtime state lock");
                    state.error = Some(error.clone());
                    state.ready = false;
                    state.keyboard.open = false;
                    state.locale
                };
                worker_notifications.send(Notification::RuntimeFailure {
                    locale,
                    detail: error,
                });
            }
        });
        Self {
            state,
            stop,
            worker: Some(worker),
            notifications,
        }
    }
}

impl DesktopRuntime {
    pub fn reopen_onboarding(&self) {
        self.state.lock().expect("runtime state lock").onboarding = Some(Onboarding::default());
    }

    pub fn onboarding_command(&self, command: TourCommand) {
        let mut state = self.state.lock().expect("runtime state lock");
        match command {
            TourCommand::Scale(scale) => state.ui_scale = scale,
            TourCommand::Language(locale) => {
                state.locale = locale;
                state.keyboard.locale = locale;
            }
            TourCommand::Skip => state.finish_onboarding(None),
            TourCommand::Finish(mode) => state.finish_onboarding(Some(mode)),
            _ => {
                if let Some(tour) = &mut state.onboarding {
                    match command {
                        TourCommand::Next => tour.next(),
                        TourCommand::Back => tour.back(),
                        TourCommand::ToggleDemo => tour.toggle_demo(),
                        _ => {}
                    }
                }
            }
        }
        // A failed platform connection stops the worker; still remember a skipped/completed tour.
        let save_now = state.error.is_some()
            && (state.onboarding.is_none() || matches!(command, TourCommand::Scale(_)));
        drop(state);
        if save_now && let Err(error) = self.save_preferences() {
            self.state
                .lock()
                .expect("runtime state lock")
                .preferences_error = Some(error);
        }
    }

    pub fn update_onboarding_demo(&self, elapsed: Duration) {
        if let Some(tour) = &mut self.state.lock().expect("runtime state lock").onboarding {
            tour.update_demo(elapsed);
        }
    }

    pub fn state(&self) -> RuntimeState {
        self.state.lock().expect("runtime state lock").clone()
    }

    pub fn configure(&self, settings: ControlSettings) {
        let mut state = self.state.lock().expect("runtime state lock");
        let enabled_changed = state.settings.enabled != settings.enabled;
        let mode_before = state.settings.mode;
        let locale = state.locale;
        state.settings = settings;
        drop(state);
        if enabled_changed {
            self.notifications.send(Notification::ControlState {
                enabled: settings.enabled,
                locale,
            });
        }
        if let Some(notice) = Notification::mode_change(mode_before, settings.mode, locale) {
            self.notifications.send(notice);
        }
    }

    pub fn configure_cursor_scale(&self, percent: u16) {
        self.state.lock().expect("runtime state lock").cursor_scale = CursorScale::new(percent);
    }

    pub fn configure_keyboard(&self, settings: KeyboardSettings) {
        self.state
            .lock()
            .expect("runtime state lock")
            .keyboard
            .settings = settings.normalized();
    }

    pub fn configure_locale(&self, locale: Locale) {
        let mut state = self.state.lock().expect("runtime state lock");
        state.locale = locale;
        state.keyboard.locale = locale;
    }

    pub fn save_preferences(&self) -> Result<(), String> {
        let state = self.state.lock().expect("runtime state lock");
        DesktopPreferences {
            locale: state.locale,
            ui_scale: state.ui_scale,
            cursor_scale: state.cursor_scale,
            controls: state.settings,
            start_on_login: state.start_on_login,
            onboarding_completed: state.onboarding_completed,
        }
        .save()
    }

    pub fn configure_start_on_login(&self, enabled: bool) -> Result<(), String> {
        let result = crate::startup::set_enabled(enabled);
        let mut state = self.state.lock().expect("runtime state lock");
        match result {
            Ok(()) => {
                state.start_on_login = enabled;
                state.preferences_error = None;
                Ok(())
            }
            Err(error) => {
                state.preferences_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn select_controller(&self, selected: Option<u32>) {
        self.state.lock().expect("runtime state lock").selected = selected;
    }

    pub fn import_mapping(&self, mapping: String) {
        self.state
            .lock()
            .expect("runtime state lock")
            .pending_mapping = Some(mapping);
    }
}

impl Drop for DesktopRuntime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = self.save_preferences();
    }
}

fn run(
    state: &Mutex<RuntimeState>,
    stop: &AtomicBool,
    notification_service: &NotificationService,
) -> Result<(), String> {
    let mut input_tracker = None;
    let mut platform = Platform::new()?;
    state.lock().expect("runtime state lock").cursor_scale_range = platform.cursor_scale_range();
    let mut applied_cursor_scale = None;
    let mut gamepads = Gamepads::new().map_err(|error| format!("SDL3: {error}"))?;
    state.lock().expect("runtime state lock").mapping_message = gamepads.warning.take();
    let mut controller = MouseController::default();
    controller.keyboard.settings = state.lock().expect("runtime state lock").keyboard.settings;
    controller.keyboard.locale = state.lock().expect("runtime state lock").locale;
    let mut saved_settings = controller.keyboard.settings;
    controller.set_system_keyboard_uses_gamepad(platform.system_keyboard_uses_gamepad());
    controller.keyboard.presentation = platform.keyboard_presentation();
    controller.keyboard.status = platform.keyboard_overlay_error().map(str::to_owned);
    state.lock().expect("runtime state lock").keyboard = controller.keyboard.clone();
    let mut saved_preferences = DesktopPreferences::load();
    let mut pending_preferences: Option<(DesktopPreferences, Instant)> = None;
    let mut preference_save: Option<(DesktopPreferences, JoinHandle<Result<(), String>>)> = None;
    let mut last_tick = Instant::now();
    let outcome = (|| {
        while !stop.load(Ordering::Relaxed) {
            let now = Instant::now();
            if let Some((preferences, worker)) = preference_save.take() {
                if worker.is_finished() {
                    match worker.join() {
                        Ok(Ok(())) => {
                            saved_preferences = preferences;
                            state.lock().expect("runtime state lock").preferences_error = None;
                        }
                        Ok(Err(error)) => {
                            state.lock().expect("runtime state lock").preferences_error =
                                Some(error);
                            if pending_preferences.is_none() {
                                pending_preferences = Some((preferences, now));
                            }
                        }
                        Err(_) => {
                            state.lock().expect("runtime state lock").preferences_error =
                                Some("Preference save worker stopped unexpectedly".to_owned());
                            if pending_preferences.is_none() {
                                pending_preferences = Some((preferences, now));
                            }
                        }
                    }
                } else {
                    preference_save = Some((preferences, worker));
                }
            }
            let (selected, mapping) = {
                let mut state = state.lock().expect("runtime state lock");
                (state.selected, state.pending_mapping.take())
            };
            let importing = mapping.is_some();
            if let Some(mapping) = mapping {
                let message = match gamepads.import_mapping(&mapping) {
                    Ok(()) => "Perfil aplicado e salvo para as próximas sessões.".to_owned(),
                    Err(error) => error,
                };
                state.lock().expect("runtime state lock").mapping_message = Some(message);
            }
            let frame = if importing {
                None
            } else {
                gamepads.poll(selected)?
            };
            let (actions, notifications) = {
                sync_input_tracker(
                    &mut input_tracker,
                    controller.keyboard.open
                        && controller.keyboard.presentation
                            != crate::keyboard::KeyboardPresentation::System,
                    crate::focused_input::InputTracker::default,
                );
                controller.keyboard.focused_input = input_tracker
                    .as_ref()
                    .and_then(crate::focused_input::InputTracker::latest);
                let mut state = state.lock().expect("runtime state lock");
                let locale = state.locale;
                let enabled_before = state.settings.enabled;
                let mode_before = state.settings.mode;
                let mut notifications = update_gamepad_state(
                    &mut state,
                    frame.is_some(),
                    &gamepads.devices,
                    || gamepads.name(),
                    enabled_before,
                    locale,
                );
                state.controller_type = frame.as_ref().and_then(|_| gamepads.active_type());
                let actions = update_control(
                    &mut state,
                    &mut controller,
                    frame.as_ref(),
                    now.duration_since(last_tick),
                );
                if state.settings.enabled != enabled_before {
                    notifications.push(Notification::ControlState {
                        enabled: state.settings.enabled,
                        locale,
                    });
                }
                if let Some(notice) =
                    Notification::mode_change(mode_before, state.settings.mode, locale)
                {
                    notifications.push(notice);
                }
                (actions, notifications)
            };
            for notification in notifications {
                notification_service.send(notification);
            }
            last_tick = now;
            let cursor_scale = state.lock().expect("runtime state lock").cursor_scale;
            if applied_cursor_scale != Some(cursor_scale) {
                applied_cursor_scale = Some(cursor_scale);
                let error = platform.set_cursor_scale(cursor_scale).err();
                state.lock().expect("runtime state lock").cursor_scale_error = error;
            }
            let (input_enabled, native_connected) = state
                .lock()
                .expect("runtime state lock")
                .native_mouse_state(frame.is_some());
            platform.update_mouse_mode(input_enabled, native_connected)?;
            platform.update_native_dpad_target();
            for action in actions {
                match action {
                    Action::OnScreenKeyboard(open) => {
                        if open
                            && controller.keyboard.presentation
                                == crate::keyboard::KeyboardPresentation::Unavailable
                        {
                            controller.keyboard.open = false;
                            controller.keyboard.status = platform
                                .keyboard_overlay_error()
                                .map(str::to_owned)
                                .or_else(|| Some("Este compositor não permite exibir o teclado próprio sem ativá-lo.".to_owned()));
                        } else {
                            controller.keyboard.status = Some(if open {
                                if controller.keyboard.presentation == crate::keyboard::KeyboardPresentation::System {
                                    "Teclado do sistema aberto · R3 fecha."
                                } else {
                                    "Teclado visual próprio · direcional move · A digita · B apaga · R3 fecha."
                                }
                            } else {
                                "Teclado visual fechado."
                            }.to_owned());
                        }
                        let mut runtime_state = state.lock().expect("runtime state lock");
                        runtime_state.keyboard = controller.keyboard.clone();
                        let locale = runtime_state.locale;
                        let detail = controller.keyboard.status.clone();
                        drop(runtime_state);
                        if open
                            && controller.keyboard.presentation
                                == crate::keyboard::KeyboardPresentation::Unavailable
                        {
                            notification_service
                                .send(Notification::KeyboardUnavailable { locale, detail });
                        }
                    }
                    action => {
                        platform.apply(action)?;
                    }
                }
            }
            let preferences = {
                let state = state.lock().expect("runtime state lock");
                DesktopPreferences {
                    locale: state.locale,
                    ui_scale: state.ui_scale,
                    cursor_scale: state.cursor_scale,
                    controls: state.settings,
                    start_on_login: state.start_on_login,
                    onboarding_completed: state.onboarding_completed,
                }
            };
            if preferences == saved_preferences {
                pending_preferences = None;
            } else if pending_preferences
                .map(|(pending, _)| pending != preferences)
                .unwrap_or(true)
            {
                pending_preferences = Some((preferences, now));
            }
            if preference_save.is_none()
                && let Some((pending, changed_at)) = pending_preferences
                && now.duration_since(changed_at) >= Duration::from_millis(750)
            {
                preference_save = Some((pending, thread::spawn(move || pending.save())));
                pending_preferences = None;
            }
            let keyboard_was_open = controller.keyboard.open;
            let presentation_before = controller.keyboard.presentation;
            if let Err(error) = platform.update_keyboard_overlay(&mut controller.keyboard) {
                let was_open = controller.keyboard.open;
                controller.keyboard.open = false;
                if controller.keyboard.presentation != crate::keyboard::KeyboardPresentation::System
                {
                    controller.keyboard.presentation =
                        crate::keyboard::KeyboardPresentation::Unavailable;
                }
                controller.keyboard.status = Some(error);
                let mut runtime_state = state.lock().expect("runtime state lock");
                runtime_state.keyboard = controller.keyboard.clone();
                let locale = runtime_state.locale;
                drop(runtime_state);
                if was_open {
                    notification_service.send(Notification::KeyboardUnavailable {
                        locale,
                        detail: controller.keyboard.status.clone(),
                    });
                }
            }
            if controller.keyboard.open != keyboard_was_open
                || controller.keyboard.presentation != presentation_before
            {
                if !controller.keyboard.open {
                    controller.require_neutral();
                }
                state.lock().expect("runtime state lock").keyboard = controller.keyboard.clone();
            }
            if !controller.keyboard.customizing && controller.keyboard.settings != saved_settings {
                saved_settings = controller.keyboard.settings;
                if let Err(error) = saved_settings.save() {
                    controller.keyboard.status =
                        Some(format!("Não foi possível salvar ajustes: {error}"));
                }
            }
            thread::sleep(poll_interval(frame.is_some()));
        }
        Ok(())
    })();
    for action in controller.update(None, Duration::ZERO, &mut ControlSettings::default()) {
        if !matches!(action, Action::OnScreenKeyboard(_)) {
            let _ = platform.apply(action);
        }
    }
    if let Some((_, worker)) = preference_save {
        let _ = worker.join();
    }
    let _ = platform.update_keyboard_overlay(&mut controller.keyboard);
    if controller.keyboard.settings != saved_settings {
        let _ = controller.keyboard.settings.save();
    }
    outcome
}

fn sync_input_tracker<T>(tracker: &mut Option<T>, keyboard_open: bool, create: impl FnOnce() -> T) {
    if keyboard_open {
        if tracker.is_none() {
            *tracker = Some(create());
        }
    } else {
        *tracker = None;
    }
}

fn update_gamepad_state(
    state: &mut RuntimeState,
    connected: bool,
    devices: &[GamepadInfo],
    name: impl FnOnce() -> Option<String>,
    control_enabled: bool,
    locale: Locale,
) -> Vec<Notification> {
    let was_connected = state.controller.is_some();
    let had_no_profile = no_usable_profile(&state.devices);
    if connected {
        if state.controller.is_none() {
            state.controller = name();
        }
    } else {
        state.controller = None;
    }
    if !same_devices(&state.devices, devices) {
        state.devices.clear();
        state.devices.extend_from_slice(devices);
    }
    state.ready = true;
    let is_connected = state.controller.is_some();
    let has_no_profile = no_usable_profile(&state.devices);
    let mut notifications = Vec::new();
    if control_enabled && was_connected != is_connected {
        if let Some(name) = state.controller.as_ref() {
            notifications.push(Notification::ControllerConnected {
                locale,
                name: name.clone(),
            });
        } else {
            notifications.push(Notification::ControllerDisconnected(locale));
        }
    }
    if has_no_profile && !had_no_profile {
        notifications.push(Notification::NoUsableProfile(locale));
    }
    notifications
}

fn no_usable_profile(devices: &[GamepadInfo]) -> bool {
    !devices.is_empty() && devices.iter().all(|device| !device.mapped)
}

fn same_devices(left: &[GamepadInfo], right: &[GamepadInfo]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.id == right.id
                && left.name == right.name
                && left.guid == right.guid
                && left.mapped == right.mapped
        })
}

fn poll_interval(connected: bool) -> Duration {
    if connected {
        ACTIVE_POLL_INTERVAL
    } else {
        IDLE_POLL_INTERVAL
    }
}

fn update_control(
    state: &mut RuntimeState,
    controller: &mut MouseController,
    frame: Option<&GamepadFrame>,
    elapsed: Duration,
) -> Vec<Action> {
    controller.keyboard.settings = state.keyboard.settings;
    controller.keyboard.locale = state.locale;
    if let Some(tour) = &mut state.onboarding {
        let finish = tour.update(frame, elapsed);
        let actions = controller.update(None, elapsed, &mut state.settings);
        publish_keyboard_state(&mut state.keyboard, &controller.keyboard);
        if let Some(mode) = finish {
            state.finish_onboarding(Some(mode));
        }
        return actions;
    }
    let actions = controller.update(frame, elapsed, &mut state.settings);
    publish_keyboard_state(&mut state.keyboard, &controller.keyboard);
    actions
}

fn publish_keyboard_state(state: &mut KeyboardSession, keyboard: &KeyboardSession) {
    if *state != *keyboard {
        state.clone_from(keyboard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::GamepadFrame;

    #[test]
    fn focused_input_tracker_is_lazy_and_scoped_to_the_open_keyboard() {
        let mut tracker = None;
        let mut creations = 0;

        sync_input_tracker(&mut tracker, false, || {
            creations += 1;
            creations
        });
        assert_eq!(tracker, None);
        assert_eq!(creations, 0);

        sync_input_tracker(&mut tracker, true, || {
            creations += 1;
            creations
        });
        assert_eq!(tracker, Some(1));

        sync_input_tracker(&mut tracker, true, || {
            creations += 1;
            creations
        });
        assert_eq!(tracker, Some(1));
        assert_eq!(creations, 1);

        sync_input_tracker(&mut tracker, false, || {
            creations += 1;
            creations
        });
        assert_eq!(tracker, None);
        assert_eq!(creations, 1);
    }

    #[test]
    fn r3_requests_the_system_keyboard_without_opening_the_app_panel() {
        let mut state = RuntimeState::default();
        let mut controller = MouseController::default();
        let neutral = GamepadFrame::default();
        update_control(&mut state, &mut controller, Some(&neutral), Duration::ZERO);
        let mut frame = neutral;
        frame.buttons[crate::control::Button::RightStick as usize] = true;

        let actions = update_control(&mut state, &mut controller, Some(&frame), Duration::ZERO);

        assert_eq!(actions, vec![Action::OnScreenKeyboard(true)]);
        assert!(state.keyboard.open);
        assert!(controller.keyboard.open);
    }

    #[test]
    fn runtime_keeps_the_active_poll_rate_and_backs_off_when_disconnected() {
        assert_eq!(poll_interval(true), Duration::from_millis(8));
        assert_eq!(poll_interval(false), Duration::from_millis(16));
    }

    #[test]
    fn unchanged_gamepad_snapshot_does_not_recreate_the_name_or_device_list() {
        let devices = vec![GamepadInfo {
            id: 7,
            name: "Virtual controller".to_owned(),
            guid: "0123456789abcdef0123456789abcdef".to_owned(),
            mapped: true,
        }];
        let mut state = RuntimeState::default();
        let mut name_reads = 0;

        update_gamepad_state(
            &mut state,
            true,
            &devices,
            || {
                name_reads += 1;
                Some("Virtual controller".to_owned())
            },
            true,
            Locale::English,
        );
        update_gamepad_state(
            &mut state,
            true,
            &devices,
            || {
                name_reads += 1;
                Some("Virtual controller".to_owned())
            },
            true,
            Locale::English,
        );

        assert_eq!(name_reads, 1);
        assert_eq!(state.controller.as_deref(), Some("Virtual controller"));
        assert!(same_devices(&state.devices, &devices));
        assert!(state.ready);
    }

    #[test]
    fn gamepad_notifications_follow_connection_and_profile_transitions_once() {
        use crate::notifications::Notification;

        let mapped = vec![GamepadInfo {
            id: 7,
            name: "Mapped controller".to_owned(),
            guid: "0123456789abcdef0123456789abcdef".to_owned(),
            mapped: true,
        }];
        let unknown = vec![GamepadInfo {
            mapped: false,
            ..mapped[0].clone()
        }];
        let mut state = RuntimeState::default();

        assert_eq!(
            update_gamepad_state(
                &mut state,
                true,
                &mapped,
                || Some("Mapped controller".to_owned()),
                true,
                Locale::English,
            ),
            vec![Notification::ControllerConnected {
                locale: Locale::English,
                name: "Mapped controller".to_owned(),
            }]
        );
        assert!(
            update_gamepad_state(
                &mut state,
                true,
                &mapped,
                || Some("ignored repeated read".to_owned()),
                true,
                Locale::English,
            )
            .is_empty()
        );
        assert_eq!(
            update_gamepad_state(&mut state, false, &[], || None, true, Locale::English,),
            vec![Notification::ControllerDisconnected(Locale::English)]
        );
        assert_eq!(
            update_gamepad_state(&mut state, false, &unknown, || None, false, Locale::English,),
            vec![Notification::NoUsableProfile(Locale::English)]
        );
        assert!(
            update_gamepad_state(&mut state, false, &unknown, || None, false, Locale::English,)
                .is_empty()
        );
    }

    #[test]
    #[ignore = "manual benchmark comparing unchanged runtime-state copies"]
    fn runtime_state_copy_benchmark() {
        use std::hint::black_box;

        const ITERATIONS: usize = 200_000;
        let mut keyboard = KeyboardSession::default();
        keyboard.status = Some("Keyboard ready".to_owned());
        for device_count in [1, 4] {
            let devices: Vec<_> = (0..device_count)
                .map(|id| GamepadInfo {
                    id,
                    name: format!("Virtual controller {id}"),
                    guid: format!("{id:032x}"),
                    mapped: true,
                })
                .collect();
            let mut initial = RuntimeState {
                controller: Some("Virtual controller 0".to_owned()),
                devices: devices.clone(),
                keyboard: keyboard.clone(),
                ..RuntimeState::default()
            };
            let name = "Virtual controller 0";

            let baseline_started = Instant::now();
            for _ in 0..ITERATIONS {
                initial.controller = Some(name.to_owned());
                initial.devices = devices.clone();
                initial.keyboard = keyboard.clone();
                black_box(&initial);
            }
            let baseline_ns = baseline_started.elapsed().as_nanos() as f64 / ITERATIONS as f64;

            let mut optimized = RuntimeState {
                controller: Some(name.to_owned()),
                devices: devices.clone(),
                keyboard: keyboard.clone(),
                ..RuntimeState::default()
            };
            let optimized_started = Instant::now();
            for _ in 0..ITERATIONS {
                update_gamepad_state(
                    &mut optimized,
                    true,
                    &devices,
                    || Some(name.to_owned()),
                    true,
                    Locale::English,
                );
                publish_keyboard_state(&mut optimized.keyboard, &keyboard);
                black_box(&optimized);
            }
            let optimized_ns = optimized_started.elapsed().as_nanos() as f64 / ITERATIONS as f64;

            let baseline_ui_started = Instant::now();
            for _ in 0..ITERATIONS {
                let first = initial.clone();
                black_box((first.locale, first.keyboard));
                black_box(initial.clone());
            }
            let baseline_ui_ns =
                baseline_ui_started.elapsed().as_nanos() as f64 / ITERATIONS as f64;

            let optimized_ui_started = Instant::now();
            for _ in 0..ITERATIONS {
                let snapshot = optimized.clone();
                black_box((snapshot.locale, snapshot.keyboard.clone()));
                black_box(snapshot);
            }
            let optimized_ui_ns =
                optimized_ui_started.elapsed().as_nanos() as f64 / ITERATIONS as f64;

            eprintln!(
                "devices={device_count} tick_ns baseline={baseline_ns:.1} optimized={optimized_ns:.1} ui_ns baseline={baseline_ui_ns:.1} optimized={optimized_ui_ns:.1}"
            );
        }
    }
}
