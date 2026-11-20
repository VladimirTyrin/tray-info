use std::cell::{Cell, RefCell};

use anyhow::{Context, Result, bail};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Shell::NIN_SELECT;
use windows::Win32::UI::WindowsAndMessaging::{
    ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetMessageW, MSG, MSGFLT_ALLOW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW,
    SetCoalescableTimer, TranslateMessage, WINDOW_EX_STYLE, WM_CONTEXTMENU, WM_DESTROY,
    WM_DISPLAYCHANGE, WM_SETTINGCHANGE, WM_TIMER, WNDCLASSW, WS_OVERLAPPED,
};
use windows::core::w;

use crate::app::App;
use crate::autostart_request::AutostartRequest;
use crate::tray::{MenuCommand, TRAY_CALLBACK_MESSAGE, show_popup_menu};
use crate::win::{
    Autostart, InstanceWait, SingleInstance, WAIT_FOR_PREVIOUS_FLAG, debug_log, relaunch_elevated,
};

const CPU_TIMER: usize = 1;
const BLUETOOTH_TIMER: usize = 2;
const CPU_INTERVAL_MS: u32 = 2_000;
/// Battery levels change slowly and Windows itself refreshes them rarely.
const BLUETOOTH_INTERVAL_MS: u32 = 15_000;
/// Lets Windows batch our wake-ups with other timers to save power; the values are not urgent.
const TIMER_TOLERANCE_MS: u32 = 500;

/// Keyboard selection of a tray icon (`NIN_SELECT | NINF_KEY`).
const NIN_KEYSELECT: u32 = NIN_SELECT | 0x1;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
}

/// Runs the tray app until the user chooses Exit, or applies an autostart change on behalf of
/// the unelevated app and exits.
pub fn run() -> Result<()> {
    if let Some(request) = AutostartRequest::from_args(std::env::args())? {
        return with_com(|| {
            Autostart::for_user(request.user)?
                .set_enabled(request.enabled)
                .context("cannot change autostart")
        });
    }
    let wait = if std::env::args().any(|argument| argument == WAIT_FOR_PREVIOUS_FLAG) {
        InstanceWait::ForPrevious
    } else {
        InstanceWait::NoWait
    };
    let Some(instance) = SingleInstance::acquire(wait)? else {
        return Ok(());
    };

    // Crisp icons at 125–200% scaling instead of bitmap-stretched 16 px ones.
    if let Err(error) =
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
    {
        debug_log(&format!("DPI awareness not changed: {error}"));
    }
    with_com(|| run_window(instance))
}

fn with_com<R>(action: impl FnOnce() -> Result<R>) -> Result<R> {
    // STA: the shell APIs (ShellExecute, tray) expect it; WMI and Task Scheduler work from
    // either apartment.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .context("CoInitializeEx failed")?;
    let result = action();
    unsafe { CoUninitialize() };
    result
}

fn run_window(instance: SingleInstance) -> Result<()> {
    TASKBAR_CREATED.set(unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) });
    let hwnd = create_window()?;
    allow_shell_messages(hwnd);

    let mut app = App::new(hwnd, instance);
    app.refresh_cpu();
    app.refresh_bluetooth();
    APP.with_borrow_mut(|slot| *slot = Some(app));

    start_timer(hwnd, CPU_TIMER, CPU_INTERVAL_MS)?;
    start_timer(hwnd, BLUETOOTH_TIMER, BLUETOOTH_INTERVAL_MS)?;

    let mut message = MSG::default();
    loop {
        match unsafe { GetMessageW(&mut message, None, 0, 0) }.0 {
            0 => return Ok(()),
            -1 => bail!(
                "GetMessageW failed: {}",
                windows::core::Error::from_thread()
            ),
            _ => unsafe {
                // TranslateMessage only reports whether it produced a character message.
                let _translated = TranslateMessage(&message);
                DispatchMessageW(&message);
            },
        }
    }
}

