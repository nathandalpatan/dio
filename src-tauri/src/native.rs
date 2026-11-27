use crate::model::{normalize_with, trim_word_at_point, Limits, Rect, Selection};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};
use tauri::{AppHandle, Manager};
use tokio::sync::oneshot;
use windows::{
    core::Interface,
    Win32::{
        Foundation::{HWND, POINT},
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_MULTITHREADED,
            },
            Ole::SafeArrayDestroy,
        },
        UI::{
            Accessibility::{
                CUIAutomation, IUIAutomation, IUIAutomationTextPattern, IUIAutomationTextRange,
                TextUnit_Word, UIA_TextPatternId,
            },
            WindowsAndMessaging::{GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId},
        },
    },
};

struct Request {
    foreground: isize,
    limits: Limits,
    reply: oneshot::Sender<Result<Selection, String>>,
}
/// UI Automation saw a terminal control, so the clipboard fallback must not send Ctrl+C.
const TERMINAL: &str = "The terminal does not expose its selection.";
pub struct SelectionReader {
    sender: SyncSender<Request>,
}
impl SelectionReader {
    pub fn new() -> Self {
        let (sender, receiver) = sync_channel::<Request>(1);
        std::thread::spawn(move || unsafe {
            let initialization = CoInitializeEx(None, COINIT_MULTITHREADED);
            let initialized = initialization.is_ok();
            let automation = initialization.ok().and_then(|_| {
                CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            });
            for request in receiver {
                if request.reply.is_closed() {
                    continue;
                }
                let result = automation
                    .as_ref()
                    .map_err(|e| format!("Accessibility is unavailable: {e}"))
                    .and_then(|automation| {
                        read_selection(automation, request.foreground, request.limits)
                    });
                let _ = request.reply.send(result);
            }
            if initialized {
                CoUninitialize();
            }
        });
        Self { sender }
    }
    pub async fn read(
        &self,
        foreground: isize,
        owner: isize,
        limits: Limits,
    ) -> Result<Selection, String> {
        let (reply, result) = oneshot::channel();
        let accessibility = match self.sender.try_send(Request {
            foreground,
            limits,
            reply,
        }) {
            Ok(()) => {
                match tokio::time::timeout(std::time::Duration::from_millis(1200), result).await {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => Err("Accessibility service stopped.".into()),
                    Err(_) => Err("Accessibility timed out.".into()),
                }
            }
            Err(_) => Err("Accessibility request is still pending.".into()),
        };
        match accessibility {
            Ok(selection) => {
                eprintln!("DIO capture: UI Automation succeeded");
                Ok(selection)
            }
            Err(error) if error == "DIO does not read password fields." => {
                eprintln!("DIO capture: UI Automation failed: {error}; clipboard skipped");
                Err(error)
            }
            Err(error) => {
                let terminal = error == TERMINAL;
                eprintln!("DIO capture: UI Automation failed: {error}");
                let result = tokio::task::spawn_blocking(move || {
                    crate::clipboard::capture(foreground, owner, limits, terminal)
                })
                .await
                .map_err(|e| e.to_string())?;
                match &result {
                    Ok(_) => eprintln!("DIO capture: clipboard succeeded"),
                    Err(error) => eprintln!("DIO capture: clipboard failed: {error}"),
                }
                result
            }
        }
    }
}
pub fn foreground() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}
/// Seconds since the last keyboard or mouse input in this session, or 0 if unknown.
pub fn idle_seconds() -> u64 {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if unsafe { GetLastInputInfo(&mut info) }.as_bool() {
        // Both are milliseconds since boot and wrap after 49.7 days, so subtract with wrapping.
        (unsafe { GetTickCount() }.wrapping_sub(info.dwTime) / 1000) as u64
    } else {
        0
    }
}
unsafe fn read_selection(
    automation: &IUIAutomation,
    original: isize,
    limits: Limits,
) -> Result<Selection, String> {
    if foreground() != original {
        return Err("The active application changed. Select the word again.".into());
    }
    let mut process = 0;
    GetWindowThreadProcessId(HWND(original as *mut _), Some(&mut process));
    let mut element = automation
        .GetFocusedElement()
        .map_err(|_| "Cannot access the selected application.")?;
    let walker = automation.ControlViewWalker().map_err(|e| e.to_string())?;
    for _ in 0..6 {
        if element.CurrentProcessId().map_err(|e| e.to_string())? as u32 != process {
            break;
        }
        let terminal = element
            .CurrentAutomationId()
            .ok()
            .is_some_and(|id| id.to_string().to_ascii_lowercase().contains("terminal"))
            || element.CurrentClassName().ok().is_some_and(|name| {
                let name = name.to_string();
                name == "TermControl" || name.to_ascii_lowercase().contains("xterm")
            });
        if element
            .CurrentIsPassword()
            .map_err(|e| e.to_string())?
            .as_bool()
        {
            return Err("DIO does not read password fields.".into());
        }
        if let Ok(pattern) = element
            .GetCurrentPattern(UIA_TextPatternId)
            .and_then(|p| p.cast::<IUIAutomationTextPattern>())
        {
            if let Ok(ranges) = pattern.GetSelection() {
                let count = ranges.Length().map_err(|e| e.to_string())?;
                if count > 1 {
                    return Err("Select one continuous word or expression.".into());
                }
                if count == 1 {
                    let range = ranges.GetElement(0).map_err(|e| e.to_string())?;
                    let text = range
                        .GetText(limits.max_chars.saturating_mul(2).saturating_add(2) as i32)
                        .map_err(|e| e.to_string())?
                        .to_string();
                    if !text.trim().is_empty() {
                        let text = normalize_with(&text, limits)?;
                        let bounds = range_bounds(&range);
                        if foreground() != original {
                            return Err("The active application changed. Try again.".into());
                        }
                        return Ok(Selection { text, bounds });
                    }
                }
            }
            // An app that handles the mouse itself, such as Claude Code, leaves the terminal with
            // no selection, but the pointer still rests on the word the user double-clicked.
            if terminal && !limits.multiline {
                if let Some(selection) = word_under_pointer(&pattern, limits) {
                    if foreground() != original {
                        return Err("The active application changed. Try again.".into());
                    }
                    return Ok(selection);
                }
            }
        }
        if terminal {
            return Err(TERMINAL.into());
        }
        element = match walker.GetParentElement(&element) {
            Ok(parent) => parent,
            Err(_) => break,
        };
    }
    Err("This application does not expose highlighted text to Windows accessibility. Try selecting text in another application.".into())
}

