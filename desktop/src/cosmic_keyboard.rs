//! COSMIC's existing org.freedesktop.DbusActivation actions; no keyboard UI of our own.
use std::{collections::HashMap, time::Duration};

const DESTINATION: &str = "com.system76.CosmicOSK";
const PATH: &str = "/com/system76/CosmicOSK";

pub(super) struct CosmicKeyboard {
    connection: zbus::Connection,
    open: bool,
}

fn is_cosmic_wayland(wayland: bool, desktop: &str) -> bool {
    wayland
        && desktop
            .split(':')
            .any(|name| name.eq_ignore_ascii_case("cosmic"))
}

impl CosmicKeyboard {
    pub(super) fn connect(wayland: bool, desktop: &str) -> Option<Self> {
        if !is_cosmic_wayland(wayland, desktop) {
            return None;
        }
        futures_lite::future::block_on(async {
            let connection = zbus::connection::Builder::session()
                .ok()?
                .method_timeout(Duration::from_millis(500))
                .build()
                .await
                .ok()?;
            let bus = zbus::fdo::DBusProxy::new(&connection).await.ok()?;
            // COSMIC starts its keyboard with the session. Do not launch another instance.
            if !bus
                .name_has_owner(DESTINATION.try_into().ok()?)
                .await
                .ok()?
            {
                return None;
            }
            Some(Self {
                connection,
                open: false,
            })
        })
    }

    pub(super) fn update(&mut self, open: bool) -> Result<(), String> {
        if open == self.open {
            return Ok(());
        }
        futures_lite::future::block_on(async {
            self.connection
                .call_method(
                    Some(DESTINATION),
                    PATH,
                    Some("org.freedesktop.DbusActivation"),
                    "ActivateAction",
                    &(
                        if open { "show" } else { "hide" },
                        Vec::<&str>::new(),
                        HashMap::<&str, zbus::zvariant::Value<'_>>::new(),
                    ),
                )
                .await
                .map_err(|error| error.to_string())?;
            self.open = open;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_cosmic_wayland_selects_native_keyboard() {
        assert!(is_cosmic_wayland(true, "COSMIC"));
        assert!(is_cosmic_wayland(true, "pop:COSMIC"));
        for desktop in ["GNOME", "KDE", "sway", "", "COSMIC-other"] {
            assert!(!is_cosmic_wayland(true, desktop));
        }
        assert!(!is_cosmic_wayland(false, "COSMIC"));
    }
    struct MockOsk(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

    #[zbus::interface(name = "org.freedesktop.DbusActivation")]
    impl MockOsk {
        fn activate_action(
            &self,
            action: &str,
            _parameters: Vec<&str>,
            _data: HashMap<&str, zbus::zvariant::Value<'_>>,
        ) {
            self.0.lock().unwrap().push(action.to_owned());
        }
    }

    #[test]
    #[ignore = "requires isolated dbus-run-session"]
    fn cosmic_actions_use_upstream_protocol_and_detect_service_loss() {
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        assert!(CosmicKeyboard::connect(true, "COSMIC").is_none());
        let server = futures_lite::future::block_on(async {
            zbus::connection::Builder::session()
                .unwrap()
                .name(DESTINATION)
                .unwrap()
                .serve_at(PATH, MockOsk(calls.clone()))
                .unwrap()
                .build()
                .await
                .unwrap()
        });
        assert!(CosmicKeyboard::connect(true, "GNOME").is_none());
        let mut keyboard = CosmicKeyboard::connect(true, "COSMIC").unwrap();
        keyboard.update(true).unwrap();
        keyboard.update(true).unwrap();
        keyboard.update(false).unwrap();
        assert_eq!(*calls.lock().unwrap(), ["show", "hide"]);
        drop(server);
        assert!(keyboard.update(true).is_err());
    }
}
