use crate::model::{PopupState, Rect, Settings, MIN_POPUP};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER},
};

#[derive(Default)]
pub struct Popup {
    pub state: PopupState,
    pub history: Vec<String>,
    pub index: usize,
    pub anchor: Option<Rect>,
}
impl Popup {
    pub fn record_word(&mut self, word: String) {
        self.history.truncate(self.index + 1);
        self.history.push(word);
        self.index = self.history.len() - 1;
    }

    pub fn back_word(&mut self) -> Option<String> {
        if self.index == 0 {
            return None;
        }
        self.index -= 1;
        Some(self.history[self.index].clone())
    }

    pub fn can_forward(&self) -> bool {
        self.index + 1 < self.history.len()
    }

    pub fn forward_word(&mut self) -> Option<String> {
        if !self.can_forward() {
            return None;
        }
        self.index += 1;
        Some(self.history[self.index].clone())
    }

    pub fn update_if_current(
        &mut self,
        request_id: u64,
        update: impl FnOnce(&mut PopupState),
    ) -> bool {
        if self.state.request_id != request_id {
            return false;
        }
        update(&mut self.state);
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn physical_size(logical: (f64, f64), scale: f64) -> (u32, u32) {
    (
        (logical.0 * scale).round() as u32,
        (logical.1 * scale).round() as u32,
    )
}

fn place(anchor: Rect, work: Bounds, size: (u32, u32)) -> (Bounds, String) {
    let (width, height) = (size.0.min(work.width), size.1.min(work.height));
    let right = anchor.x.round() as i32 + anchor.width.round() as i32 + 10;
    let left = anchor.x.round() as i32 - width as i32 - 10;
    let below = anchor.y.round() as i32 + anchor.height.round() as i32 + 10;
    let above = anchor.y.round() as i32 - height as i32 - 10;
    let east = right + width as i32 <= work.x + work.width as i32 || left < work.x;
    let south = below + height as i32 <= work.y + work.height as i32 || above < work.y;
    let direction = format!(
        "{}{}",
        if south { "s" } else { "n" },
        if east { "e" } else { "w" }
    );
    (
        place_in_direction(anchor, work, (width, height), &direction),
        direction,
    )
}

fn place_in_direction(anchor: Rect, work: Bounds, size: (u32, u32), direction: &str) -> Bounds {
    let (width, height) = (size.0.min(work.width), size.1.min(work.height));
    let east = direction.ends_with('e');
    let south = direction.starts_with('s');
    let x = (if east {
        anchor.x.round() as i32 + anchor.width.round() as i32 + 10
    } else {
        anchor.x.round() as i32 - width as i32 - 10
    })
    .clamp(work.x, work.x + work.width as i32 - width as i32);
    let y = (if south {
        anchor.y.round() as i32 + anchor.height.round() as i32 + 10
    } else {
        anchor.y.round() as i32 - height as i32 - 10
    })
    .clamp(work.y, work.y + work.height as i32 - height as i32);
    Bounds {
        x,
        y,
        width,
        height,
    }
}

fn resize_from_corner(start: Bounds, work: Bounds, size: (u32, u32), direction: &str) -> Bounds {
    let (width, height) = (size.0.min(work.width), size.1.min(work.height));
    let x = if direction.ends_with('e') {
        start.x
    } else {
        start.x + start.width as i32 - width as i32
    };
    let y = if direction.starts_with('s') {
        start.y
    } else {
        start.y + start.height as i32 - height as i32
    };
    Bounds {
        x: x.clamp(work.x, work.x + work.width as i32 - width as i32),
        y: y.clamp(work.y, work.y + work.height as i32 - height as i32),
        width,
        height,
    }
}

const EXPANDED: (f64, f64) = (514.0, 378.0);

/// Work-area height, in logical pixels, at which the popup sizes are used as written: a 1080p
/// screen at 100% scaling once the taskbar is taken off.
const REFERENCE_HEIGHT: f64 = 1040.0;

/// The popup size in logical pixels for a work area of `work` logical pixels. Without a size the
/// user dragged to, the chosen size gets a little more room on tall screens and a little less on
/// short ones. Text size never follows it.
fn logical_size(expanded: bool, settings: &Settings, work: (f64, f64)) -> (f64, f64) {
    let factor = (work.1 / REFERENCE_HEIGHT).clamp(0.9, 1.15);
    let compact = settings.popup_custom.unwrap_or_else(|| {
        let base = match settings.size.as_str() {
            "small" => (328.0, 293.0),
            "large" => (429.0, 372.0),
            _ => (378.0, 332.0),
        };
        (base.0 * factor, base.1 * factor)
    });
    let (width, height) = if expanded {
        // Expanding never shrinks a popup the user made larger than the usual expanded size.
        settings.expanded_custom.unwrap_or((
            (EXPANDED.0 * factor).max(compact.0),
            (EXPANDED.1 * factor).max(compact.1),
        ))
    } else {
        compact
    };
    let fit = |value: f64, minimum: f64, available: f64| {
        value.min(available * 0.9).max(minimum.min(available))
    };
    (
        fit(width, MIN_POPUP.0, work.0),
        fit(height, MIN_POPUP.1, work.1),
    )
}

fn logical_work(work: Bounds, scale: f64) -> (f64, f64) {
    (work.width as f64 / scale, work.height as f64 / scale)
}

pub fn emit(app: &AppHandle, state: &PopupState) {
    let _ = app.emit("popup-state", state);
}

fn target(
    app: &AppHandle,
    anchor: Option<Rect>,
    expanded: bool,
    settings: &Settings,
) -> Result<(tauri::WebviewWindow, Bounds, String), String> {
    let window = app
        .get_webview_window("popup")
        .ok_or("Popup window is unavailable.")?;
    let pointer = window.cursor_position().map_err(|e| e.to_string())?;
    let anchor = anchor.unwrap_or(Rect {
        x: pointer.x,
        y: pointer.y,
        width: 0.0,
        height: 0.0,
    });
    let monitor = window
        .monitor_from_point(anchor.x, anchor.y)
        .map_err(|e| e.to_string())?
        .or(window.current_monitor().map_err(|e| e.to_string())?)
        .ok_or("No display is available.")?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let work = Bounds {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
    };
    let logical = logical_size(expanded, settings, logical_work(work, scale));
    let largest = logical_size(true, settings, logical_work(work, scale));
    let (_, direction) = place(anchor, work, physical_size(largest, scale));
    let bounds = place_in_direction(anchor, work, physical_size(logical, scale), &direction);
    Ok((window, bounds, direction))
}

fn apply(window: &tauri::WebviewWindow, bounds: Bounds) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    unsafe {
        SetWindowPos(
            HWND(hwnd.0 as *mut _),
            None,
            bounds.x,
            bounds.y,
            bounds.width as i32,
            bounds.height as i32,
            SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOZORDER,
        )
        .map_err(|e| e.to_string())
    }
}

pub fn set_bounds(
    app: &AppHandle,
    anchor: Option<Rect>,
    expanded: bool,
    settings: &Settings,
) -> Result<String, String> {
    app.state::<crate::AppState>()
        .motion
        .fetch_add(1, Ordering::AcqRel);
    let (window, bounds, direction) = target(app, anchor, expanded, settings)?;
    apply(&window, bounds)?;
    Ok(direction)
}

pub async fn animate_bounds(
    app: &AppHandle,
    expanded: bool,
    settings: &Settings,
    reduced_motion: bool,
    direction: &str,
) -> Result<bool, String> {
    let motion = &app.state::<crate::AppState>().motion;
    let generation = motion.fetch_add(1, Ordering::AcqRel) + 1;
    let window = app
        .get_webview_window("popup")
        .ok_or("Popup window is unavailable.")?;
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No display is available.")?;
    let area = monitor.work_area();
    let work = Bounds {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
    };
    let logical = logical_size(
        expanded,
        settings,
        logical_work(work, monitor.scale_factor()),
    );
    if reduced_motion {
        let position = window.outer_position().map_err(|e| e.to_string())?;
        let dimensions = window.outer_size().map_err(|e| e.to_string())?;
        let start = Bounds {
            x: position.x,
            y: position.y,
            width: dimensions.width,
            height: dimensions.height,
        };
        apply(
            &window,
            resize_from_corner(
                start,
                work,
                physical_size(logical, monitor.scale_factor()),
                direction,
            ),
        )?;
        return Ok(true);
    }
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let dimensions = window.outer_size().map_err(|e| e.to_string())?;
    let start = Bounds {
        x: position.x,
        y: position.y,
        width: dimensions.width,
        height: dimensions.height,
    };
    let bounds = resize_from_corner(
        start,
        work,
        physical_size(logical, monitor.scale_factor()),
        direction,
    );
    let began = std::time::Instant::now();
    loop {
        if motion.load(Ordering::Acquire) != generation {
            return Ok(false);
        }
        let part = (began.elapsed().as_secs_f64() / 0.18).min(1.0);
        let ease = 1.0 - (1.0 - part).powi(3);
        let mix = |a: f64, b: f64| (a + (b - a) * ease).round();
        apply(
            &window,
            Bounds {
                x: mix(start.x as f64, bounds.x as f64) as i32,
                y: mix(start.y as f64, bounds.y as f64) as i32,
                width: mix(start.width as f64, bounds.width as f64) as u32,
                height: mix(start.height as f64, bounds.height as f64) as u32,
            },
        )?;
        if part >= 1.0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
    }
    Ok(true)
}

pub fn show(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("popup")
        .ok_or("Popup window is unavailable.")?;
    window.show().map_err(|e| e.to_string())?;
    app.state::<crate::native::Hooks>()
        .visible
        .store(true, std::sync::atomic::Ordering::Relaxed);
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// Hides the popup however it was shown. `show_without_focus` bypasses tao, whose `hide` then
/// believes the window is already hidden and does nothing, so the window is hidden natively too.
pub fn hide(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    window.hide().map_err(|e| e.to_string())?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    unsafe {
        let _ = ShowWindow(HWND(hwnd.0 as *mut _), SW_HIDE);
    }
    Ok(())
}

/// Shows the popup while the source application keeps keyboard focus.
pub fn show_without_focus(app: &AppHandle) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
    let window = app
        .get_webview_window("popup")
        .ok_or("Popup window is unavailable.")?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    unsafe {
        let _ = ShowWindow(HWND(hwnd.0 as *mut _), SW_SHOWNOACTIVATE);
    }
    app.state::<crate::native::Hooks>()
        .visible
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_history_is_replaced_after_a_new_lookup() {
        let mut popup = Popup::default();
        popup.record_word("truth".into());
        popup.record_word("fact".into());
        assert_eq!(popup.back_word().as_deref(), Some("truth"));
        assert!(popup.can_forward());
        assert_eq!(popup.forward_word().as_deref(), Some("fact"));
        assert!(!popup.can_forward());
        assert_eq!(popup.back_word().as_deref(), Some("truth"));
        popup.record_word("reality".into());
        assert_eq!(popup.history, ["truth".to_string(), "reality".to_string()]);
        assert_eq!(popup.forward_word(), None);
    }

    #[test]
    fn positions_on_negative_monitor_and_flips() {
        let work = Bounds {
            x: -1920,
            y: -100,
            width: 1920,
            height: 1080,
        };
        let (bounds, direction) = place(
            Rect {
                x: -50.0,
                y: 850.0,
                width: 20.0,
                height: 20.0,
            },
            work,
            (720, 530),
        );
        assert_eq!(direction, "nw");
        assert!(bounds.x >= -1920 && bounds.x + bounds.width as i32 <= 0);
        assert!(bounds.y >= -100 && bounds.y + bounds.height as i32 <= 980);
    }
    #[test]
    fn constrains_oversized_popup() {
        let (bounds, _) = place(
            Rect::default(),
            Bounds {
                x: 0,
                y: 0,
                width: 300,
                height: 200,
            },
            (720, 530),
        );
        assert_eq!((bounds.width, bounds.height), (300, 200));
    }
    #[test]
    fn late_response_does_not_replace_newer_lookup() {
        let mut popup = Popup::default();
        popup.state.request_id = 2;
        popup.state.query = "new".into();
        assert!(!popup.update_if_current(1, |state| state.query = "old".into()));
        assert_eq!(popup.state.query, "new");
    }
    #[test]
    fn scales_popup_on_mixed_dpi_monitor() {
        let work = Bounds {
            x: -2400,
            y: 0,
            width: 2400,
            height: 1350,
        };
        let size = physical_size(EXPANDED, 1.5);
        assert_eq!(size, (771, 567));
        let (bounds, _) = place(
            Rect {
                x: -250.0,
                y: 900.0,
                width: 40.0,
                height: 30.0,
            },
            work,
            size,
        );
        assert!(bounds.x >= work.x && bounds.x + bounds.width as i32 <= 0);
        assert!(bounds.y >= 0 && bounds.y + bounds.height as i32 <= 1350);
    }

    #[test]
    fn resize_keeps_the_source_facing_corner_fixed() {
        let work = Bounds {
            x: 0,
            y: 0,
            width: 1600,
            height: 1000,
        };
        for direction in ["se", "sw", "ne", "nw"] {
            let compact = Bounds {
                x: 600,
                y: 400,
                width: 530,
                height: 465,
            };
            let expanded = resize_from_corner(compact, work, (720, 530), direction);
            let collapsed = resize_from_corner(expanded, work, (530, 465), direction);
            assert_eq!(collapsed, compact, "{direction}");
            assert_eq!(
                if direction.ends_with('e') {
                    expanded.x
                } else {
                    expanded.x + expanded.width as i32
                },
                if direction.ends_with('e') {
                    compact.x
                } else {
                    compact.x + compact.width as i32
                },
                "{direction}"
            );
            assert_eq!(
                if direction.starts_with('s') {
                    expanded.y
                } else {
                    expanded.y + expanded.height as i32
                },
                if direction.starts_with('s') {
                    compact.y
                } else {
                    compact.y + compact.height as i32
                },
                "{direction}"
            );
        }
    }

    #[test]
    fn popup_size_follows_the_screen() {
        let standard = Settings::default();
        assert_eq!(
            logical_size(false, &standard, (1920.0, 1040.0)),
            (378.0, 332.0)
        );
        let roomy = logical_size(false, &standard, (2560.0, 1392.0));
        assert_eq!(roomy, (378.0 * 1.15, 332.0 * 1.15));
        let cramped = logical_size(false, &standard, (1366.0, 728.0));
        assert_eq!(cramped, (378.0 * 0.9, 332.0 * 0.9));
        let tiny = logical_size(true, &standard, (400.0, 300.0));
        assert_eq!(tiny, (360.0, 270.0));
    }
    #[test]
    fn remembered_sizes_win_and_expanding_never_shrinks() {
        let dragged = Settings {
            popup_custom: Some((700.0, 600.0)),
            ..Settings::default()
        };
        let work = (1920.0, 1040.0);
        assert_eq!(logical_size(false, &dragged, work), (700.0, 600.0));
        assert_eq!(logical_size(true, &dragged, work), (700.0, 600.0));
        let both = Settings {
            expanded_custom: Some((900.0, 700.0)),
            ..dragged
        };
        assert_eq!(logical_size(true, &both, work), (900.0, 700.0));
        assert_eq!(logical_size(false, &both, (800.0, 500.0)).1, 450.0);
    }
    #[test]
    fn compact_placement_uses_the_expanded_direction() {
        let work = Bounds {
            x: 0,
            y: 0,
            width: 1600,
            height: 1200,
        };
        let anchor = Rect {
            x: 1200.0,
            y: 1000.0,
            width: 20.0,
            height: 20.0,
        };
        let (_, direction) = place(anchor, work, (720, 530));
        assert_eq!(direction, "nw");
        let compact = place_in_direction(anchor, work, (530, 465), &direction);
        assert_eq!(
            (
                compact.x + compact.width as i32,
                compact.y + compact.height as i32
            ),
            (1190, 990)
        );
    }
}
