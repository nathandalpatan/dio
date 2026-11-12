use crate::model::{normalize, Selection};
use std::{
    mem::size_of,
    ptr::copy_nonoverlapping,
    thread::sleep,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{CloseHandle, GlobalFree, HANDLE, HGLOBAL, HWND},
    System::{
        DataExchange::{
            CloseClipboard, CountClipboardFormats, EmptyClipboard, EnumClipboardFormats,
            GetClipboardData, GetClipboardOwner, GetClipboardSequenceNumber, OpenClipboard,
            SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
        Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION},
    },
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
            KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        },
        WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId},
    },
};

struct OpenedClipboard;
impl OpenedClipboard {
    unsafe fn new(owner: HWND) -> Result<Self, String> {
        OpenClipboard(Some(owner)).map_err(|e| format!("Clipboard unavailable: {e}"))?;
        Ok(Self)
    }
}
impl Drop for OpenedClipboard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

struct SavedHandle(HGLOBAL);
impl Drop for SavedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalFree(Some(self.0));
        }
    }
}

type ClipboardBackup = Vec<(u32, Vec<u8>)>;

fn restorable_format(format: u32) -> bool {
    !matches!(
        format,
        0 | 2 | 3 | 9 | 14 | 0x80 | 0x82 | 0x83 | 0x8e | 0x200..=0x3ff
    )
}

fn synthesized_image_format(format: u32, has_dib: bool) -> bool {
    has_dib && matches!(format, 2 | 9)
}

fn terminal_process(name: &str) -> bool {
    matches!(
        name,
        "windowsterminal.exe"
            | "wt.exe"
            | "openconsole.exe"
            | "conhost.exe"
            | "cmd.exe"
            | "powershell.exe"
            | "pwsh.exe"
            | "bash.exe"
            | "wsl.exe"
    )
}

unsafe fn source_process(window: HWND) -> Result<u32, String> {
    let mut pid = 0;
    GetWindowThreadProcessId(window, Some(&mut pid));
    if pid == 0 {
        Err("Cannot identify the selected application.".into())
    } else {
        Ok(pid)
    }
}

unsafe fn safe_source(window: HWND) -> Result<u32, String> {
    if window.0.is_null() || GetForegroundWindow() != window {
        return Err("The selected application lost focus.".into());
    }
    let mut class = [0u16; 256];
    let len = GetClassNameW(window, &mut class);
    if len == 0 {
        return Err("Cannot identify the selected application.".into());
    }
    let class = String::from_utf16_lossy(&class[..len as usize]);
    if class.eq_ignore_ascii_case("ConsoleWindowClass")
        || class.eq_ignore_ascii_case("CASCADIA_HOSTING_WINDOW_CLASS")
    {
        return Err("Copy is disabled in terminal windows.".into());
    }
    let pid = source_process(window)?;
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
        .map_err(|e| format!("Cannot inspect selected application: {e}"))?;
    let mut path = [0u16; 1024];
    let mut len = path.len() as u32;
    let query = QueryFullProcessImageNameW(
        process,
        Default::default(),
        windows::core::PWSTR(path.as_mut_ptr()),
        &mut len,
    );
    let _ = CloseHandle(process);
    query.map_err(|e| format!("Cannot inspect selected application: {e}"))?;
    let name = String::from_utf16_lossy(&path[..len as usize]).to_ascii_lowercase();
    if terminal_process(name.rsplit('\\').next().unwrap_or(&name)) {
        return Err("Copy is disabled in terminal windows.".into());
    }
    Ok(pid)
}

unsafe fn snapshot(owner: HWND) -> Result<(u32, ClipboardBackup), String> {
    let _open = OpenedClipboard::new(owner)?;
    let count = CountClipboardFormats();
    if !(0..=64).contains(&count) {
        return Err("Clipboard formats cannot be safely preserved.".into());
    }
    let mut available = Vec::with_capacity(count as usize);
    let mut format = 0;
    for _ in 0..count {
        format = EnumClipboardFormats(format);
        if format == 0 {
            return Err("Clipboard formats cannot be enumerated.".into());
        }
        available.push(format);
    }
    let has_dib = available.contains(&8) || available.contains(&17);
    let mut formats = Vec::with_capacity(available.len());
    let mut total = 0usize;
    for format in available {
        // Windows can recreate bitmap and palette views from a preserved DIB.
        if synthesized_image_format(format, has_dib) {
            continue;
        }
        // GDI handles and owner-managed/private formats cannot be copied as HGLOBAL data.
        if !restorable_format(format) {
            return Err("Clipboard formats cannot be safely preserved.".into());
        }
        let handle =
            GetClipboardData(format).map_err(|e| format!("Clipboard format unavailable: {e}"))?;
        let size = GlobalSize(HGLOBAL(handle.0));
        total = total.saturating_add(size);
        if size == 0 || total > 32 * 1024 * 1024 {
            return Err("Clipboard formats cannot be safely preserved.".into());
        }
        let pointer = GlobalLock(HGLOBAL(handle.0));
        if pointer.is_null() {
            return Err("Clipboard format cannot be read.".into());
        }
        let bytes = std::slice::from_raw_parts(pointer as *const u8, size).to_vec();
        let _ = GlobalUnlock(HGLOBAL(handle.0));
        formats.push((format, bytes));
    }
    let sequence = GetClipboardSequenceNumber();
    if sequence == 0 {
        return Err("Clipboard change tracking is unavailable.".into());
    }
    Ok((sequence, formats))
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                ..Default::default()
            },
        },
    }
}

