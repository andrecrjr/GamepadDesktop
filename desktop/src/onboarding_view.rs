use super::{
    ACCENT, BORDER, CARD_BACKGROUND, MUTED_TEXT, PAGE_BACKGROUND, TEXT, egui, gamepad_icons,
};
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use gamepad_desktop::{
    control::{ControlMode, MOUSE_SHORTCUTS},
    i18n::{Locale, Text},
    keyboard::{KEY_ROWS, key_width_quarters},
    onboarding::{Command, Onboarding, Step},
    runtime::RuntimeState,
};
use sdl3::gamepad::GamepadType;

const CHAPTERS: [Text; 6] = [
    Text::TourShortWelcome,
    Text::TourShortModes,
    Text::MouseLabel,
    Text::TourShortKeyboard,
    Text::TourShortSwitch,
    Text::TourShortReady,
];

pub fn show(ui: &mut egui::Ui, state: &RuntimeState) -> Option<Command> {
    let tour = state.onboarding.as_ref()?;
    let locale = state.locale;
    auto_family(ui.ctx(), state.controller_type);
    let mut command = None;
    egui::Panel::bottom("welcome-navigation")
        .frame(egui::Frame::new().fill(PAGE_BACKGROUND).inner_margin(18))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        tour.step != Step::Welcome,
                        egui::Button::new(locale.text(Text::TourBack)),
                    )
                    .clicked()
                {
                    command = Some(Command::Back);
                }
                if tour.step == Step::Ready {
                    for (mode, text) in [
                        (ControlMode::Desktop, Text::TourStartDesktop),
                        (ControlMode::Game, Text::TourStartGame),
                    ] {
                        if ui
                            .add(egui::Button::new(locale.text(text)).fill(
                                if tour.start_mode == mode {
                                    Color32::from_rgb(28, 83, 77)
                                } else {
                                    CARD_BACKGROUND
                                },
                            ))
                            .clicked()
                        {
                            command = Some(Command::Finish(mode));
                        }
                    }
                } else {
                    let text = if tour.step == Step::Welcome {
                        Text::TourStart
                    } else {
                        Text::TourNext
                    };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(locale.text(text)).strong())
                                .fill(Color32::from_rgb(28, 83, 77)),
                        )
                        .clicked()
                    {
                        command = Some(Command::Next);
                    }
                }
                if ui.link(locale.text(Text::TourSkip)).clicked() {
                    command = Some(Command::Skip);
                }
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                let family = gamepad_icons::family(ui.ctx());
                gamepad_icons::show_shortcut_icons(ui, family, "LB / L1");
                gamepad_icons::show_shortcut_icons(ui, family, "RB / R1");
                ui.small(locale.text(match tour.step {
                    Step::WindowSwitcher => Text::TourNavigationWindowSwitcher,
                    Step::DesktopShortcuts => Text::TourNavigationDesktopShortcuts,
                    _ => Text::TourNavigation,
                }));
            });
        });
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("../assets/gamepad-desktop.svg"))
                        .fit_to_exact_size(Vec2::splat(54.0)),
                );
                ui.vertical(|ui| {
                    ui.label(RichText::new("Gamepad Desktop").strong().size(22.0));
                    ui.label(RichText::new(locale.text(Text::TourShortWelcome)).color(ACCENT));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut language = locale;
                    egui::ComboBox::from_id_salt("welcome-language")
                        .selected_text(locale.name())
                        .show_ui(ui, |ui| {
                            for choice in Locale::ALL {
                                ui.selectable_value(&mut language, choice, choice.name());
                            }
                        });
                    if language != locale {
                        command = Some(Command::Language(language));
                    }
                });
            });
            ui.add_space(10.0);
            if let Some(scale) = super::interface_scale(ui, locale, state.ui_scale) {
                command = Some(Command::Scale(scale));
            }
            ui.add_space(16.0);
            progress(ui, tour, locale);
            ui.add_space(14.0);
            egui::Frame::new()
                .fill(CARD_BACKGROUND)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(16)
                .inner_margin(20)
                .show(ui, |ui| {
                    let (title, copy) = lesson_copy(tour.step);
                    ui.label(RichText::new(locale.text(title)).strong().size(26.0));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(locale.text(copy))
                            .size(15.0)
                            .color(MUTED_TEXT),
                    );
                    ui.add_space(16.0);
                    if matches!(tour.step, Step::Welcome | Step::Modes | Step::Ready) {
                        modes(ui, locale);
                    } else {
                        let width = ui.available_width();
                        if width >= 600.0 {
                            ui.horizontal_top(|ui| {
                                ui.allocate_ui(Vec2::new(190.0, 210.0), |ui| {
                                    controller_diagram(ui, tour.step)
                                });
                                ui.allocate_ui(Vec2::new(width - 210.0, 210.0), |ui| {
                                    scene(ui, tour, locale)
                                });
                            });
                        } else {
                            scene(ui, tour, locale);
                            controller_diagram(ui, tour.step);
                        }
                        ui.add_space(10.0);
                        shortcut(ui, tour.step, locale);
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(locale.text(if tour.complete {
                                if tour.demonstrating {
                                    Text::TourDemoSuccess
                                } else {
                                    Text::TourSuccess
                                }
                            } else {
                                Text::TourOptional
                            }))
                            .color(if tour.complete {
                                ACCENT
                            } else {
                                MUTED_TEXT
                            }),
                        );
                        ui.add_space(8.0);
                        if ui
                            .button(locale.text(if tour.demonstrating {
                                Text::TourPractice
                            } else {
                                Text::TourDemo
                            }))
                            .clicked()
                        {
                            command = Some(Command::ToggleDemo);
                        }
                        if tour.demonstrating {
                            ui.label(
                                RichText::new(locale.text(Text::TourDemoBadge))
                                    .small()
                                    .color(ACCENT),
                            );
                        }
                        ui.add_space(8.0);
                        ui.small(locale.text(Text::TourSandbox));
                        if state.ready
                            && state.keyboard.presentation
                                == gamepad_desktop::keyboard::KeyboardPresentation::Unavailable
                            && matches!(
                                tour.step,
                                Step::OpenKeyboard | Step::Type | Step::CloseKeyboard
                            )
                        {
                            ui.small(locale.text(Text::TourKeyboardUnavailable));
                        }
                    }
                });
            ui.add_space(12.0);
            if let Some(controller) = &state.controller {
                ui.label(
                    RichText::new(format!(
                        "{}{controller}",
                        locale.text(Text::ControllerPrefix)
                    ))
                    .color(ACCENT),
                );
            } else {
                let needs_profile =
                    !state.devices.is_empty() && state.devices.iter().all(|device| !device.mapped);
                ui.label(
                    RichText::new(locale.text(if needs_profile {
                        Text::TourProfile
                    } else {
                        Text::TourConnect
                    }))
                    .color(MUTED_TEXT),
                );
            }
            if tour.step == Step::Welcome {
                ui.add_space(8.0);
                let before = gamepad_icons::family(ui.ctx());
                gamepad_icons::show_family_selector(
                    ui,
                    locale.text(Text::ControllerSymbols),
                    locale.text(Text::ControllerSymbolsHelp),
                );
                if before != gamepad_icons::family(ui.ctx()) {
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(egui::Id::unique("welcome-manual-family"), true)
                    });
                }
            }
        });
    });
    command
}