unsafe fn word_under_pointer(
    pattern: &IUIAutomationTextPattern,
    limits: Limits,
) -> Option<Selection> {
    let mut point = POINT::default();
    GetCursorPos(&mut point).ok()?;
    let range = pattern.RangeFromPoint(point).ok()?;
    range.ExpandToEnclosingUnit(TextUnit_Word).ok()?;
    let text = range
        .GetText(limits.max_chars.saturating_mul(2).saturating_add(2) as i32)
        .ok()?
        .to_string();
    let text = normalize_with(trim_word_at_point(&text), limits).ok()?;
    Some(Selection {
        text,
        bounds: range_bounds(&range),
    })
}

unsafe fn range_bounds(range: &IUIAutomationTextRange) -> Option<Rect> {
    use windows::Win32::System::Ole::{
        SafeArrayAccessData, SafeArrayGetLBound, SafeArrayGetUBound, SafeArrayUnaccessData,
    };
    let array = range.GetBoundingRectangles().ok()?;
    let mut result = None;
    if !array.is_null()
        && SafeArrayGetLBound(array, 1)
            .ok()
            .zip(SafeArrayGetUBound(array, 1).ok())
            .is_some_and(|(lower, upper)| upper - lower >= 3)
    {
        let mut data = std::ptr::null_mut();
        if SafeArrayAccessData(array, &mut data).is_ok() {
            let values = std::slice::from_raw_parts(data as *const f64, 4);
            if values.iter().all(|v| v.is_finite()) && values[2] > 0.0 && values[3] > 0.0 {
                result = Some(Rect {
                    x: values[0],
                    y: values[1],
                    width: values[2],
                    height: values[3],
                });
            }
            let _ = SafeArrayUnaccessData(array);
        }
    }
    if !array.is_null() {
        let _ = SafeArrayDestroy(array);
    }
    result
}

#[derive(Clone, Copy)]
enum InputEvent {
    Click { x: i32, y: i32, middle: bool },
    Escape,
}
static INPUT_EVENTS: OnceLock<SyncSender<InputEvent>> = OnceLock::new();

unsafe extern "system" fn mouse_hook(
    code: i32,
    key: windows::Win32::Foundation::WPARAM,
    data: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_NCLBUTTONDOWN,
        WM_NCMBUTTONDOWN, WM_RBUTTONDOWN,
    };
    if code >= 0
        && [
            WM_LBUTTONDOWN,
            WM_MBUTTONDOWN,
            WM_NCLBUTTONDOWN,
            WM_NCMBUTTONDOWN,
            WM_RBUTTONDOWN,
        ]
        .contains(&(key.0 as u32))
    {
        let point = (*(data.0 as *const MSLLHOOKSTRUCT)).pt;
        let _ = INPUT_EVENTS.get().map(|sender| {
            sender.try_send(InputEvent::Click {
                x: point.x,
                y: point.y,
                middle: key.0 as u32 == WM_MBUTTONDOWN || key.0 as u32 == WM_NCMBUTTONDOWN,
            })
        });
    }
    CallNextHookEx(None, code, key, data)
}

