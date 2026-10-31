#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use eframe::egui::{self, Color32, RichText};
use gamepad_desktop::runtime::{DesktopRuntime, RuntimeState};
use gamepad_desktop::tray::{PanelControl, TrayAction, TrayMenu, create_tray, gamepad_icon_data};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use gamepad_desktop::updater::Updater;
use gamepad_desktop::{
    control::{ControlMode, KEYBOARD_SHORTCUTS, MOUSE_SHORTCUTS},
    i18n::{Locale, Text},
    keyboard::{self, KeyboardPosition, KeyboardPresentation, KeyboardSession},
    preferences::UiScale,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

mod desktop_settings;
mod gamepad_icons;
mod keyboard_view;
mod onboarding_view;
#[cfg(target_os = "windows")]
mod windows_keyboard_overlay;

type SharedDesktop = Arc<Mutex<DesktopRuntime>>;
const APP_NAME: &str = "GamepadDesktop";
const UI_REPAINT_INTERVAL: Duration = Duration::from_millis(16);
const ACCENT: Color32 = Color32::from_rgb(94, 234, 212);
const PAGE_BACKGROUND: Color32 = Color32::from_rgb(8, 12, 21);
const CARD_BACKGROUND: Color32 = Color32::from_rgb(15, 22, 36);
const WIDGET_BACKGROUND: Color32 = Color32::from_rgb(23, 34, 54);
const BORDER: Color32 = Color32::from_rgb(42, 59, 82);
const TEXT: Color32 = Color32::from_rgb(228, 235, 244);
const MUTED_TEXT: Color32 = Color32::from_rgb(160, 177, 197);
const DANGER: Color32 = Color32::from_rgb(255, 126, 140);
const WARNING: Color32 = Color32::from_rgb(255, 191, 105);
const KEYCAP_BORDER: Color32 = Color32::from_rgb(51, 103, 112);

fn configure_desktop_style(context: &egui::Context) {
    egui_extras::install_image_loaders(context);
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.weak_text_color = Some(MUTED_TEXT);
    visuals.panel_fill = PAGE_BACKGROUND;
    visuals.window_fill = CARD_BACKGROUND;
    visuals.faint_bg_color = Color32::from_rgb(19, 29, 46);
    visuals.extreme_bg_color = Color32::from_rgb(7, 10, 18);
    visuals.code_bg_color = Color32::from_rgb(19, 29, 46);
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = Color32::from_rgb(32, 94, 91);
    visuals.window_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    visuals.widgets.noninteractive.bg_fill = CARD_BACKGROUND;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke.color = TEXT;
    visuals.widgets.inactive.bg_fill = WIDGET_BACKGROUND;
    visuals.widgets.inactive.weak_bg_fill = WIDGET_BACKGROUND;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.fg_stroke.color = TEXT;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(27, 58, 69);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(27, 58, 69);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    visuals.widgets.active.bg_fill = Color32::from_rgb(32, 94, 91);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(32, 94, 91);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    context.set_visuals(visuals);
    context.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(12.0, 10.0);
        style.spacing.window_margin = egui::Margin::same(16);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.slider_width = 180.0;
    });
}

fn interface_scale(ui: &mut egui::Ui, locale: Locale, scale: UiScale) -> Option<UiScale> {
    let mut selected = scale;
    ui.horizontal_wrapped(|ui| {
        ui.label(locale.text(Text::InterfaceSize));
        if ui
            .add_enabled(scale != UiScale::Standard, egui::Button::new("A−"))
            .clicked()
        {
            selected = scale.step(false);
        }
        ui.label(format!("{}%", scale as u16));
        if ui
            .add_enabled(scale != UiScale::ExtraLarge, egui::Button::new("A+"))
            .clicked()
        {
            selected = scale.step(true);
        }
    });
    ui.small(locale.text(Text::InterfaceSizeHelp));
    (selected != scale).then_some(selected)
}

fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD_BACKGROUND)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(12)
        .inner_margin(16)
}

fn content_card<R>(
    ui: &mut egui::Ui,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        contents(ui)
    })
}

fn status_badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    let fill = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 36);
    let stroke = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 130);
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.0, color);
                ui.label(RichText::new(text).strong().color(color));
            });
        });
}

fn accent_heading(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        let (bar, _) = ui.allocate_exact_size(egui::vec2(3.0, 18.0), egui::Sense::hover());
        ui.painter().rect_filled(bar, 1.5, ACCENT);
        ui.label(RichText::new(text).strong().size(17.0).color(ACCENT));
    });
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
enum AppPage {
    #[default]
    Home,
    ControllerGuide,
    Settings,
}