unsafe fn copy_selection(window: HWND, sequence: u32) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN, VK_C]
        .iter()
        .any(|vk| GetAsyncKeyState(vk.0 as i32) < 0)
    {
        if Instant::now() >= deadline {
            return Err("Shortcut keys are still held.".into());
        }
        sleep(Duration::from_millis(5));
    }
    if GetForegroundWindow() != window || GetClipboardSequenceNumber() != sequence {
        return Err("The selection or clipboard changed.".into());
    }
    let inputs = [
        key(VK_CONTROL, false),
        key(VK_C, false),
        key(VK_C, true),
        key(VK_CONTROL, true),
    ];
    if SendInput(&inputs, size_of::<INPUT>() as i32) != inputs.len() as u32 {
        let release = [key(VK_C, true), key(VK_CONTROL, true)];
        let _ = SendInput(&release, size_of::<INPUT>() as i32);
        return Err("Copy input was blocked.".into());
    }
    Ok(())
}

unsafe fn clipboard_text() -> Result<String, String> {
    let handle = GetClipboardData(13).map_err(|_| "Copy did not provide text.".to_string())?;
    let size = GlobalSize(HGLOBAL(handle.0));
    if size < 2 || !size.is_multiple_of(2) {
        return Err("Copied text is invalid.".into());
    }
    let pointer = GlobalLock(HGLOBAL(handle.0));
    if pointer.is_null() {
        return Err("Copied text cannot be read.".into());
    }
    let chars = std::slice::from_raw_parts(pointer as *const u16, (size / 2).min(130));
    let end = chars.iter().position(|&c| c == 0);
    let text = end.and_then(|end| String::from_utf16(&chars[..end]).ok());
    let _ = GlobalUnlock(HGLOBAL(handle.0));
    normalize(&text.ok_or("Copied text is invalid or too long.")?)
}

unsafe fn restore(owner: HWND, expected: u32, formats: ClipboardBackup) -> Result<(), String> {
    let _open = OpenedClipboard::new(owner)?;
    if GetClipboardSequenceNumber() != expected {
        return Err("Clipboard changed again; leaving newer contents intact.".into());
    }
    let mut saved = Vec::with_capacity(formats.len());
    for (format, bytes) in formats {
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len())
            .map_err(|e| format!("Clipboard backup allocation failed: {e}"))?;
        let pointer = GlobalLock(handle);
        if pointer.is_null() {
            let _ = GlobalFree(Some(handle));
            return Err("Clipboard backup allocation failed.".into());
        }
        copy_nonoverlapping(bytes.as_ptr(), pointer as *mut u8, bytes.len());
        let _ = GlobalUnlock(handle);
        saved.push((format, SavedHandle(handle)));
    }
    EmptyClipboard().map_err(|e| format!("Clipboard restore failed: {e}"))?;
    let mut error = None;
    for (format, handle) in saved {
        if error.is_none() {
            let raw = handle.0;
            if let Err(e) = SetClipboardData(format, Some(HANDLE(raw.0))) {
                error = Some(format!("Clipboard restore failed: {e}"));
            } else {
                std::mem::forget(handle);
            }
        }
    }
    error.map_or(Ok(()), Err)
}

pub fn capture(foreground: isize, owner: isize) -> Result<Selection, String> {
    unsafe {
        let window = HWND(foreground as *mut _);
        let owner = HWND(owner as *mut _);
        if owner.0.is_null() {
            return Err("Popup window is unavailable.".into());
        }
        let pid = safe_source(window)?;
        let (sequence, formats) = snapshot(owner)?;
        copy_selection(window, sequence)?;
        let deadline = Instant::now() + Duration::from_millis(400);
        let mut changed = sequence;
        let mut last_change = Instant::now();
        loop {
            let current = GetClipboardSequenceNumber();
            if current != sequence {
                if current != changed {
                    changed = current;
                    last_change = Instant::now();
                }
                if last_change.elapsed() >= Duration::from_millis(20) {
                    break;
                }
            }
            if Instant::now() >= deadline {
                return Err("Copy timed out.".into());
            }
            sleep(Duration::from_millis(5));
        }
        let (result, copied_sequence, source_owned) = (|| {
            if GetForegroundWindow() != window {
                return Err("The selected application lost focus.".into());
            }
            let _open = OpenedClipboard::new(owner)?;
            let source_owned =
                source_process(GetClipboardOwner().map_err(|_| "Copy has no clipboard owner.")?)?
                    == pid;
            if GetClipboardSequenceNumber() != changed || !source_owned {
                return Err("Clipboard changed outside the selected application.".into());
            }
            let text = clipboard_text();
            Ok((text, GetClipboardSequenceNumber(), source_owned))
        })()
        .unwrap_or_else(|error| (Err(error), 0, false));
        if source_owned {
            restore(owner, copied_sequence, formats)?;
        }
        result.map(|text| Selection { text, bounds: None })
    }
}

#[cfg(test)]
mod tests {
    use super::{restorable_format, synthesized_image_format, terminal_process};
    #[test]
    fn terminal_processes_do_not_receive_copy() {
        assert!(terminal_process("windowsterminal.exe"));
        assert!(terminal_process("conhost.exe"));
        assert!(!terminal_process("code.exe"));
        assert!(!terminal_process("notepad.exe"));
    }
    #[test]
    fn preserves_global_memory_formats_only() {
        assert!(restorable_format(13)); // Unicode text
        assert!(restorable_format(15)); // File drop list
        assert!(restorable_format(0xc000)); // Registered format, such as HTML
        for format in [2, 3, 9, 14, 0x80, 0x82, 0x83, 0x8e, 0x200, 0x300] {
            assert!(!restorable_format(format));
        }
    }
    #[test]
    fn dib_backup_covers_synthesized_bitmap_formats() {
        assert!(synthesized_image_format(2, true));
        assert!(synthesized_image_format(9, true));
        assert!(!synthesized_image_format(2, false));
        assert!(!synthesized_image_format(14, true));
    }
}