fn progress(ui: &mut egui::Ui, tour: &Onboarding, locale: Locale) {
    let (chapter, part, count) = tour.step.chapter();
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{:02} / 06", chapter + 1))
                .strong()
                .color(ACCENT),
        );
        ui.label(locale.text(CHAPTERS[chapter]));
        if count > 1 {
            ui.label(RichText::new(format!("· {part}/{count}")).color(MUTED_TEXT));
        }
    });
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 4.0), Sense::hover());
    for index in 0..6 {
        let width = (rect.width() - 5.0 * 6.0) / 6.0;
        let bar = Rect::from_min_size(
            rect.min + Vec2::new(index as f32 * (width + 6.0), 0.0),
            Vec2::new(width, 4.0),
        );
        ui.painter()
            .rect_filled(bar, 2.0, if index <= chapter { ACCENT } else { BORDER });
    }
}

fn modes(ui: &mut egui::Ui, locale: Locale) {
    let draw = |ui: &mut egui::Ui, title, copy, color| {
        egui::Frame::new()
            .fill(PAGE_BACKGROUND)
            .stroke(Stroke::new(1.0, color))
            .corner_radius(12)
            .inner_margin(16)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(locale.text(title))
                        .size(20.0)
                        .strong()
                        .color(color),
                );
                ui.add_space(8.0);
                ui.label(locale.text(copy));
            });
    };
    if ui.available_width() >= 480.0 {
        ui.columns(2, |columns| {
            draw(
                &mut columns[0],
                Text::DesktopMode,
                Text::TourDesktopCard,
                ACCENT,
            );
            draw(
                &mut columns[1],
                Text::GameMode,
                Text::TourGameCard,
                Color32::from_rgb(150, 129, 246),
            );
        });
    } else {
        draw(ui, Text::DesktopMode, Text::TourDesktopCard, ACCENT);
        ui.add_space(8.0);
        draw(
            ui,
            Text::GameMode,
            Text::TourGameCard,
            Color32::from_rgb(150, 129, 246),
        );
    }
    ui.add_space(14.0);
    ui.horizontal_wrapped(|ui| {
        gamepad_icons::show_shortcut_icons(
            ui,
            gamepad_icons::family(ui.ctx()),
            "Select · dois toques rápidos",
        );
        ui.label(RichText::new(locale.text(Text::DoubleTapSelectHelp)).color(ACCENT));
    });
    ui.small(locale.text(Text::ResumeGuide));
}

