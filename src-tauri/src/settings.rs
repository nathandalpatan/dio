use crate::model::Settings;
use crate::{shared, AppState};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

/// Global shortcuts the settings ask for: the dictionary binding and the read-aloud binding.
pub fn bindings(settings: &Settings) -> Vec<String> {
    [
        (settings.keyboard_enabled, &settings.shortcut),
        (settings.reader_enabled, &settings.reader_shortcut),
    ]
    .into_iter()
    .filter(|(enabled, _)| *enabled)
    .map(|(_, binding)| binding.clone())
    .collect()
}

/// Registers new bindings before removing old ones, undoing partial work on failure so the
/// previous shortcuts stay active.
fn replace_bindings(
    old: &[String],
    new: &[String],
    mut register: impl FnMut(&str) -> Result<(), String>,
    mut unregister: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    let mut registered: Vec<&String> = Vec::new();
    for binding in new.iter().filter(|binding| !old.contains(binding)) {
        if let Err(error) = register(binding) {
            for done in registered {
                let _ = unregister(done);
            }
            return Err(error);
        }
        registered.push(binding);
    }
    let mut unregistered: Vec<&String> = Vec::new();
    for binding in old.iter().filter(|binding| !new.contains(binding)) {
        if let Err(error) = unregister(binding) {
            for done in registered {
                let _ = unregister(done);
            }
            for done in unregistered {
                let _ = register(done);
            }
            return Err(error);
        }
        unregistered.push(binding);
    }
    Ok(())
}

pub fn update(app: &AppHandle, next: Settings) -> Result<(), String> {
    next.validate()?;
    let current = shared(app).settings.lock().unwrap().clone();
    let shortcut = app.global_shortcut();
    let old: Vec<String> = bindings(&current)
        .into_iter()
        .filter(|binding| shortcut.is_registered(binding.as_str()))
        .collect();
    let new = bindings(&next);
    let register = |binding: &str| {
        shortcut.register(binding).map_err(|e| {
            format!("Could not register {binding}: {e}. The existing shortcut remains active.")
        })
    };
    let unregister = |binding: &str| {
        shortcut
            .unregister(binding)
            .map_err(|e| format!("Could not remove {binding}: {e}"))
    };
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
    if let Err(error) = replace_bindings(&old, &new, register, unregister) {
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
        let _ = replace_bindings(&new, &old, register, unregister);
        if next.startup != current.startup {
            let _ = if current.startup {
                autostart.enable()
            } else {
                autostart.disable()
            };
        }
        return Err(error);
    }
    shared(app).tts.set_volume(next.volume);
    shared(app).tts.set_enabled(next.tts_enabled);
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
    use std::cell::RefCell;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn failed_replacement_keeps_existing_binding() {
        let actions = RefCell::new(Vec::new());
        let result = replace_bindings(
            &strings(&["Ctrl+Alt+D"]),
            &strings(&["Ctrl+Alt+W"]),
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

    #[test]
    fn reader_conflict_rolls_back_without_touching_the_dictionary_shortcut() {
        let actions = RefCell::new(Vec::new());
        let result = replace_bindings(
            &strings(&["Ctrl+Alt+D", "Ctrl+Alt+R"]),
            &strings(&["Ctrl+Alt+E", "Ctrl+Alt+T"]),
            |binding| {
                actions.borrow_mut().push(format!("register {binding}"));
                if binding == "Ctrl+Alt+T" {
                    Err("conflict".into())
                } else {
                    Ok(())
                }
            },
            |binding| {
                actions.borrow_mut().push(format!("unregister {binding}"));
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(
            *actions.borrow(),
            [
                "register Ctrl+Alt+E",
                "register Ctrl+Alt+T",
                "unregister Ctrl+Alt+E"
            ]
        );
    }

    #[test]
    fn unchanged_bindings_are_left_alone_and_disabled_ones_removed() {
        let actions = RefCell::new(Vec::new());
        let current = Settings::default();
        let next = Settings {
            keyboard_enabled: false,
            ..current.clone()
        };
        assert_eq!(bindings(&current), ["Ctrl+Alt+D", "Ctrl+Alt+R"]);
        replace_bindings(
            &bindings(&current),
            &bindings(&next),
            |binding| {
                actions.borrow_mut().push(format!("register {binding}"));
                Ok(())
            },
            |binding| {
                actions.borrow_mut().push(format!("unregister {binding}"));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(*actions.borrow(), ["unregister Ctrl+Alt+D"]);
    }
}
