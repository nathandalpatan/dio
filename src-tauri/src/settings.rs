use crate::model::Settings;
use crate::{shared, AppState};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

fn replace_binding(
    old: Option<&str>,
    new: Option<&str>,
    mut register: impl FnMut(&str) -> Result<(), String>,
    mut unregister: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(next) = new {
        register(next)?;
    }
    if let Some(previous) = old {
        if let Err(error) = unregister(previous) {
            if let Some(next) = new {
                let _ = unregister(next);
            }
            return Err(error);
        }
    }
    Ok(())
}

pub fn update(app: &AppHandle, next: Settings) -> Result<(), String> {
    next.validate()?;
    let current = shared(app).settings.lock().unwrap().clone();
    let shortcut = app.global_shortcut();
    let old_registered =
        current.keyboard_enabled && shortcut.is_registered(current.shortcut.as_str());
    let replacement =
        next.keyboard_enabled && (!old_registered || current.shortcut != next.shortcut);
    let autostart = app.autolaunch();
    if next.startup != current.startup {
        let result = if next.startup {
            autostart.enable()
        } else {
            autostart.disable()
        };
        if let Err(error) = result {
            return Err(format!("Could not change startup setting: {error}"));
        }
    }
    let old = if old_registered && (!next.keyboard_enabled || current.shortcut != next.shortcut) {
        Some(current.shortcut.as_str())
    } else {
        None
    };
    let new = if replacement {
        Some(next.shortcut.as_str())
    } else {
        None
    };
    if let Err(error) = replace_binding(
        old,
        new,
        |binding| {
            shortcut.register(binding).map_err(|e| {
                format!("Could not register {binding}: {e}. The existing shortcut remains active.")
            })
        },
        |binding| {
            shortcut
                .unregister(binding)
                .map_err(|e| format!("Could not remove {binding}: {e}"))
        },
    ) {
        if next.startup != current.startup {
            let _ = if current.startup {
                autostart.enable()
            } else {
                autostart.disable()
            };
        }
        return Err(error);
    }
    if let Err(error) = app
        .state::<AppState>()
        .storage
        .lock()
        .unwrap()
        .write_settings(&next)
    {
        if old_registered && (!next.keyboard_enabled || current.shortcut != next.shortcut) {
            let _ = shortcut.register(current.shortcut.as_str());
        }
        if next.startup != current.startup {
            let _ = if current.startup {
                autostart.enable()
            } else {
                autostart.disable()
            };
        }
        if replacement {
            let _ = shortcut.unregister(next.shortcut.as_str());
        }
        return Err(error);
    }
    *shared(app).settings.lock().unwrap() = next;
    app.state::<crate::native::Hooks>().middle.store(
        shared(app).settings.lock().unwrap().middle_click,
        Ordering::Relaxed,
    );
    *shared(app).shortcut_error.lock().unwrap() = None;
    let _ = app.emit(
        "settings-changed",
        shared(app).settings.lock().unwrap().clone(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_replacement_keeps_existing_binding() {
        use std::cell::RefCell;
        let actions = RefCell::new(Vec::new());
        let result = replace_binding(
            Some("Ctrl+Alt+D"),
            Some("Ctrl+Alt+W"),
            |binding| {
                actions.borrow_mut().push(format!("register {binding}"));
                Err("conflict".into())
            },
            |binding| {
                actions.borrow_mut().push(format!("unregister {binding}"));
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(*actions.borrow(), vec!["register Ctrl+Alt+W"]);
    }
}