impl AppPage {
    const ALL: [Self; 3] = [Self::Home, Self::ControllerGuide, Self::Settings];

    const fn label(self) -> Text {
        match self {
            Self::Home => Text::Home,
            Self::ControllerGuide => Text::CommandGuide,
            Self::Settings => Text::Settings,
        }
    }

    fn icon(self) -> egui::Image<'static> {
        let source = match self {
            Self::Home => egui::include_image!("../assets/icons/house.svg"),
            Self::ControllerGuide => egui::include_image!("../assets/icons/gamepad-2.svg"),
            Self::Settings => egui::include_image!("../assets/icons/settings.svg"),
        };
        egui::Image::new(source)
            .fit_to_exact_size(egui::vec2(18.0, 18.0))
            .tint(TEXT)
    }
}

struct GamepadDesktopApp {
    desktop: SharedDesktop,
    panel: Arc<PanelControl>,
    tray_error: Option<String>,
    tray_menu: Option<TrayMenu>,
    mapping_text: String,
    page: AppPage,
    shutdown_started: bool,
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    updater: Updater,
}

impl GamepadDesktopApp {
    fn new(
        desktop: SharedDesktop,
        panel: Arc<PanelControl>,
        tray_error: Option<String>,
        tray_menu: Option<TrayMenu>,
    ) -> Self {
        Self {
            desktop,
            panel,
            tray_error,
            tray_menu,
            mapping_text: String::new(),
            page: AppPage::default(),
            shutdown_started: false,
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            updater: Updater::default(),
        }
    }

    fn show_navigation(&mut self, ui: &mut egui::Ui, state: &RuntimeState) {
        let locale = state.locale;
        egui::Panel::top("desktop-navigation").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let menu_icon = egui::Image::new(egui::include_image!("../assets/icons/menu.svg"))
                    .fit_to_exact_size(egui::vec2(18.0, 18.0))
                    .tint(TEXT);
                ui.menu_image_text_button(menu_icon, locale.text(Text::Menu), |ui| {
                    ui.set_min_width(220.0);
                    for page in AppPage::ALL {
                        if ui
                            .add(
                                egui::Button::image_and_text(
                                    page.icon(),
                                    locale.text(page.label()),
                                )
                                .selected(self.page == page),
                            )
                            .clicked()
                        {
                            self.page = page;
                            ui.close();
                        }
                    }
                    ui.separator();
                    let quit_icon =
                        egui::Image::new(egui::include_image!("../assets/icons/door-open.svg"))
                            .fit_to_exact_size(egui::vec2(18.0, 18.0))
                            .tint(TEXT);
                    if ui
                        .add(egui::Button::image_and_text(
                            quit_icon,
                            locale.text(Text::QuitApplication),
                        ))
                        .clicked()
                    {
                        self.panel.exit();
                        ui.close();
                    }
                });
                ui.separator();
                ui.add(
                    egui::Image::new(egui::include_image!("../assets/gamepad-desktop.svg"))
                        .fit_to_exact_size(egui::vec2(32.0, 32.0)),
                );
                ui.label(RichText::new(APP_NAME).strong().size(18.0));
                ui.label(RichText::new(locale.text(self.page.label())).color(MUTED_TEXT));
                if state.ready && state.error.is_none() && state.controller.is_some() {
                    let (mode, color) = if state.keyboard.open {
                        (locale.text(Text::KeyboardVisual), ACCENT)
                    } else if state.settings.mode == ControlMode::Game {
                        (locale.text(Text::GameModeBadge), ACCENT)
                    } else if state.settings.enabled {
                        (locale.text(Text::DesktopMode), ACCENT)
                    } else {
                        (locale.text(Text::PausedLabel), WARNING)
                    };
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_badge(ui, mode, color);
                    });
                }
            });
        });
    }
}