unsafe extern "system" fn keyboard_hook(
    code: i32,
    key: windows::Win32::Foundation::WPARAM,
    data: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_SYSKEYDOWN,
    };
    if code >= 0
        && (key.0 as u32 == WM_KEYDOWN || key.0 as u32 == WM_SYSKEYDOWN)
        && (*(data.0 as *const KBDLLHOOKSTRUCT)).vkCode == 0x1b
        && INPUT_EVENTS
            .get()
            .is_some_and(|sender| sender.try_send(InputEvent::Escape).is_ok())
    {
        return windows::Win32::Foundation::LRESULT(1);
    }
    CallNextHookEx(None, code, key, data)
}

pub struct Hooks {
    pub middle: Arc<AtomicBool>,
    pub visible: Arc<AtomicBool>,
    pub error: Arc<std::sync::Mutex<Option<String>>>,
}
impl Hooks {
    pub fn new(app: AppHandle, middle_enabled: bool) -> Self {
        let middle = Arc::new(AtomicBool::new(middle_enabled));
        let visible = Arc::new(AtomicBool::new(false));
        let error = Arc::new(std::sync::Mutex::new(None));
        let (sender, receiver) = sync_channel(32);
        let _ = INPUT_EVENTS.set(sender);
        let (m, v, e) = (middle.clone(), visible.clone(), error.clone());
        std::thread::spawn(move || {
            use windows::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, PeekMessageW, SetWindowsHookExW, TranslateMessage,
                UnhookWindowsHookEx, MSG, PM_REMOVE, WH_KEYBOARD_LL, WH_MOUSE_LL,
            };
            use windows::Win32::{Foundation::HINSTANCE, System::LibraryLoader::GetModuleHandleW};
            let mut mouse = None;
            let mut keyboard = None;
            let module = unsafe {
                GetModuleHandleW(None)
                    .ok()
                    .map(|handle| HINSTANCE(handle.0))
            };
            let mut mouse_failed = false;
            let mut keyboard_failed = false;
            loop {
                let enabled = m.load(Ordering::Relaxed);
                let showing = v.load(Ordering::Relaxed);
                unsafe {
                    if (enabled || showing) && mouse.is_none() && !mouse_failed {
                        match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0) {
                            Ok(hook) => {
                                mouse = Some(hook);
                                *e.lock().unwrap() = None;
                            }
                            Err(err) => {
                                mouse_failed = true;
                                *e.lock().unwrap() =
                                    Some(format!("Mouse monitoring is unavailable: {err}"));
                            }
                        }
                    } else if !enabled && !showing {
                        if let Some(hook) = mouse.take() {
                            let _ = UnhookWindowsHookEx(hook);
                        }
                        mouse_failed = false;
                    }
                    if showing && keyboard.is_none() && !keyboard_failed {
                        match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), module, 0) {
                            Ok(hook) => keyboard = Some(hook),
                            Err(err) => {
                                keyboard_failed = true;
                                *e.lock().unwrap() =
                                    Some(format!("Escape monitoring is unavailable: {err}"));
                            }
                        }
                    } else if !showing {
                        if let Some(hook) = keyboard.take() {
                            let _ = UnhookWindowsHookEx(hook);
                        }
                        keyboard_failed = false;
                    }
                    let mut message = MSG::default();
                    while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                while let Ok(event) = receiver.try_recv() {
                    match event {
                        InputEvent::Escape if v.load(Ordering::Relaxed) => {
                            if let Some(window) = app.get_webview_window("popup") {
                                let _ = crate::popup::hide(&window);
                            }
                            v.store(false, Ordering::Relaxed);
                            crate::stop_closed_popup_speech(&app);
                        }
                        InputEvent::Click { x, y, middle } => {
                            if v.load(Ordering::Relaxed) {
                                if let Some(window) = app.get_webview_window("popup") {
                                    if let (Ok(position), Ok(size)) =
                                        (window.outer_position(), window.outer_size())
                                    {
                                        if x < position.x
                                            || y < position.y
                                            || x >= position.x + size.width as i32
                                            || y >= position.y + size.height as i32
                                        {
                                            let _ = crate::popup::hide(&window);
                                            v.store(false, Ordering::Relaxed);
                                            crate::stop_popup_speech(&app);
                                        }
                                    }
                                }
                            }
                            if middle && m.load(Ordering::Relaxed) {
                                let app = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    crate::activate(app).await;
                                });
                            }
                        }
                        _ => {}
                    }
                }
                // Low-level hook callbacks only run while this thread pumps messages, so wake on
                // input immediately instead of sleeping, which would delay every mouse event.
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        MsgWaitForMultipleObjects, QS_ALLINPUT,
                    };
                    let _ = MsgWaitForMultipleObjects(None, false, 12, QS_ALLINPUT);
                }
            }
        });
        Self {
            middle,
            visible,
            error,
        }
    }
}
