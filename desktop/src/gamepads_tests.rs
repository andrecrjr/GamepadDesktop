use super::*;
use sdl3::joystick::{JoystickType, VirtualJoystickConnection, VirtualJoystickDescription};

fn connect(reader: &Gamepads, name: &str, mapped: bool) -> VirtualJoystickConnection {
    reader
        .joysticks
        .attach_virtual_joystick(
            VirtualJoystickDescription::new()
                .name(name)
                .joystick_type(if mapped {
                    JoystickType::Gamepad
                } else {
                    JoystickType::Unknown
                })
                .with_axes(vec![
                    Axis::LeftX,
                    Axis::LeftY,
                    Axis::RightX,
                    Axis::RightY,
                    Axis::TriggerLeft,
                    Axis::TriggerRight,
                ])
                .with_buttons(vec![
                    PadButton::South,
                    PadButton::East,
                    PadButton::West,
                    PadButton::North,
                    PadButton::Back,
                    PadButton::Guide,
                    PadButton::Start,
                    PadButton::LeftStick,
                    PadButton::RightStick,
                ]),
        )
        .unwrap()
}

#[test]
#[ignore = "initializes SDL; run serially"]
fn virtual_sdl_controller_normalizes_axes_buttons_and_disconnects() {
    let mut reader = Gamepads::new().unwrap();
    let connection = connect(&reader, "Virtual mapped controller", true);
    let id = connection.id();
    let joystick = reader.joysticks.open(id).unwrap();
    assert!(reader.poll(Some(id.raw())).unwrap().is_some());
    joystick.set_virtual_axis(0, i16::MAX).unwrap();
    joystick.set_virtual_axis(1, i16::MIN).unwrap();
    joystick.set_virtual_axis(3, i16::MAX).unwrap();
    joystick.set_virtual_button(0, true).unwrap();
    joystick.set_virtual_button(4, true).unwrap();
    joystick.set_virtual_button(8, true).unwrap();
    joystick.set_virtual_button(7, true).unwrap();
    let frame = reader.poll(Some(id.raw())).unwrap().unwrap();
    assert_eq!((frame.x, frame.y, frame.scroll), (1.0, -1.0, 1.0));
    assert!(frame.pressed(Button::South));
    assert!(frame.pressed(Button::Select));
    assert!(frame.pressed(Button::RightStick));
    assert!(frame.pressed(Button::LeftStick));
    drop(connection);
    assert!(reader.poll(Some(id.raw())).unwrap().is_none());
    assert!(reader.name().is_none());
    let reconnected = connect(&reader, "Virtual mapped controller", true);
    let frame = reader.poll(Some(reconnected.id().raw())).unwrap().unwrap();
    assert_eq!((frame.x, frame.y), (0.0, 0.0));
    assert!(!frame.pressed(Button::South));
}

#[test]
#[ignore = "initializes SDL; run serially"]
fn unknown_controller_becomes_usable_after_importing_a_saved_profile() {
    let mut reader = Gamepads::new().unwrap();
    let connection = connect(&reader, "Virtual unknown controller", false);
    let id = connection.id();
    assert!(reader.poll(Some(id.raw())).unwrap().is_none());
    let device = reader
        .devices
        .iter()
        .find(|device| device.id == id.raw())
        .unwrap();
    assert!(!device.mapped);
    let mapping = format!(
        "{},Virtual unknown controller,a:b0,leftx:a0,lefty:a1,righty:a3,",
        device.guid
    );
    let directory =
        std::env::temp_dir().join(format!("gamepad-sdl-profile-{}", std::process::id()));
    let path = directory.join("gamecontrollerdb.txt");
    reader.profile_path = Some(path.clone());
    reader.import_mapping(&mapping).unwrap();
    assert!(reader.poll(Some(id.raw())).unwrap().is_some());
    let joystick = reader.joysticks.open(id).unwrap();
    joystick.set_virtual_button(0, true).unwrap();
    assert!(
        reader
            .poll(Some(id.raw()))
            .unwrap()
            .unwrap()
            .pressed(Button::South)
    );
    assert!(std::fs::read_to_string(path).unwrap().contains(&mapping));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires a physical gamepad connected and access to it"]
fn physical_controller_is_detected_and_read_by_sdl() {
    let mut reader = Gamepads::new().unwrap();
    let frame = reader
        .poll(None)
        .unwrap()
        .expect("connect a mapped physical gamepad");
    assert!(reader.name().is_some());
    assert!(!reader.devices.is_empty());
    assert!(
        [frame.x, frame.y, frame.scroll]
            .iter()
            .all(|axis| axis.is_finite() && axis.abs() <= 1.0)
    );
    eprintln!("SDL3 detected: {}", reader.name().unwrap());
}