impl eframe::App for GamepadDesktopApp {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = ui.ctx();
        context.request_repaint_after(UI_REPAINT_INTERVAL);
        let desktop = Arc::clone(&self.desktop);
        let shutdown_complete = desktop
            .lock()
            .expect("desktop runtime lock")
            .shutdown_complete();
        let close = handle_shutdown_close(
            context,
            self.panel.exiting() || self.tray_error.is_some(),
            &mut self.shutdown_started,
            shutdown_complete,
            || {
                desktop
                    .lock()
                    .expect("desktop runtime lock")
                    .begin_shutdown();
            },
        );
        if close == ShutdownClose::Waiting || close == ShutdownClose::Allowed {
            let locale = self
                .desktop
                .lock()
                .expect("desktop runtime lock")
                .state()
                .locale;
            show_shutdown_pending(ui, locale.text(Text::FinishingCleanup));
            return;
        }
        if close == ShutdownClose::Idle {
            handle_panel_close(context, &self.panel, self.tray_error.is_none());
        }
        self.desktop
            .lock()
            .expect("desktop runtime lock")
            .update_onboarding_demo(Duration::from_secs_f32(
                ui.input(|input| input.stable_dt).clamp(0.0, 0.05),
            ));
        let mut state = self.desktop.lock().expect("desktop runtime lock").state();
        ui.ctx().set_zoom_factor(state.ui_scale.factor());
        let locale = state.locale;
        if let Some(error) = &state.error {
            egui::Panel::top("runtime-error").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(DANGER, locale.translate(error));
                    if ui.button(locale.text(Text::Retry)).clicked() {
                        *self.desktop.lock().expect("desktop runtime lock") =
                            DesktopRuntime::default();
                    }
                });
            });
        }
        let mut keyboard = state.keyboard.clone();
        keyboard.open &= state.onboarding.is_none();
        if keyboard.presentation == KeyboardPresentation::Viewport {
            show_keyboard_viewport(ui, keyboard, cfg!(target_os = "windows"));
        }
        if state.onboarding.is_some() {
            if let Some(command) = onboarding_view::show(ui, &state) {
                self.desktop
                    .lock()
                    .expect("desktop runtime lock")
                    .onboarding_command(command);
            }
            return;
        }
        self.show_navigation(ui, &state);
        egui::Panel::bottom("desktop-footer").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                {
                    if self.updater.poll() {
                        self.panel.exit();
                    }
                    if ui
                        .add_enabled(
                            self.updater.enabled(),
                            egui::Button::new(locale.text(self.updater.button_text())),
                        )
                        .clicked()
                    {
                        self.updater.activate();
                    }
                    if let Some(message) = self.updater.message(locale) {
                        ui.small(message);
                    }
                    ui.separator();
                }
                ui.hyperlink_to(locale.text(Text::Donate), "https://ko-fi.com/andrecrjr");
                ui.separator();
                ui.hyperlink_to(locale.text(Text::Projects), "https://ac-jr.com");
                ui.separator();
                if ui.button(locale.text(Text::TourReplay)).clicked() {
                    self.desktop
                        .lock()
                        .expect("desktop runtime lock")
                        .reopen_onboarding();
                }
                ui.separator();
                show_app_version(ui);
            });
        });
        let page = self.page;
        let desktop = &self.desktop;
        let panel = &self.panel;
        let tray_error = self.tray_error.as_deref();
        let tray_menu = self.tray_menu.as_ref();
        let mapping_text = &mut self.mapping_text;
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt(page).show(ui, |ui| {
                let width = ui.available_width().min(760.0);
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.add_space(8.0);
                            match page {
                                AppPage::Home => {
                                    if show_home(ui, &mut state) {
                                        desktop
                                            .lock()
                                            .expect("desktop runtime lock")
                                            .configure(state.settings);
                                    }
                                }
                                AppPage::ControllerGuide => {
                                    show_controller_guide(ui, &state);
                                }
                                AppPage::Settings => {
                                    show_desktop_settings(
                                        ui,
                                        &mut state,
                                        desktop,
                                        mapping_text,
                                        tray_error,
                                        panel,
                                        tray_menu,
                                    );
                                }
                            }
                        },
                    );
                });
            });
        });
    }
}

fn show_mode_guide(ui: &mut egui::Ui, state: &mut RuntimeState) -> bool {
    let locale = state.locale;
    ui.strong(locale.text(Text::ChooseMode));
    let previous = state.settings.mode;
    for (mode, label, description) in [
        (
            ControlMode::Desktop,
            Text::DesktopMode,
            Text::DesktopModeHelp,
        ),
        (ControlMode::Game, Text::GameMode, Text::GameModeHelp),
    ] {
        egui::Frame::new()
            .fill(WIDGET_BACKGROUND)
            .stroke(egui::Stroke::new(
                1.0,
                if state.settings.mode == mode {
                    ACCENT
                } else {
                    KEYCAP_BORDER
                },
            ))
            .corner_radius(8)
            .inner_margin(10)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.selectable_value(&mut state.settings.mode, mode, locale.text(label));
                ui.label(locale.text(description));
            });
        ui.add_space(5.0);
    }
    ui.add_space(8.0);
    ui.separator();
    ui.strong(locale.text(Text::QuickControls));
    shortcut_guide(ui, locale, &MOUSE_SHORTCUTS[12..13]);
    shortcut_guide(ui, locale, &MOUSE_SHORTCUTS[11..12]);
    if state.settings.mode == ControlMode::Desktop && !state.settings.enabled {
        ui.colored_label(WARNING, locale.text(Text::Paused));
    }
    ui.small(locale.text(Text::ResumeGuide));
    previous != state.settings.mode
}