fn family_for_sdl_type(gamepad_type: GamepadType) -> Option<gamepad_icons::ControllerFamily> {
    match gamepad_type {
        GamepadType::Xbox360 | GamepadType::XboxOne => Some(gamepad_icons::ControllerFamily::Xbox),
        GamepadType::PS3 | GamepadType::PS4 | GamepadType::PS5 => {
            Some(gamepad_icons::ControllerFamily::PlayStation)
        }
        _ => None,
    }
}

fn auto_family(context: &egui::Context, gamepad_type: Option<GamepadType>) {
    let Some(gamepad_type) = gamepad_type else {
        return;
    };
    let id = egui::Id::unique("gamepad-guide-controller-family");
    let detected_id = egui::Id::unique("welcome-detected-sdl-type");
    context.data_mut(|data| {
        if data.get_temp::<bool>(egui::Id::unique("welcome-manual-family")) != Some(true)
            && data.get_temp::<GamepadType>(detected_id) != Some(gamepad_type)
        {
            if let Some(family) = family_for_sdl_type(gamepad_type) {
                data.insert_temp(id, family);
            }
            data.insert_temp(detected_id, gamepad_type);
        }
    });
}

fn lesson_copy(step: Step) -> (Text, Text) {
    match step {
        Step::Welcome => (Text::TourWelcome, Text::TourWelcomeCopy),
        Step::Modes => (Text::TourModes, Text::TourModesCopy),
        Step::Pointer => (Text::TourPointer, Text::TourPointerCopy),
        Step::Click => (Text::TourClick, Text::TourClickCopy),
        Step::RightClick => (Text::TourRightClick, Text::TourRightClickCopy),
        Step::Drag => (Text::TourDrag, Text::TourDragCopy),
        Step::Scroll => (Text::TourScroll, Text::TourScrollCopy),
        Step::OpenKeyboard => (Text::TourOpenKeyboard, Text::TourOpenKeyboardCopy),
        Step::Type => (Text::TourType, Text::TourTypeCopy),
        Step::CloseKeyboard => (Text::TourCloseKeyboard, Text::TourCloseKeyboardCopy),
        Step::Game => (Text::TourGame, Text::TourGameCopy),
        Step::Desktop => (Text::TourDesktop, Text::TourDesktopCopy),
        Step::WindowSwitcher => (Text::TourWindowSwitcher, Text::TourWindowSwitcherCopy),
        Step::DesktopShortcuts => (
            Text::ScreenshotAndFullscreen,
            Text::TourDesktopShortcutsCopy,
        ),
        Step::Pause => (Text::TourPause, Text::TourPauseCopy),
        Step::Ready => (Text::TourReady, Text::TourReadyCopy),
    }
}