fn create_window() -> Result<HWND> {
    let module = unsafe { GetModuleHandleW(None) }.context("GetModuleHandleW failed")?;
    let class_name = w!("tray-info.window");
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: module.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        bail!(
            "RegisterClassW failed: {}",
            windows::core::Error::from_thread()
        );
    }
    // A regular top-level window that is never shown: message-only windows would not receive
    // the TaskbarCreated broadcast needed to restore icons after Explorer restarts.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("tray-info"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(module.into()),
            None,
        )
    }
    .context("CreateWindowExW failed")
}

/// When running elevated, UIPI drops messages from the medium-integrity Explorer: icon clicks
/// and the TaskbarCreated broadcast would never arrive. Whitelist exactly those two.
fn allow_shell_messages(hwnd: HWND) {
    for message in [TRAY_CALLBACK_MESSAGE, TASKBAR_CREATED.get()] {
        if let Err(error) =
            unsafe { ChangeWindowMessageFilterEx(hwnd, message, MSGFLT_ALLOW, None) }
        {
            debug_log(&format!(
                "ChangeWindowMessageFilterEx({message:#x}) failed: {error}"
            ));
        }
    }
}

fn start_timer(hwnd: HWND, id: usize, interval_ms: u32) -> Result<()> {
    if unsafe { SetCoalescableTimer(Some(hwnd), id, interval_ms, None, TIMER_TOLERANCE_MS) } == 0 {
        bail!(
            "SetCoalescableTimer failed: {}",
            windows::core::Error::from_thread()
        );
    }
    Ok(())
}

/// Runs `action` on the app unless it is already borrowed. A nested call can happen when a
/// COM call made by the app pumps messages while it waits; skipping that tick is harmless.
fn with_app<R>(action: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|slot| {
        let mut slot = slot.try_borrow_mut().ok()?;
        slot.as_mut().map(action)
    })
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_TIMER => {
            match wparam.0 {
                CPU_TIMER => with_app(App::refresh_cpu),
                BLUETOOTH_TIMER => with_app(App::refresh_bluetooth),
                _ => None,
            };
            LRESULT(0)
        }
        TRAY_CALLBACK_MESSAGE => {
            let event = (lparam.0 as u32) & 0xFFFF;
            if event == WM_CONTEXTMENU || event == NIN_SELECT || event == NIN_KEYSELECT {
                // With NOTIFYICON_VERSION_4 the anchor point comes packed in wParam.
                let x = i32::from(wparam.0 as u16 as i16);
                let y = i32::from((wparam.0 >> 16) as u16 as i16);
                on_menu_requested(hwnd, x, y);
            }
            LRESULT(0)
        }
        WM_SETTINGCHANGE | WM_DISPLAYCHANGE => {
            with_app(App::redraw);
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        WM_DESTROY => {
            // Drop outside the borrow: removing icons talks to the shell.
            let app = APP.with_borrow_mut(Option::take);
            drop(app);
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ if message != 0 && message == TASKBAR_CREATED.get() => {
            with_app(|app| app.on_taskbar_created());
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn on_menu_requested(hwnd: HWND, x: i32, y: i32) {
    // Build the entries, then release the app: the menu's modal loop keeps dispatching timers.
    let Some(entries) = with_app(|app| app.menu()) else {
        return;
    };
    let command = match show_popup_menu(hwnd, x, y, &entries) {
        Ok(command) => command,
        Err(error) => {
            debug_log(&format!("context menu failed: {error:#}"));
            return;
        }
    };
    match command {
        Some(MenuCommand::Exit) => destroy(hwnd),
        Some(MenuCommand::StartWithWindows) => {
            with_app(|app| app.toggle_autostart());
        }
        Some(MenuCommand::RestartAsAdministrator) => match relaunch_elevated() {
            Ok(()) => destroy(hwnd),
            // Most often the user declined the UAC prompt; keep running as is.
            Err(error) => debug_log(&format!("elevated restart failed: {error:#}")),
        },
        None => {}
    }
}

fn destroy(hwnd: HWND) {
    if let Err(error) = unsafe { DestroyWindow(hwnd) } {
        debug_log(&format!("DestroyWindow failed: {error}"));
        unsafe { PostQuitMessage(0) };
    }
}