fn show_desktop_overview(ui: &mut egui::Ui, state: &RuntimeState) {
    let locale = state.locale;
    accent_heading(ui, locale.text(Text::ConnectionStatus));
    if state.error.is_none() {
        if !state.ready {
            ui.label(locale.text(Text::Connecting));
        } else if let Some(name) = &state.controller {
            ui.colored_label(
                ACCENT,
                format!("{}{name}", locale.text(Text::ControllerPrefix)),
            );
        } else {
            ui.label(if state.devices.is_empty() {
                locale.text(Text::ConnectGamepad)
            } else {
                locale.text(Text::SelectController)
            });
            ui.small(locale.text(Text::DeviceSupport));
        }
    }
    ui.add_space(8.0);
    if state.keyboard.presentation != KeyboardPresentation::Unavailable
        && let Some(status) = &state.keyboard.status
    {
        ui.small(locale.translate(status));
    }
    if state.keyboard.open {
        ui.small(locale.text(Text::KeyboardFocusNote));
    }
    if state.keyboard.presentation == KeyboardPresentation::Unavailable
        && let Some(status) = &state.keyboard.status
    {
        ui.colored_label(DANGER, locale.translate(status));
    }
}

fn show_home(ui: &mut egui::Ui, state: &mut RuntimeState) -> bool {
    let locale = state.locale;
    accent_heading(ui, locale.text(Text::Home));
    ui.label(locale.text(Text::PanelIntro));
    ui.add_space(12.0);
    content_card(ui, |ui| show_desktop_overview(ui, state));
    ui.add_space(8.0);
    content_card(ui, |ui| show_mode_guide(ui, state)).inner
}

fn show_controller_guide(ui: &mut egui::Ui, state: &RuntimeState) {
    let locale = state.locale;
    accent_heading(ui, locale.text(Text::CommandGuide));
    content_card(ui, |ui| {
        gamepad_icons::show_family_selector(
            ui,
            locale.text(Text::ControllerSymbols),
            locale.text(Text::ControllerSymbolsHelp),
        );
    });
    ui.add_space(8.0);
    if state.settings.mode == ControlMode::Game {
        content_card(ui, |ui| {
            accent_heading(ui, locale.text(Text::GameMode));
            ui.label(locale.text(Text::GameCommandsHelp));
            shortcut_guide(ui, locale, &MOUSE_SHORTCUTS[13..14]);
        });
        ui.add_space(8.0);
    }
    content_card(ui, |ui| {
        egui::CollapsingHeader::new(locale.text(Text::DesktopCommands))
            .default_open(state.settings.mode == ControlMode::Desktop && !state.keyboard.open)
            .show(ui, |ui| {
                ui.small(locale.text(Text::DesktopCommandsHelp));
                ui.strong(locale.text(Text::PointerCommands));
                shortcut_guide(ui, locale, &MOUSE_SHORTCUTS[2..6]);
                ui.add_space(10.0);
                ui.strong(locale.text(Text::NavigationCommands));
                shortcut_guide(ui, locale, &MOUSE_SHORTCUTS[6..11]);
                shortcut_guide(ui, locale, &MOUSE_SHORTCUTS[1..2]);
                ui.add_space(10.0);
                ui.strong(locale.text(Text::ScreenshotAndFullscreen));
                shortcut_guide(ui, locale, &MOUSE_SHORTCUTS[14..16]);
            });
    });
    ui.add_space(8.0);
    content_card(ui, |ui| {
        egui::CollapsingHeader::new(locale.text(Text::KeyboardCommands))
            .default_open(state.keyboard.open)
            .show(ui, |ui| {
                if state.keyboard.presentation == KeyboardPresentation::System {
                    ui.small(locale.text(Text::SystemKeyboardHelp));
                    shortcut_guide(ui, locale, &MOUSE_SHORTCUTS[..1]);
                    return;
                }
                ui.small(locale.text(Text::KeyboardCommandsHelp));
                shortcut_guide(ui, locale, &MOUSE_SHORTCUTS[..1]);
                ui.add_space(8.0);
                shortcut_guide(ui, locale, KEYBOARD_SHORTCUTS);
            });
        ui.small(locale.text(Text::ButtonPositionNote));
    });
}

