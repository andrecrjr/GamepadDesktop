use std::{
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use windows::{
    UI::ViewManagement::Core::{CoreInputView, CoreInputViewKind},
    Win32::{
        Foundation::{HWND, RPC_E_CHANGED_MODE},
        System::Com::{
            CLSCTX_ALL, CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance,
            CoInitializeEx, CoUninitialize,
        },
        UI::{
            Shell::{FrameworkInputPane, IFrameworkInputPane},
            WindowsAndMessaging::GetDesktopWindow,
        },
    },
    core::{GUID, HRESULT, IUnknown, IUnknown_Vtbl, Interface},
};

use crate::{
    keyboard::KeyboardSession,
    windows_navigation::{KeyboardBounds, TouchKeyboardState},
};

// ITipInvocation is the internal touch-keyboard launcher used by desktop
// utilities. Unlike InputPane.TryShow, it can request input for another app.
#[repr(transparent)]
#[derive(Clone)]
struct TipInvocation(IUnknown);

#[repr(C)]
struct TipInvocationVtable {
    base: IUnknown_Vtbl,
    toggle: unsafe extern "system" fn(*mut std::ffi::c_void, HWND) -> HRESULT,
}

// SAFETY: the wrapper has IUnknown's layout and the ITipInvocation
// IID/vtable used by Windows desktop clients: IUnknown then Toggle(HWND).
unsafe impl Interface for TipInvocation {
    type Vtable = TipInvocationVtable;
    const IID: GUID = GUID::from_u128(0x37c994e7_432b_4834_a2f7_dce1f13b834b);
}

#[derive(Default)]
pub(crate) struct WindowsTouchKeyboard {
    state: TouchKeyboardState,
    pub(crate) bounds: Option<KeyboardBounds>,
    pane: Option<IFrameworkInputPane>,
    core_input_view: Option<CoreInputView>,
    gamepad_view_active: bool,
    checked_availability: bool,
    com_ready: bool,
    uninitialize: bool,
    last_check: Option<Instant>,
    transition_since: Option<Instant>,
    launched: bool,
}

impl WindowsTouchKeyboard {
    pub(crate) fn visible(&self) -> bool {
        self.state.visible
    }

    pub(crate) fn active(&self) -> bool {
        self.state.requested || self.state.visible
    }

    pub(crate) fn uses_native_gamepad(&self) -> bool {
        self.state.gamepad_session && self.gamepad_view_active
    }

    pub(crate) fn gamepad_session_active(&self) -> bool {
        self.state.gamepad_session
    }

    fn initialize(&mut self) -> windows::core::Result<()> {
        if !self.com_ready {
            let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            if result != RPC_E_CHANGED_MODE {
                result.ok()?;
                self.uninitialize = true;
            }
            self.com_ready = true;
        }
        if self.pane.is_none() {
            self.pane = Some(unsafe { CoCreateInstance(&FrameworkInputPane, None, CLSCTX_ALL)? });
        }
        if self.core_input_view.is_none() {
            self.core_input_view = CoreInputView::GetForCurrentView().ok();
        }
        Ok(())
    }

    fn request_visibility(&mut self, visible: bool) -> Result<(), String> {
        if visible {
            self.gamepad_view_active = self.core_input_view.as_ref().is_some_and(|view| {
                view.TryShowWithKind(CoreInputViewKind::Gamepad)
                    .unwrap_or(false)
            });
            if !self.gamepad_view_active {
                Self::toggle().map_err(|error| error.to_string())?;
            }
        } else if self.gamepad_view_active {
            let hidden = self
                .core_input_view
                .as_ref()
                .is_some_and(|view| view.TryHide().unwrap_or(false));
            if !hidden {
                Self::toggle().map_err(|error| error.to_string())?;
            }
            self.gamepad_view_active = false;
        } else {
            Self::toggle().map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn toggle() -> windows::core::Result<()> {
        let host = GUID::from_u128(0x4ce576fa_83dc_4f88_951c_9d0782b4e376);
        let invocation: TipInvocation =
            unsafe { CoCreateInstance(&host, None, CLSCTX_LOCAL_SERVER)? };
        unsafe { (invocation.vtable().toggle)(invocation.as_raw(), GetDesktopWindow()).ok() }
    }

    fn launch() -> Result<(), String> {
        let common = std::env::var_os("CommonProgramFiles")
            .map(PathBuf::from)
            .ok_or("Windows CommonProgramFiles directory is unavailable")?;
        Command::new(common.join(r"microsoft shared\ink\TabTip.exe"))
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub(crate) fn update(&mut self, session: &mut KeyboardSession) -> Result<(), String> {
        if !session.open && self.pane.is_none() {
            if self.checked_availability {
                return Ok(());
            }
            self.checked_availability = true;
            // Observe keyboards opened by Windows itself, too. Unavailable
            // native input must not prevent the mouse runtime from starting.
            if self.initialize().is_err() {
                return Ok(());
            }
        }
        let changed = session.open != self.state.requested;
        let now = Instant::now();
        if !changed
            && self
                .last_check
                .is_some_and(|last| now.duration_since(last) < Duration::from_millis(50))
        {
            return Ok(());
        }
        self.last_check = Some(now);
        self.initialize().map_err(|error| error.to_string())?;
        let bounds = unsafe {
            self.pane
                .as_ref()
                .expect("initialized input pane")
                .Location()
        }
        .map_err(|error| error.to_string())?;
        self.bounds = KeyboardBounds::new(bounds.left, bounds.top, bounds.right, bounds.bottom);
        let visible = self.bounds.is_some();
        let requested = session.open;
        let command = self.state.observe(requested, visible);
        session.open = self.state.requested;
        if session.open != requested {
            session.status = Some(
                if session.open {
                    "Teclado do Windows aberto · R3 fecha."
                } else {
                    "Teclado visual fechado."
                }
                .to_owned(),
            );
        }
        if changed {
            self.transition_since = Some(now);
            self.launched = false;
        }
        if command.is_some() || self.state.waiting() {
            let since = *self.transition_since.get_or_insert(now);
            if now.duration_since(since) > Duration::from_secs(5) {
                return Err(
                    "Windows touch keyboard did not complete its open/close request".to_owned(),
                );
            }
            if let Some(target) = command {
                match self.request_visibility(target) {
                    Ok(()) => self.state.command_sent(target),
                    Err(error) => {
                        if target && !self.launched {
                            Self::launch()?;
                            self.launched = true;
                        } else if !target {
                            return Err(error.to_string());
                        }
                    }
                }
            }
        } else {
            self.transition_since = None;
            self.launched = false;
        }
        Ok(())
    }
}

impl Drop for WindowsTouchKeyboard {
    fn drop(&mut self) {
        // Release COM interfaces before balancing our own CoInitializeEx call.
        self.pane = None;
        if self.uninitialize {
            unsafe { CoUninitialize() };
        }
    }
}