fn shortcut(ui: &mut egui::Ui, step: Step, locale: Locale) {
    let indices: &[usize] = match step {
        Step::Pointer => &[2],
        Step::Click => &[4],
        Step::RightClick => &[5],
        Step::Drag => &[4, 2],
        Step::Scroll => &[3],
        Step::OpenKeyboard | Step::CloseKeyboard => &[0],
        Step::Game => &[12],
        Step::Desktop => &[12],
        Step::WindowSwitcher => &[10, 6, 7],
        Step::DesktopShortcuts => &[14, 15],
        Step::Pause => &[11],
        _ => &[],
    };
    ui.horizontal_wrapped(|ui| {
        let family = gamepad_icons::family(ui.ctx());
        if step == Step::Type {
            for label in [
                "Direcional / analógico esquerdo",
                "A / botão inferior",
                "B / botão direito",
            ] {
                gamepad_icons::show_shortcut_icons(ui, family, label);
                ui.label(locale.translate(label));
            }
        } else if step == Step::DesktopShortcuts {
            for &index in indices {
                gamepad_icons::show_shortcut_icons(ui, family, "Start / Menu / +");
                gamepad_icons::show_shortcut_icons(
                    ui,
                    family,
                    if index == 14 { "LB / L1" } else { "RB / R1" },
                );
                let (button, _) = MOUSE_SHORTCUTS[index];
                ui.label(
                    RichText::new(locale.translate(button))
                        .strong()
                        .color(ACCENT),
                );
            }
        } else {
            for &index in indices {
                let (button, _) = MOUSE_SHORTCUTS[index];
                gamepad_icons::show_shortcut_icons(ui, family, button);
                ui.label(
                    RichText::new(locale.translate(button))
                        .strong()
                        .color(ACCENT),
                );
            }
        }
    });
}