fn show_desktop_settings(
    ui: &mut egui::Ui,
    state: &mut RuntimeState,
    desktop: &SharedDesktop,
    mapping_text: &mut String,
    tray_error: Option<&str>,
    panel: &PanelControl,
    tray_menu: Option<&TrayMenu>,
) {
    desktop_settings::show(
        ui,
        state,
        desktop,
        mapping_text,
        tray_error,
        panel,
        tray_menu,
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShutdownClose {
    Idle,
    Waiting,
    Allowed,
}

fn handle_shutdown_close(
    context: &egui::Context,
    should_exit: bool,
    shutdown_started: &mut bool,
    shutdown_complete: bool,
    begin_shutdown: impl FnOnce(),
) -> ShutdownClose {
    let close_requested = context.input(|input| input.viewport().close_requested());
    if !*shutdown_started {
        if !should_exit || !close_requested {
            return ShutdownClose::Idle;
        }
        *shutdown_started = true;
        begin_shutdown();
    }

    if !shutdown_complete {
        if close_requested {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        ShutdownClose::Waiting
    } else if close_requested {
        ShutdownClose::Allowed
    } else {
        context.send_viewport_cmd(egui::ViewportCommand::Close);
        ShutdownClose::Waiting
    }
}

fn show_shutdown_pending(ui: &mut egui::Ui, message: &str) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.centered_and_justified(|ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(message);
            });
        });
    });
}

fn handle_panel_close(context: &egui::Context, panel: &PanelControl, tray_available: bool) {
    // Wayland cannot hide a native window. Linux closes the panel and waits in
    // main's tray loop; the desktop worker and layer keyboard belong to the process.
    if cfg!(target_os = "windows")
        && context.input(|input| input.viewport().close_requested())
        && tray_available
        && !panel.exiting()
    {
        context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }
}

fn show_keyboard_viewport(ui: &mut egui::Ui, session: KeyboardSession, immediate: bool) {
    let locale = session.locale;
    let screen = ui
        .ctx()
        .input(|input| input.viewport().monitor_size)
        .unwrap_or(egui::vec2(1920.0, 1080.0));
    let (width, height) = session.settings.surface_size();
    let viewport = egui::ViewportId::from_hash_of("gamepad-desktop-keyboard");
    let pixels_per_point = ui
        .ctx()
        .input(|i| {
            i.raw
                .viewports
                .get(&viewport)
                .and_then(|v| v.native_pixels_per_point)
        })
        .unwrap_or_else(|| ui.ctx().native_pixels_per_point().unwrap_or(1.0));
    // UIA uses physical pixels; egui viewport positions use logical points.
    let factor = if cfg!(target_os = "windows") {
        pixels_per_point
    } else {
        1.0
    };
    let screen =
        session
            .focused_input
            .and_then(|f| f.screen)
            .unwrap_or(gamepad_desktop::placement::Rect {
                x: 0,
                y: 0,
                width: (screen.x * factor) as i32,
                height: (screen.y * factor) as i32,
            });
    let rect = gamepad_desktop::placement::keyboard_rect(
        screen,
        session.focused_input.map(|f| f.bounds),
        (
            (width as f32 * factor) as u32,
            (height as f32 * factor) as u32,
        ),
        session.settings.position == KeyboardPosition::Top,
    );
    let size = egui::vec2(rect.width as f32 / factor, rect.height as f32 / factor);
    let position = egui::pos2(rect.x as f32 / factor, rect.y as f32 / factor);
    let builder = keyboard_viewport_builder(
        size,
        position,
        session.open,
        cfg!(target_os = "windows"),
        locale,
    );
    if immediate {
        ui.ctx()
            .show_viewport_immediate(viewport, builder, move |ui, _| {
                keyboard_viewport_ui(ui, &session)
            });
    } else {
        ui.ctx()
            .show_viewport_deferred(viewport, builder, move |ui, _| {
                keyboard_viewport_ui(ui, &session)
            });
    }
}

