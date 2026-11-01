use std::{
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE,
        PeekMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
    },
};

use crate::windows_navigation::GamepadKeyFilter;

struct HookState {
    enabled: Arc<AtomicBool>,
    keys: GamepadKeyFilter,
}

thread_local! {
    static STATE: RefCell<Option<HookState>> = const { RefCell::new(None) };
}

pub(crate) struct GamepadNavigationHook {
    enabled: Arc<AtomicBool>,
    thread_id: u32,
    worker: Option<JoinHandle<()>>,
}

impl GamepadNavigationHook {
    pub(crate) fn new() -> Result<Self, String> {
        let enabled = Arc::new(AtomicBool::new(false));
        let worker_enabled = Arc::clone(&enabled);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("windows-gamepad-navigation".to_owned())
            .spawn(move || {
                STATE.with(|state| {
                    *state.borrow_mut() = Some(HookState {
                        enabled: worker_enabled,
                        keys: GamepadKeyFilter::default(),
                    });
                });
                let hook = unsafe {
                    GetModuleHandleW(None).and_then(|module| {
                        SetWindowsHookExW(
                            WH_KEYBOARD_LL,
                            Some(keyboard_hook),
                            Some(module.into()),
                            0,
                        )
                    })
                };
                let hook = match hook {
                    Ok(hook) => hook,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.to_string()));
                        return;
                    }
                };
                let mut message = MSG::default();
                // Create this thread's queue before publishing its ID for shutdown.
                unsafe {
                    let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
                }
                if ready_tx.send(Ok(unsafe { GetCurrentThreadId() })).is_ok() {
                    // Hooks are delivered on the installing thread. Keep it free of
                    // SDL polling, UI rendering, disk access, and blocking locks.
                    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
                        unsafe {
                            let _ = TranslateMessage(&message);
                            DispatchMessageW(&message);
                        }
                    }
                }
                unsafe {
                    let _ = UnhookWindowsHookEx(hook);
                }
            })
            .map_err(|error| error.to_string())?;
        let thread_id = match ready_rx
            .recv()
            .map_err(|error| error.to_string())
            .and_then(|result| result)
        {
            Ok(thread_id) => thread_id,
            Err(error) => {
                let _ = worker.join();
                return Err(error);
            }
        };
        Ok(Self {
            enabled,
            thread_id,
            worker: Some(worker),
        })
    }

    pub(crate) fn set_active(&self, active: bool) -> Result<(), String> {
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            return Err("Windows gamepad navigation hook stopped unexpectedly".to_owned());
        }
        self.enabled.store(active, Ordering::Release);
        Ok(())
    }

    pub(crate) fn is_active(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }
}

impl Drop for GamepadNavigationHook {
    fn drop(&mut self) {
        self.enabled.store(false, Ordering::Release);
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

unsafe extern "system" fn keyboard_hook(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    if code >= 0 && data.0 != 0 {
        let down = match message.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => Some(true),
            WM_KEYUP | WM_SYSKEYUP => Some(false),
            _ => None,
        };
        if let Some(down) = down {
            // Windows owns this pointer and guarantees KBDLLHOOKSTRUCT for
            // nonnegative hook codes. Do not retain it beyond the callback.
            let key = unsafe { (*(data.0 as *const KBDLLHOOKSTRUCT)).vkCode };
            let blocked = STATE.with(|state| {
                state.borrow_mut().as_mut().is_some_and(|state| {
                    state
                        .keys
                        .process(key, down, state.enabled.load(Ordering::Acquire))
                })
            });
            if blocked {
                return LRESULT(1);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, message, data) }
}
