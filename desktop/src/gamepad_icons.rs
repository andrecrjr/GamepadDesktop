use eframe::egui::{self, Image, Ui};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControllerFamily {
    #[default]
    Xbox,
    PlayStation,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Icon {
    LeftStick,
    RightStickUp,
    RightStickDown,
    South,
    East,
    West,
    North,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    L1,
    R1,
    L3,
    R3,
    Menu,
    View,
}

pub fn show_family_selector(ui: &mut Ui, label: &str, help: &str) {
    let id = egui::Id::unique("gamepad-guide-controller-family");
    let mut family = ui
        .ctx()
        .data_mut(|data| data.get_temp::<ControllerFamily>(id).unwrap_or_default());

    ui.horizontal_wrapped(|ui| {
        ui.strong(label);
        ui.selectable_value(&mut family, ControllerFamily::Xbox, "Xbox");
        ui.selectable_value(&mut family, ControllerFamily::PlayStation, "PlayStation");
    });
    ui.small(help);
    ui.ctx().data_mut(|data| data.insert_temp(id, family));
}

pub fn family(context: &egui::Context) -> ControllerFamily {
    context.data_mut(|data| {
        data.get_temp::<ControllerFamily>(egui::Id::unique("gamepad-guide-controller-family"))
            .unwrap_or_default()
    })
}

pub fn show_shortcut_icons(ui: &mut Ui, family: ControllerFamily, button: &str) {
    let icons: &[Icon] = match button {
        "R3 · pressionar analógico direito" => &[Icon::R3],
        "L3 · pressionar analógico esquerdo" => &[Icon::L3],
        "Analógico esquerdo" => &[Icon::LeftStick],
        "Analógico direito ↕" => &[Icon::RightStickUp, Icon::RightStickDown],
        "Inferior · A / ✕ / B no Switch" | "A / botão inferior" => &[Icon::South],
        "Direito · B / ○ / A no Switch" | "B / botão direito" => &[Icon::East],
        "Esquerdo · X / □ / Y no Switch" | "X / botão esquerdo" => &[Icon::West],
        "Superior · Y / △ / X no Switch" | "Y / botão superior" => &[Icon::North],
        "Direcional" => &[
            Icon::DpadUp,
            Icon::DpadDown,
            Icon::DpadLeft,
            Icon::DpadRight,
        ],
        "Direcional / analógico esquerdo" => &[
            Icon::DpadUp,
            Icon::DpadDown,
            Icon::DpadLeft,
            Icon::DpadRight,
            Icon::LeftStick,
        ],
        "LB / L1" | "L1" => &[Icon::L1],
        "RB / R1" | "R1" => &[Icon::R1],
        "Start + Select / Menu + View" => &[Icon::Menu, Icon::View],
        "Select · dois toques rápidos" => &[Icon::View],
        "Start / Menu / +" => &[Icon::Menu],
        _ => &[],
    };

    for icon in icons {
        show_icon(ui, family, *icon);
    }
}

fn show_icon(ui: &mut Ui, family: ControllerFamily, icon: Icon) {
    let image = image_source(family, icon);
    ui.add(Image::new(image).fit_to_exact_size(egui::vec2(24.0, 24.0)));
}

pub(crate) fn image_source(family: ControllerFamily, icon: Icon) -> egui::ImageSource<'static> {
    match (family, icon) {
        (ControllerFamily::Xbox, Icon::LeftStick) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_stick_l.svg")
        }
        (ControllerFamily::PlayStation, Icon::LeftStick) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_stick_l.svg")
        }
        (ControllerFamily::Xbox, Icon::RightStickUp) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_stick_r_up.svg")
        }
        (ControllerFamily::PlayStation, Icon::RightStickUp) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_stick_r_up.svg")
        }
        (ControllerFamily::Xbox, Icon::RightStickDown) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_stick_r_down.svg")
        }
        (ControllerFamily::PlayStation, Icon::RightStickDown) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_stick_r_down.svg")
        }
        (ControllerFamily::Xbox, Icon::South) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_button_a.svg")
        }
        (ControllerFamily::PlayStation, Icon::South) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_button_cross.svg")
        }
        (ControllerFamily::Xbox, Icon::East) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_button_b.svg")
        }
        (ControllerFamily::PlayStation, Icon::East) => {
            egui::include_image!(
                "../assets/gamepad-icons/playstation/playstation_button_circle.svg"
            )
        }
        (ControllerFamily::Xbox, Icon::West) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_button_x.svg")
        }
        (ControllerFamily::PlayStation, Icon::West) => {
            egui::include_image!(
                "../assets/gamepad-icons/playstation/playstation_button_square.svg"
            )
        }
        (ControllerFamily::Xbox, Icon::North) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_button_y.svg")
        }
        (ControllerFamily::PlayStation, Icon::North) => {
            egui::include_image!(
                "../assets/gamepad-icons/playstation/playstation_button_triangle.svg"
            )
        }
        (ControllerFamily::Xbox, Icon::DpadUp) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_dpad_up.svg")
        }
        (ControllerFamily::PlayStation, Icon::DpadUp) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_dpad_up.svg")
        }
        (ControllerFamily::Xbox, Icon::DpadDown) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_dpad_down.svg")
        }
        (ControllerFamily::PlayStation, Icon::DpadDown) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_dpad_down.svg")
        }
        (ControllerFamily::Xbox, Icon::DpadLeft) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_dpad_left.svg")
        }
        (ControllerFamily::PlayStation, Icon::DpadLeft) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_dpad_left.svg")
        }
        (ControllerFamily::Xbox, Icon::DpadRight) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_dpad_right.svg")
        }
        (ControllerFamily::PlayStation, Icon::DpadRight) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_dpad_right.svg")
        }
        (ControllerFamily::Xbox, Icon::L1) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_lb.svg")
        }
        (ControllerFamily::PlayStation, Icon::L1) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_trigger_l1.svg")
        }
        (ControllerFamily::Xbox, Icon::R1) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_rb.svg")
        }
        (ControllerFamily::PlayStation, Icon::R1) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_trigger_r1.svg")
        }
        (ControllerFamily::Xbox, Icon::L3) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_stick_l_press.svg")
        }
        (ControllerFamily::PlayStation, Icon::L3) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_button_l3.svg")
        }
        (ControllerFamily::Xbox, Icon::R3) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_stick_r_press.svg")
        }
        (ControllerFamily::PlayStation, Icon::R3) => {
            egui::include_image!("../assets/gamepad-icons/playstation/playstation_button_r3.svg")
        }
        (ControllerFamily::Xbox, Icon::Menu) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_button_menu.svg")
        }
        (ControllerFamily::PlayStation, Icon::Menu) => {
            egui::include_image!(
                "../assets/gamepad-icons/playstation/playstation5_button_options.svg"
            )
        }
        (ControllerFamily::Xbox, Icon::View) => {
            egui::include_image!("../assets/gamepad-icons/xbox/xbox_button_view.svg")
        }
        (ControllerFamily::PlayStation, Icon::View) => {
            egui::include_image!(
                "../assets/gamepad-icons/playstation/playstation5_button_create.svg"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icons_load_for_both_controller_families() {
        let context = egui::Context::default();
        egui_extras::install_image_loaders(&context);
        let icons = [
            Icon::LeftStick,
            Icon::RightStickUp,
            Icon::RightStickDown,
            Icon::South,
            Icon::East,
            Icon::West,
            Icon::North,
            Icon::DpadUp,
            Icon::DpadDown,
            Icon::DpadLeft,
            Icon::DpadRight,
            Icon::L1,
            Icon::R1,
            Icon::L3,
            Icon::R3,
            Icon::Menu,
            Icon::View,
        ];

        for family in [ControllerFamily::Xbox, ControllerFamily::PlayStation] {
            for icon in icons {
                let source = image_source(family, icon);
                let image = Image::new(source).fit_to_exact_size(egui::vec2(24.0, 24.0));
                assert!(
                    image
                        .load_for_size(&context, egui::vec2(24.0, 24.0))
                        .is_ok(),
                    "failed to load {family:?} {icon:?}"
                );
            }
        }
    }
}