fn keyboard_viewport_builder(
    size: egui::Vec2,
    position: egui::Pos2,
    open: bool,
    windows: bool,
    locale: Locale,
) -> egui::ViewportBuilder {
    let builder = egui::ViewportBuilder::default()
        .with_title(locale.text(Text::KeyboardWindowTitle))
        .with_inner_size(size)
        .with_position(position)
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_active(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_has_shadow(false)
        .with_visible(open);
    if windows {
        // winit sets WS_EX_LAYERED for mouse passthrough on Windows, hiding
        // transparent overlay content. The Windows helper applies click-through
        // without that style after the viewport exists.
        builder
    } else {
        builder.with_mouse_passthrough(true)
    }
}

fn main_viewport_builder(windows: bool) -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(APP_NAME)
        .with_icon(gamepad_icon_data(512))
        .with_inner_size([860.0, 760.0])
        .with_min_inner_size([660.0, 500.0])
        // eframe shares the root GL config with child viewports.
        .with_transparent(windows)
}

fn show_app_version(ui: &mut egui::Ui) {
    ui.small(concat!("v", env!("CARGO_PKG_VERSION")));
}

#[cfg(test)]
mod overlay_tests {
    use super::*;

    #[test]
    fn windows_keyboard_viewport_renders_during_the_parent_pass() {
        let context = egui::Context::default();
        context.set_embed_viewports(false);
        let rendered_immediately = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let rendered = Arc::clone(&rendered_immediately);
        egui::Context::set_immediate_viewport_renderer(move |context, mut viewport| {
            rendered.store(true, std::sync::atomic::Ordering::Relaxed);
            let mut input = egui::RawInput {
                viewport_id: viewport.ids.this,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 258.0),
                )),
                ..Default::default()
            };
            input.viewports.insert(
                viewport.ids.this,
                egui::ViewportInfo {
                    parent: Some(viewport.ids.parent),
                    ..Default::default()
                },
            );
            let mut output = context.run_ui(input, |ui| (viewport.viewport_ui_cb)(ui));
            output.textures_delta.clear();
        });
        let mut session = KeyboardSession::default();
        session.open = true;

        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 258.0),
                )),
                ..Default::default()
            },
            |ui| show_keyboard_viewport(ui, session.clone(), true),
        );
        output.textures_delta.clear();

        assert!(rendered_immediately.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn windows_root_viewport_requests_transparency_for_keyboard_overlay() {
        assert_eq!(main_viewport_builder(true).transparent, Some(true));
        assert_eq!(main_viewport_builder(false).transparent, Some(false));
    }

    #[test]
    fn windows_keyboard_viewport_avoids_winit_mouse_passthrough() {
        let builder = keyboard_viewport_builder(
            egui::vec2(640.0, 229.0),
            egui::Pos2::ZERO,
            true,
            true,
            Locale::default(),
        );
        assert_eq!(builder.mouse_passthrough, None);
    }

    #[test]
    fn closed_keyboard_has_no_visible_window_or_native_shadow() {
        let builder = keyboard_viewport_builder(
            egui::vec2(640.0, 229.0),
            egui::Pos2::ZERO,
            false,
            false,
            Locale::default(),
        );
        assert_eq!(builder.visible, Some(false));
        assert_eq!(builder.has_shadow, Some(false));
        assert_eq!(builder.transparent, Some(true));
        assert_eq!(builder.active, Some(false));
    }
}

fn keyboard_viewport_ui(ui: &mut egui::Ui, session: &KeyboardSession) {
    #[cfg(target_os = "windows")]
    if session.open {
        windows_keyboard_overlay::enable_click_through();
    }
    ui.ctx().request_repaint_after(UI_REPAINT_INTERVAL);
    keyboard_view::show(ui, session);
}

