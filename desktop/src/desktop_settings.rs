use eframe::egui::{self, Color32};
use gamepad_desktop::{
    control::ControlMode,
    cursor_scale::CursorScale,
    i18n::{Locale, Text},
    keyboard::{KeyboardPosition, KeyboardPresentation},
    runtime::RuntimeState,
    tray::{PanelControl, TrayMenu},
};

use super::{DANGER, SharedDesktop, accent_heading, interface_scale};

pub(super) fn show(
    ui: &mut egui::Ui,
    state: &mut RuntimeState,
    desktop: &SharedDesktop,
    mapping_text: &mut String,
    tray_error: Option<&str>,
    panel: &PanelControl,
    tray_menu: Option<&TrayMenu>,
) {
    accent_heading(ui, state.locale.text(Text::Settings));
    show_general_settings(ui, state, desktop, tray_menu);
    show_cursor_scale(ui, state, desktop);
    show_control_settings(ui, state, desktop);
    show_keyboard_settings(ui, state, desktop);
    show_panel_controls(ui, state.locale, tray_error, panel);
    show_controller_profiles(ui, state, desktop, mapping_text);
}

fn show_general_settings(
    ui: &mut egui::Ui,
    state: &mut RuntimeState,
    desktop: &SharedDesktop,
    tray_menu: Option<&TrayMenu>,
) {
    if let Some(scale) = interface_scale(ui, state.locale, state.ui_scale) {
        desktop
            .lock()
            .expect("desktop runtime lock")
            .onboarding_command(gamepad_desktop::onboarding::Command::Scale(scale));
        state.ui_scale = scale;
    }

    let mut locale = state.locale;
    ui.horizontal(|ui| {
        ui.label(state.locale.text(Text::Language));
        egui::ComboBox::from_id_salt("desktop-language")
            .selected_text(locale.name())
            .show_ui(ui, |ui| {
                for option in Locale::ALL {
                    ui.selectable_value(&mut locale, option, option.name());
                }
            });
    });
    if locale != state.locale {
        desktop
            .lock()
            .expect("desktop runtime lock")
            .configure_locale(locale);
        if let Some(tray_menu) = tray_menu {
            tray_menu.set_locale(locale);
        }
        state.locale = locale;
        state.keyboard.locale = locale;
    }
    if let Some(error) = &state.preferences_error {
        ui.small(format!(
            "{}{error}",
            state.locale.text(Text::PreferencesSaveFailedPrefix)
        ));
    }

    let locale = state.locale;
    let mut start_on_login = state.start_on_login;
    if ui
        .checkbox(&mut start_on_login, locale.text(Text::StartWithOS))
        .changed()
    {
        let result = desktop
            .lock()
            .expect("desktop runtime lock")
            .configure_start_on_login(start_on_login);
        if let Err(error) = result {
            state.preferences_error = Some(error);
        } else {
            state.start_on_login = start_on_login;
        }
    }
}

fn show_cursor_scale(ui: &mut egui::Ui, state: &mut RuntimeState, desktop: &SharedDesktop) {
    let locale = state.locale;
    if state.cursor_scale_range.supported {
        let max = state.cursor_scale_range.max_percent;
        let mut cursor_scale = state.cursor_scale.percent().clamp(100, max);
        if max > 100
            && ui
                .add(
                    egui::Slider::new(&mut cursor_scale, 100..=max)
                        .text(locale.text(Text::CursorScale))
                        .suffix("%")
                        .step_by(5.0),
                )
                .changed()
        {
            desktop
                .lock()
                .expect("desktop runtime lock")
                .configure_cursor_scale(cursor_scale);
            state.cursor_scale = CursorScale::new(cursor_scale);
        }
        if max == 100 {
            ui.label(format!("{} · 100%", locale.text(Text::CursorScale)));
        }
        ui.small(
            locale
                .text(Text::CursorScaleHelp)
                .replace("{max}", &max.to_string()),
        );
        if let Some(error) = &state.cursor_scale_error {
            ui.small(format!(
                "{}{error}",
                locale.text(Text::CursorScaleErrorPrefix)
            ));
        }
    } else {
        ui.small(locale.text(Text::CursorScaleUnavailable));
    }
}