fn controller_diagram(ui: &mut egui::Ui, step: Step) {
    let size = Vec2::new(ui.available_width().min(190.0), 135.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let point = |x, y| rect.min + Vec2::new(x * rect.width(), y * rect.height());
    let painter = ui.painter();
    let body = [
        (0.19, 0.25),
        (0.81, 0.25),
        (0.9, 0.42),
        (0.99, 0.85),
        (0.86, 0.96),
        (0.67, 0.73),
        (0.33, 0.73),
        (0.14, 0.96),
        (0.01, 0.85),
        (0.1, 0.42),
    ]
    .map(|(x, y)| point(x, y))
    .to_vec();
    painter.add(egui::Shape::convex_polygon(
        body,
        PAGE_BACKGROUND,
        Stroke::new(2.0, BORDER),
    ));
    let ps = gamepad_icons::family(ui.ctx()) == gamepad_icons::ControllerFamily::PlayStation;
    let left = if ps { (0.39, 0.65) } else { (0.27, 0.43) };
    let dpad = if ps { (0.25, 0.43) } else { (0.39, 0.65) };
    use gamepad_icons::Icon;
    let family = gamepad_icons::family(ui.ctx());
    let button = |x, y, icon, active, size| {
        let center = point(x, y);
        if active {
            painter.circle_filled(center, size * 0.7, Color32::from_rgb(28, 83, 77));
            painter.circle_stroke(center, size * 0.7, Stroke::new(2.0, ACCENT));
        }
        egui::Image::new(gamepad_icons::image_source(family, icon))
            .paint_at(ui, Rect::from_center_size(center, Vec2::splat(size)));
    };
    button(
        left.0,
        left.1,
        Icon::LeftStick,
        matches!(step, Step::Pointer | Step::Drag | Step::Type),
        24.0,
    );
    button(
        0.61,
        0.65,
        if step == Step::Scroll {
            Icon::RightStickUp
        } else {
            Icon::R3
        },
        matches!(
            step,
            Step::Scroll | Step::OpenKeyboard | Step::CloseKeyboard
        ),
        24.0,
    );
    button(dpad.0, dpad.1, Icon::DpadUp, step == Step::Type, 24.0);
    for (x, y, icon) in [
        (0.76, 0.30, Icon::North),
        (0.85, 0.43, Icon::East),
        (0.76, 0.56, Icon::South),
        (0.67, 0.43, Icon::West),
    ] {
        button(
            x,
            y,
            icon,
            (y == 0.56 && matches!(step, Step::Click | Step::Drag | Step::Type))
                || (y == 0.43 && x == 0.85 && step == Step::RightClick)
                || (step == Step::WindowSwitcher
                    && ((y == 0.43 && x == 0.67) || (y == 0.30 && x == 0.76))),
            18.0,
        );
    }
    button(
        0.45,
        0.4,
        Icon::View,
        matches!(step, Step::Game | Step::Desktop | Step::Pause),
        18.0,
    );
    button(
        0.55,
        0.4,
        Icon::Menu,
        matches!(step, Step::Pause | Step::DesktopShortcuts),
        18.0,
    );
}

fn scene(ui: &mut egui::Ui, tour: &Onboarding, locale: Locale) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 210.0), Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 12.0, PAGE_BACKGROUND);
    painter.rect_stroke(rect, 12.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
    painter.text(
        rect.min + Vec2::new(14.0, 17.0),
        Align2::LEFT_CENTER,
        locale.text(match tour.step {
            Step::WindowSwitcher => Text::TourWindowSwitcher,
            Step::DesktopShortcuts => Text::TourShortcutPreview,
            _ => Text::TourPracticeDesktop,
        }),
        FontId::proportional(11.0),
        MUTED_TEXT,
    );
    let content = Rect::from_min_max(
        rect.min + Vec2::new(12.0, 34.0),
        rect.max - Vec2::splat(12.0),
    );
    match tour.step {
        Step::Pointer | Step::Click | Step::RightClick | Step::Drag => {
            let center = content.min + content.size() * Vec2::new(0.72, 0.5);
            let target = Rect::from_center_size(center, content.size() * Vec2::new(0.28, 0.44));
            painter.rect_filled(
                target,
                9.0,
                if tour.complete || tour.mouse_held {
                    Color32::from_rgb(28, 83, 77)
                } else {
                    CARD_BACKGROUND
                },
            );
            painter.rect_stroke(target, 9.0, Stroke::new(1.5, ACCENT), StrokeKind::Inside);
            painter.text(
                center,
                Align2::CENTER_CENTER,
                locale.text(match tour.step {
                    Step::Pointer => Text::TourMoveTarget,
                    Step::Drag => Text::TourDropTarget,
                    _ => Text::TourTarget,
                }),
                FontId::proportional(12.0),
                ACCENT,
            );
            if tour.step == Step::Drag {
                let item = Rect::from_center_size(
                    content.min
                        + content.size() * Vec2::new(tour.item_position[0], tour.item_position[1]),
                    content.size() * Vec2::new(0.24, 0.34),
                );
                painter.rect_filled(item, 6.0, if tour.mouse_held { ACCENT } else { BORDER });
                painter.text(
                    item.center(),
                    Align2::CENTER_CENTER,
                    locale.text(Text::TourDragItem),
                    FontId::proportional(12.0),
                    if tour.mouse_held {
                        PAGE_BACKGROUND
                    } else {
                        TEXT
                    },
                );
            }
            if tour.step == Step::RightClick && tour.complete {
                let menu = Rect::from_min_size(
                    Pos2::new(
                        (center.x - 80.0)
                            .max(content.left())
                            .min(content.right() - content.width().min(160.0)),
                        content.top() + content.height() * 0.65,
                    ),
                    Vec2::new(content.width().min(160.0), content.height() * 0.3),
                );
                painter.rect_filled(menu, 6.0, BORDER);
                painter.text(
                    menu.center(),
                    Align2::CENTER_CENTER,
                    locale.text(Text::TourContextMenu),
                    FontId::proportional(12.0),
                    TEXT,
                );
            }
            let cursor = content.min + content.size() * Vec2::new(tour.cursor[0], tour.cursor[1]);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    cursor,
                    cursor + Vec2::new(0.0, 24.0),
                    cursor + Vec2::new(7.0, 17.0),
                    cursor + Vec2::new(19.0, 16.0),
                ],
                TEXT,
                Stroke::new(1.0, PAGE_BACKGROUND),
            ));
        }
        Step::Scroll => {
            let clipped = painter.with_clip_rect(content);
            for index in 0..5 {
                let item = Rect::from_min_size(
                    content.min + Vec2::new(8.0, index as f32 * 52.0 - tour.scroll * 85.0),
                    Vec2::new(content.width() - 16.0, 42.0),
                );
                clipped.rect_filled(item, 6.0, CARD_BACKGROUND);
                clipped.circle_filled(item.left_center() + Vec2::new(20.0, 0.0), 10.0, ACCENT);
                clipped.line_segment(
                    [
                        item.left_center() + Vec2::new(44.0, -4.0),
                        item.right_center() - Vec2::new(30.0, 4.0),
                    ],
                    Stroke::new(4.0, BORDER),
                );
            }
        }
        Step::OpenKeyboard | Step::Type | Step::CloseKeyboard => {
            let field = Rect::from_min_size(content.min, Vec2::new(content.width(), 30.0));
            painter.rect_stroke(field, 5.0, Stroke::new(1.0, ACCENT), StrokeKind::Inside);
            painter.text(
                field.left_center() + Vec2::new(9.0, 0.0),
                Align2::LEFT_CENTER,
                if tour.typed.is_empty() {
                    locale.text(Text::TourTypingField)
                } else {
                    &tour.typed
                },
                FontId::proportional(12.0),
                TEXT,
            );
            if tour.keyboard().open {
                let key_height = (content.height() - 40.0) / 6.0;
                let unit = content.width() / 15.0;
                for (row, keys) in KEY_ROWS.iter().enumerate() {
                    let mut x = content.left();
                    for (column, key) in keys.iter().enumerate() {
                        let width = f32::from(key_width_quarters(row, column)) / 4.0 * unit;
                        let cell = Rect::from_min_size(
                            Pos2::new(x, content.top() + 38.0 + row as f32 * key_height),
                            Vec2::new(width - 2.0, key_height - 2.0),
                        );
                        let selected = tour.keyboard().selected_row == row
                            && tour.keyboard().selected_column == column;
                        painter.rect_filled(
                            cell,
                            3.0,
                            if selected { ACCENT } else { CARD_BACKGROUND },
                        );
                        painter.text(
                            cell.center(),
                            Align2::CENTER_CENTER,
                            key.label_in(false, locale),
                            FontId::proportional(9.0),
                            if selected { PAGE_BACKGROUND } else { TEXT },
                        );
                        x += width;
                    }
                }
            } else {
                egui::Image::new(gamepad_icons::image_source(
                    gamepad_icons::family(ui.ctx()),
                    gamepad_icons::Icon::R3,
                ))
                .paint_at(
                    ui,
                    Rect::from_center_size(
                        content.center() + Vec2::new(0.0, 18.0),
                        Vec2::splat(48.0),
                    ),
                );
            }
        }
        Step::WindowSwitcher => {
            let gap = 8.0;
            let card_width = (content.width() - gap * 2.0) / 3.0;
            let card_height = 96.0;
            let top = content.center().y - card_height / 2.0;
            for index in 0..3 {
                let selected = tour.window_switcher_preview() == Some(index);
                let card = Rect::from_min_size(
                    Pos2::new(content.left() + index as f32 * (card_width + gap), top),
                    Vec2::new(card_width, card_height),
                );
                painter.rect_filled(
                    card,
                    8.0,
                    if selected {
                        Color32::from_rgb(28, 83, 77)
                    } else {
                        CARD_BACKGROUND
                    },
                );
                painter.rect_stroke(
                    card,
                    8.0,
                    Stroke::new(
                        if selected { 2.0 } else { 1.0 },
                        if selected { ACCENT } else { BORDER },
                    ),
                    StrokeKind::Inside,
                );
                let title_bar = Rect::from_min_size(card.min, Vec2::new(card.width(), 17.0));
                painter.rect_filled(title_bar, 7.0, BORDER);
                painter.circle_filled(
                    title_bar.left_center() + Vec2::new(9.0, 0.0),
                    2.0,
                    if selected { ACCENT } else { MUTED_TEXT },
                );
                let left = card.left() + 10.0;
                let body_top = title_bar.bottom() + 10.0;
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(left, body_top), Vec2::splat(15.0)),
                    3.0,
                    if selected { ACCENT } else { BORDER },
                );
                for row in 0..3 {
                    let width = card.width() * [0.48, 0.68, 0.38][row];
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(left + 22.0, body_top + row as f32 * 8.0 + 2.0),
                            Vec2::new(width, 3.0),
                        ),
                        2.0,
                        MUTED_TEXT,
                    );
                }
            }
        }
        Step::DesktopShortcuts => {
            let gap = 8.0;
            let height = (content.height() - gap) / 2.0;
            for (row, (index, label)) in [
                (14, Text::PrintScreenShortcut),
                (15, Text::FullscreenShortcut),
            ]
            .into_iter()
            .enumerate()
            {
                let card = Rect::from_min_size(
                    Pos2::new(content.left(), content.top() + row as f32 * (height + gap)),
                    Vec2::new(content.width(), height),
                );
                let active = matches!(
                    (index, tour.shortcut_preview()),
                    (
                        14,
                        Some(gamepad_desktop::control::KeyboardInput::PrintScreen)
                    ) | (
                        15,
                        Some(gamepad_desktop::control::KeyboardInput::Function(11))
                    )
                );
                painter.rect_filled(
                    card,
                    8.0,
                    if active {
                        Color32::from_rgb(28, 83, 77)
                    } else {
                        CARD_BACKGROUND
                    },
                );
                painter.rect_stroke(
                    card,
                    8.0,
                    Stroke::new(
                        if active { 2.0 } else { 1.0 },
                        if active { ACCENT } else { BORDER },
                    ),
                    StrokeKind::Inside,
                );
                painter.text(
                    card.min + Vec2::new(12.0, 20.0),
                    Align2::LEFT_CENTER,
                    locale.translate(MOUSE_SHORTCUTS[index].0),
                    FontId::proportional(11.0),
                    if active { ACCENT } else { MUTED_TEXT },
                );
                painter.text(
                    card.min + Vec2::new(12.0, 48.0),
                    Align2::LEFT_CENTER,
                    locale.text(label),
                    FontId::proportional(13.0),
                    TEXT,
                );
            }
        }
        _ => {
            let settings = tour.practice_settings();
            let color = if settings.mode == ControlMode::Game {
                Color32::from_rgb(150, 129, 246)
            } else {
                ACCENT
            };
            let text = if !settings.enabled {
                Text::PausedLabel
            } else if settings.mode == ControlMode::Game {
                Text::GameModeBadge
            } else {
                Text::DesktopMode
            };
            painter.text(
                content.center(),
                Align2::CENTER_CENTER,
                locale.text(text),
                FontId::proportional(26.0),
                color,
            );
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(Rect::from_center_size(
                    content.center() + Vec2::new(0.0, 42.0),
                    Vec2::new(100.0, 28.0),
                )),
                |ui| {
                    ui.horizontal(|ui| {
                        let family = gamepad_icons::family(ui.ctx());
                        if tour.step == Step::Pause {
                            gamepad_icons::show_shortcut_icons(
                                ui,
                                family,
                                "Start + Select / Menu + View",
                            );
                        } else {
                            gamepad_icons::show_shortcut_icons(
                                ui,
                                family,
                                "Select · dois toques rápidos",
                            );
                            ui.label("× 2");
                        }
                    });
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_return_lesson_stays_focused_on_one_action_in_all_languages() {
        for locale in Locale::ALL {
            let copy = locale.text(Text::TourDesktopCopy);
            assert!(
                copy.split_whitespace().count() <= 14,
                "desktop return lesson is too dense in {locale:?}: {copy}"
            );
            assert!(
                !["Alt+Tab", "Print Screen", "F11", "X/□", "Y/△"]
                    .iter()
                    .any(|command| copy.contains(command)),
                "desktop return lesson mixes in another shortcut in {locale:?}: {copy}"
            );
        }
    }

    #[test]
    fn window_switcher_lesson_explains_controls_and_navigation_in_all_languages() {
        for locale in Locale::ALL {
            let copy = locale.text(Text::TourWindowSwitcherCopy);
            for control in ["R1", "Alt+Tab", "X/□", "Y/△", "A/✕"] {
                assert!(
                    copy.contains(control),
                    "missing {control} in {locale:?}: {copy}"
                );
            }
        }
    }

    #[test]
    fn desktop_shortcut_lesson_explains_both_combinations_in_all_languages() {
        for (locale, print_screen) in [
            (Locale::English, "Print Screen"),
            (Locale::Spanish, "Impr Pant"),
            (Locale::PortugueseBrazil, "Print Screen"),
        ] {
            let copy = locale.text(Text::TourDesktopShortcutsCopy);
            for control in ["Start/Menu", "LB/L1", "RB/R1", "F11", print_screen] {
                assert!(
                    copy.contains(control),
                    "missing {control} in {locale:?}: {copy}"
                );
            }
        }
    }

    #[test]
    fn controller_symbols_help_explains_button_positions_in_all_languages() {
        for (locale, xbox, playstation) in [
            (
                Locale::English,
                "on Xbox, A is on the bottom and X is on the left",
                "PlayStation Cross (✕) is on the bottom",
            ),
            (
                Locale::Spanish,
                "en Xbox, A está abajo y X a la izquierda",
                "cruz de PlayStation (✕) está abajo",
            ),
            (
                Locale::PortugueseBrazil,
                "no Xbox, A fica embaixo e X à esquerda",
                "cruz do PlayStation (✕) fica embaixo",
            ),
        ] {
            let help = locale.text(Text::ControllerSymbolsHelp);
            assert!(help.contains(xbox), "{locale:?}: {help}");
            assert!(help.contains(playstation), "{locale:?}: {help}");
        }
    }

    #[test]
    fn sdl_types_select_only_supported_icon_families() {
        use sdl3::gamepad::GamepadType;

        let context = egui::Context::default();
        auto_family(&context, Some(GamepadType::PS5));
        assert_eq!(
            gamepad_icons::family(&context),
            gamepad_icons::ControllerFamily::PlayStation
        );

        for gamepad_type in [GamepadType::Xbox360, GamepadType::XboxOne] {
            assert_eq!(
                family_for_sdl_type(gamepad_type),
                Some(gamepad_icons::ControllerFamily::Xbox)
            );
        }
        for gamepad_type in [GamepadType::PS3, GamepadType::PS4, GamepadType::PS5] {
            assert_eq!(
                family_for_sdl_type(gamepad_type),
                Some(gamepad_icons::ControllerFamily::PlayStation)
            );
        }
        for gamepad_type in [
            GamepadType::Unknown,
            GamepadType::Standard,
            GamepadType::NintendoSwitchPro,
            GamepadType::NintendoSwitchJoyconPair,
        ] {
            assert_eq!(family_for_sdl_type(gamepad_type), None);
        }
    }

    #[test]
    fn controller_diagram_uses_svg_buttons_for_both_families() {
        for family in [
            gamepad_icons::ControllerFamily::Xbox,
            gamepad_icons::ControllerFamily::PlayStation,
        ] {
            for step in Step::ALL {
                let context = egui::Context::default();
                crate::configure_desktop_style(&context);
                context.data_mut(|data| {
                    data.insert_temp(egui::Id::unique("gamepad-guide-controller-family"), family)
                });
                let mut output =
                    context.run_ui(egui::RawInput::default(), |ui| controller_diagram(ui, step));
                output.textures_delta.clear();
                assert!(context.tessellate(output.shapes, output.pixels_per_point).iter().any(|shape| matches!(&shape.primitive, egui::epaint::Primitive::Mesh(mesh) if mesh.texture_id != egui::TextureId::default())), "missing SVG buttons for {family:?}, {step:?}");
            }
        }
    }

    #[test]
    fn every_step_fits_the_minimum_window_in_all_languages_and_scales() {
        for locale in Locale::ALL {
            for scale in gamepad_desktop::preferences::UiScale::ALL {
                for step in Step::ALL {
                    let context = egui::Context::default();
                    crate::configure_desktop_style(&context);
                    let mut tour = Onboarding::default();
                    while tour.step != step {
                        tour.next();
                    }
                    let mut state = RuntimeState::default();
                    state.locale = locale;
                    state.ui_scale = scale;
                    state.onboarding = Some(tour);
                    let size = Vec2::new(660.0, 500.0) / scale.factor();
                    let mut bounds = Rect::NOTHING;
                    let mut output = context.run_ui(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                            ..Default::default()
                        },
                        |ui| {
                            show(ui, &state);
                            bounds = ui.min_rect();
                        },
                    );
                    output.textures_delta.clear();
                    assert!(
                        bounds.right() <= size.x + 1.0,
                        "horizontal overflow in {step:?}, {locale:?}, {scale:?}: {bounds:?}"
                    );
                    assert!(
                        bounds.bottom() <= size.y + 1.0,
                        "navigation outside window in {step:?}, {locale:?}, {scale:?}: {bounds:?}"
                    );
                }
            }
        }
    }
}