fn shortcut_guide(ui: &mut egui::Ui, locale: Locale, shortcuts: &[(&str, &str)]) {
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .max_col_width((ui.available_width() - 16.0) / 2.0)
        .spacing([16.0, 9.0])
        .show(ui, |ui| {
            for (button, action) in shortcuts {
                let source_button = *button;
                let button = locale.translate(source_button);
                let action = locale.translate(action);
                let family = gamepad_icons::family(ui.ctx());
                egui::Frame::new()
                    .fill(WIDGET_BACKGROUND)
                    .stroke(egui::Stroke::new(1.0, KEYCAP_BORDER))
                    .corner_radius(6)
                    .inner_margin(egui::Margin::symmetric(7, 3))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            match source_button {
                                "Start/Menu + LB/L1" => {
                                    gamepad_icons::show_shortcut_icons(
                                        ui,
                                        family,
                                        "Start / Menu / +",
                                    );
                                    gamepad_icons::show_shortcut_icons(ui, family, "LB / L1");
                                }
                                "Start/Menu + RB/R1" => {
                                    gamepad_icons::show_shortcut_icons(
                                        ui,
                                        family,
                                        "Start / Menu / +",
                                    );
                                    gamepad_icons::show_shortcut_icons(ui, family, "RB / R1");
                                }
                                _ => gamepad_icons::show_shortcut_icons(ui, family, source_button),
                            }
                            ui.label(RichText::new(button).strong().color(ACCENT));
                        });
                    });
                ui.label(action);
                ui.end_row();
            }
        });
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn acquire_instance_lock() -> std::io::Result<Option<std::fs::File>> {
    #[cfg(target_os = "linux")]
    let directory = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".cache"))
        })
        .ok_or_else(|| std::io::Error::other("XDG_RUNTIME_DIR and HOME are unavailable"))?
        .join("gamepad-desktop");

    #[cfg(target_os = "windows")]
    let directory = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(|home| std::path::PathBuf::from(home).join("AppData").join("Local"))
        })
        .ok_or_else(|| std::io::Error::other("LOCALAPPDATA and USERPROFILE are unavailable"))?
        .join("gamepad-desktop");

    std::fs::create_dir_all(&directory)?;
    acquire_instance_lock_at(&directory.join("desktop.lock"))
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn acquire_instance_lock_at(path: &std::path::Path) -> std::io::Result<Option<std::fs::File>> {
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    match lock.try_lock() {
        Ok(()) => Ok(Some(lock)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(error)) => Err(error),
    }
}

fn wait_for_desktop_shutdown(desktop: &SharedDesktop) {
    desktop
        .lock()
        .expect("desktop runtime lock")
        .begin_shutdown();
    while !desktop
        .lock()
        .expect("desktop runtime lock")
        .shutdown_complete()
    {
        std::thread::sleep(UI_REPAINT_INTERVAL);
    }
}