fn show_control_settings(ui: &mut egui::Ui, state: &mut RuntimeState, desktop: &SharedDesktop) {
    let locale = state.locale;
    egui::CollapsingHeader::new(locale.text(Text::ControlsAndAdjustments))
        .default_open(true)
        .show(ui, |ui| {
            let mut selected = state.selected;
            let selected_name = if selected.is_none() {
                locale.text(Text::Automatic)
            } else {
                state
                    .devices
                    .iter()
                    .find(|device| Some(device.id) == selected)
                    .map_or(locale.text(Text::DisconnectedController), |device| {
                        device.name.as_str()
                    })
            };
            egui::ComboBox::from_id_salt("desktop-controller")
                .selected_text(selected_name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, None, locale.text(Text::Automatic));
                    for device in &state.devices {
                        let label = format!(
                            "{}{}",
                            device.name,
                            if device.mapped {
                                ""
                            } else {
                                locale.text(Text::NeedsProfile)
                            }
                        );
                        ui.selectable_value(&mut selected, Some(device.id), label);
                    }
                });
            if selected != state.selected {
                desktop
                    .lock()
                    .expect("desktop runtime lock")
                    .select_controller(selected);
            }
            let mode = state.settings.mode;
            let mut changed = false;
            ui.add_enabled_ui(mode == ControlMode::Desktop, |ui| {
                changed |= ui
                    .checkbox(
                        &mut state.settings.enabled,
                        locale.text(Text::ControlDesktop),
                    )
                    .changed();
            });
            ui.add_enabled_ui(mode == ControlMode::Desktop, |ui| {
                changed |= ui
                    .add(
                        egui::Slider::new(&mut state.settings.speed, 100.0..=2400.0)
                            .text(locale.text(Text::Speed)),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::Slider::new(&mut state.settings.deadzone, 0.05..=0.5)
                            .text(locale.text(Text::Deadzone)),
                    )
                    .changed();
            });
            if changed {
                desktop
                    .lock()
                    .expect("desktop runtime lock")
                    .configure(state.settings);
            }
        });
}

fn show_keyboard_settings(ui: &mut egui::Ui, state: &RuntimeState, desktop: &SharedDesktop) {
    let locale = state.locale;
    egui::CollapsingHeader::new(locale.text(Text::CustomizeKeyboard)).show(ui, |ui| {
        if state.keyboard.presentation == KeyboardPresentation::System {
            ui.label(locale.text(Text::SystemKeyboardHelp));
            return;
        }
        let mut settings = state.keyboard.settings;
        let mut changed = ui
            .add(
                egui::Slider::new(&mut settings.width, 520..=1000)
                    .text(locale.text(Text::WidthPx))
                    .step_by(20.0),
            )
            .changed();
        ui.horizontal(|ui| {
            changed |= ui
                .selectable_value(
                    &mut settings.position,
                    KeyboardPosition::Top,
                    locale.text(Text::Top),
                )
                .changed();
            changed |= ui
                .selectable_value(
                    &mut settings.position,
                    KeyboardPosition::Bottom,
                    locale.text(Text::Bottom),
                )
                .changed();
        });
        changed |= ui
            .add(
                egui::Slider::new(&mut settings.opacity, 160..=255)
                    .text(locale.text(Text::Opacity)),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut settings.repeat_enabled,
                locale.text(Text::RepeatDirection),
            )
            .changed();
        ui.add_enabled_ui(settings.repeat_enabled, |ui| {
            changed |= ui
                .add(
                    egui::Slider::new(&mut settings.repeat_delay_ms, 400..=1200)
                        .text(locale.text(Text::InitialWait))
                        .step_by(50.0),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut settings.repeat_interval_ms, 150..=1000)
                        .text(locale.text(Text::RepeatInterval))
                        .step_by(50.0),
                )
                .changed();
        });
        if ui.button(locale.text(Text::RestoreKeyboard)).clicked() {
            settings = Default::default();
            changed = true;
        }
        if changed {
            desktop
                .lock()
                .expect("desktop runtime lock")
                .configure_keyboard(settings);
        }
        ui.small(locale.text(Text::KeyboardFollowsField));
        ui.small(locale.text(Text::KeyboardSettingsHelp));
        ui.small(locale.text(Text::KeyboardBehaviorHelp));
    });
}

fn show_panel_controls(
    ui: &mut egui::Ui,
    locale: Locale,
    tray_error: Option<&str>,
    panel: &PanelControl,
) {
    if let Some(error) = tray_error {
        ui.colored_label(
            DANGER,
            format!(
                "{}{error}. {}",
                locale.text(Text::TrayUnavailablePrefix),
                locale.text(Text::ClosePanelKeepsRunning)
            ),
        );
    } else {
        ui.label(locale.text(Text::ClosePanelKeepsRunning));
    }
    let quit = egui::Button::new(locale.text(Text::QuitApplication))
        .fill(Color32::from_rgb(71, 37, 48))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(130, 62, 77)));
    if ui.add(quit).clicked() {
        panel.exit();
    }
}

fn show_controller_profiles(
    ui: &mut egui::Ui,
    state: &RuntimeState,
    desktop: &SharedDesktop,
    mapping_text: &mut String,
) {
    let locale = state.locale;
    egui::CollapsingHeader::new(locale.text(Text::ControllerProfiles)).show(ui, |ui| {
        ui.label(locale.text(Text::KnownProfiles));
        for device in &state.devices {
            ui.label(format!("{} · GUID: {}", device.name, device.guid));
        }
        ui.add(
            egui::TextEdit::multiline(mapping_text)
                .hint_text("GUID,nome,a:b0,leftx:a0,lefty:a1,...")
                .desired_rows(2),
        );
        if ui
            .add_enabled(
                !mapping_text.trim().is_empty(),
                egui::Button::new(locale.text(Text::ApplySaveProfile)),
            )
            .clicked()
        {
            desktop
                .lock()
                .expect("desktop runtime lock")
                .import_mapping(mapping_text.clone());
        }
        if let Some(message) = &state.mapping_message {
            ui.label(locale.translate(message));
        }
    });
}
