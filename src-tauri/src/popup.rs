use crate::model::{PopupState, Rect};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size};

#[derive(Default)]
pub struct Popup {
    pub state: PopupState,
    pub history: Vec<String>,
    pub index: usize,
    pub anchor: Option<Rect>,
}
impl Popup {
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
    let x =
        (if east { right } else { left }).clamp(work.x, work.x + work.width as i32 - width as i32);
    let y = (if south { below } else { above })
        .clamp(work.y, work.y + work.height as i32 - height as i32);
    (
        Bounds {
            x,
            y,
            width,
            height,
        },
        format!(
            "{}{}",
            if south { "s" } else { "n" },
            if east { "e" } else { "w" }
        ),
    )
}

pub fn emit(app: &AppHandle, state: &PopupState) {
    let _ = app.emit("popup-state", state);
}

fn target(
    app: &AppHandle,
    anchor: Option<Rect>,
    expanded: bool,
    size: &str,
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
    let logical = if expanded {
        (440.0, 560.0)
    } else {
        match size {
            "small" => (300.0, 176.0),
            "large" => (390.0, 230.0),
            _ => (340.0, 200.0),
        }
    };
    let area = monitor.work_area();
    let work = Bounds {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
    };
    let (bounds, direction) = place(anchor, work, physical_size(logical, scale));
    Ok((window, bounds, direction))
}

fn apply(window: &tauri::WebviewWindow, bounds: Bounds) -> Result<(), String> {
    window
        .set_size(Size::Physical(PhysicalSize::new(
            bounds.width,
            bounds.height,
        )))
        .map_err(|e| e.to_string())?;
    window
        .set_position(Position::Physical(PhysicalPosition::new(
            bounds.x, bounds.y,
        )))
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_bounds(
    app: &AppHandle,
    anchor: Option<Rect>,
    expanded: bool,
    size: &str,
) -> Result<String, String> {
    let (window, bounds, direction) = target(app, anchor, expanded, size)?;
    apply(&window, bounds)?;
    Ok(direction)
}

pub async fn animate_bounds(
    app: &AppHandle,
    anchor: Option<Rect>,
    expanded: bool,
    size: &str,
    reduced_motion: bool,
) -> Result<String, String> {
    let (window, bounds, direction) = target(app, anchor, expanded, size)?;
    if reduced_motion {
        apply(&window, bounds)?;
        return Ok(direction);
    }
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let dimensions = window.outer_size().map_err(|e| e.to_string())?;
    let start = Bounds {
        x: position.x,
        y: position.y,
        width: dimensions.width,
        height: dimensions.height,
    };
    for step in 1..=10 {
        let part = step as f64 / 10.0;
        let ease = part * part * (3.0 - 2.0 * part);
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
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    Ok(direction)
}

pub fn show(app: &AppHandle) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
    let window = app
        .get_webview_window("popup")
        .ok_or("Popup window is unavailable.")?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    unsafe {
        let _ = ShowWindow(
            windows::Win32::Foundation::HWND(hwnd.0 as *mut _),
            SW_SHOWNOACTIVATE,
        );
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
            (440, 560),
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
            (440, 560),
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
        let size = physical_size((440.0, 560.0), 1.5);
        assert_eq!(size, (660, 840));
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
}