fn main() -> eframe::Result {
    let background_start = std::env::args_os().any(|argument| argument == "--background");
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    let _instance_lock = match acquire_instance_lock() {
        Ok(Some(lock)) => lock,
        Ok(None) => return Ok(()),
        Err(error) => return Err(eframe::Error::AppCreation(Box::new(error))),
    };

    // The service belongs to the process, so closing a Wayland window keeps it alive.
    let desktop = Arc::new(Mutex::new(DesktopRuntime::default()));
    let (panel, actions) = PanelControl::new();
    let locale = desktop.lock().expect("desktop runtime lock").state().locale;
    let tray_result = create_tray(Arc::clone(&panel), locale);
    let tray_error = tray_result.as_ref().err().cloned();
    let tray_menu = tray_result.as_ref().ok().map(|(_, menu)| menu.clone());
    let tray = tray_result.map(|(icon, _)| icon);
    loop {
        let options = eframe::NativeOptions {
            viewport: main_viewport_builder(cfg!(target_os = "windows")).with_visible(
                !background_start
                    || tray.is_err()
                    || desktop
                        .lock()
                        .expect("desktop runtime lock")
                        .state()
                        .onboarding
                        .is_some(),
            ),
            run_and_return: true,
            ..Default::default()
        };

        let app_desktop = Arc::clone(&desktop);
        let app_panel = Arc::clone(&panel);
        let app_error = tray_error.clone();
        let app_tray_menu = tray_menu.clone();
        eframe::run_native(
            APP_NAME,
            options,
            Box::new(move |creation_context| {
                configure_desktop_style(&creation_context.egui_ctx);
                app_panel.attach(Some(creation_context.egui_ctx.clone()));
                Ok(Box::new(GamepadDesktopApp::new(
                    app_desktop,
                    app_panel,
                    app_error,
                    app_tray_menu,
                )))
            }),
        )?;
        panel.attach(None);
        if panel.exiting() || tray.is_err() || cfg!(target_os = "windows") {
            wait_for_desktop_shutdown(&desktop);
            break;
        }
        match actions.recv() {
            Ok(TrayAction::Open) => {}
            Ok(TrayAction::Exit) | Err(_) => {
                wait_for_desktop_shutdown(&desktop);
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_renders_the_current_package_version() {
        let mut output = egui::Context::default().run_ui(Default::default(), show_app_version);
        let expected = format!("v{}", env!("CARGO_PKG_VERSION"));
        output.textures_delta.clear();

        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Text(text) if text.galley.job.text.as_str() == expected
        )));
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn only_one_desktop_instance_can_hold_the_lock() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "gamepad-desktop-lock-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("desktop.lock");

        let first = acquire_instance_lock_at(&path)
            .unwrap()
            .expect("first instance should acquire the lock");
        assert!(acquire_instance_lock_at(&path).unwrap().is_none());
        drop(first);
        assert!(acquire_instance_lock_at(&path).unwrap().is_some());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn closing_windows_panel_hides_it_without_ending_the_tray_message_loop() {
        let (panel, _) = PanelControl::new();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let mut output = egui::Context::default()
            .run_pass(input, |context| handle_panel_close(context, &panel, true));
        output.textures_delta.clear();
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::CancelClose))
        );
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Visible(false)))
        );
        assert!(!panel.exiting());
    }

    #[test]
    fn explicit_exit_is_never_canceled_by_close_to_tray() {
        let (panel, actions) = PanelControl::new();
        panel.exit();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let mut output = egui::Context::default()
            .run_pass(input, |context| handle_panel_close(context, &panel, true));
        output.textures_delta.clear();
        assert!(
            !output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::CancelClose))
        );
        assert_eq!(actions.try_recv().unwrap(), TrayAction::Exit);
    }

    #[test]
    fn shutdown_closes_only_after_async_cleanup_finishes() {
        let context = egui::Context::default();
        let mut shutdown_started = false;
        let mut shutdown_requests = 0;
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let mut output = context.run_pass(input, |context| {
            assert_eq!(
                handle_shutdown_close(context, true, &mut shutdown_started, false, || {
                    shutdown_requests += 1;
                }),
                ShutdownClose::Waiting
            );
        });
        output.textures_delta.clear();
        assert!(shutdown_started);
        assert_eq!(shutdown_requests, 1);
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::CancelClose))
        );

        let mut output = context.run_pass(egui::RawInput::default(), |context| {
            assert_eq!(
                handle_shutdown_close(context, true, &mut shutdown_started, true, || {
                    shutdown_requests += 1;
                }),
                ShutdownClose::Waiting
            );
        });
        output.textures_delta.clear();
        assert_eq!(shutdown_requests, 1);
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close))
        );
    }

    #[test]
    fn close_to_tray_does_not_start_runtime_shutdown() {
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let mut shutdown_started = false;
        let mut shutdown_requests = 0;
        let mut output = egui::Context::default().run_pass(input, |context| {
            assert_eq!(
                handle_shutdown_close(context, false, &mut shutdown_started, false, || {
                    shutdown_requests += 1;
                }),
                ShutdownClose::Idle
            );
        });
        output.textures_delta.clear();
        assert!(!shutdown_started);
        assert_eq!(shutdown_requests, 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn closing_linux_panel_keeps_the_service_but_allows_the_window_to_close() {
        let (panel, actions) = PanelControl::new();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let mut output = egui::Context::default()
            .run_pass(input, |context| handle_panel_close(context, &panel, true));
        output.textures_delta.clear();
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::CancelClose)),
            "Wayland cannot hide a window with Visible(false); allow its actual close"
        );
        assert!(!panel.exiting());
        assert!(actions.try_recv().is_err());
    }

    #[test]
    fn shortcut_guides_fit_the_minimum_panel_width() {
        let context = egui::Context::default();
        configure_desktop_style(&context);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(660.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut output = context.run_pass(input, |context| {
            let mut root = egui::Ui::new(
                context.clone(),
                egui::Id::unique("shortcut-guide-test"),
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(660.0, 1000.0),
                )),
            );
            egui::CentralPanel::default().show(&mut root, |ui| {
                shortcut_guide(ui, Locale::default(), MOUSE_SHORTCUTS);
                assert!(
                    ui.min_rect().right() <= 660.0,
                    "guide overflows: {:?}",
                    ui.min_rect()
                );
            });
        });
        output.textures_delta.clear();
    }

    #[test]
    fn mode_guide_fits_a_narrow_panel_in_every_language_and_mode() {
        for locale in Locale::ALL {
            for mode in [ControlMode::Desktop, ControlMode::Game] {
                let context = egui::Context::default();
                configure_desktop_style(&context);
                let mut state = RuntimeState::default();
                state.locale = locale;
                state.settings.mode = mode;
                state.settings.enabled = false;
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(620.0, 1400.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        assert!(!show_mode_guide(ui, &mut state));
                        assert!(ui.min_rect().right() <= 620.0);
                    },
                );
                output.textures_delta.clear();
                assert_eq!(state.settings.mode, mode);
            }
        }
    }
}
