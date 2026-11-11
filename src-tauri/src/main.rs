#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dictionary;
mod model;
mod native;
mod popup;
mod settings;
mod storage;

use dictionary::Dictionary;
use model::{normalize, PopupState, Settings};
use native::{Hooks, SelectionReader};
use popup::Popup;
use std::sync::Mutex;
use storage::{now, Storage};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

struct AppState {
    dictionary: Dictionary,
    reader: SelectionReader,
    storage: Mutex<Storage>,
    settings: Mutex<Settings>,
    popup: Mutex<Popup>,
    shortcut_error: Mutex<Option<String>>,
}

fn shared(app: &AppHandle) -> &AppState {
    app.state::<AppState>().inner()
}
fn publish(app: &AppHandle) {
    popup::emit(app, &shared(app).popup.lock().unwrap().state.clone());
}

fn begin(app: &AppHandle, word: String, history: bool) -> u64 {
    let default_expanded = shared(app).settings.lock().unwrap().expanded;
    let preserve_expansion = app
        .get_webview_window("popup")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    let mut popup = shared(app).popup.lock().unwrap();
    let expanded = if preserve_expansion && popup.state.status == "ready" {
        popup.state.expanded
    } else {
        default_expanded
    };
    if history {
        let next = popup.index + 1;
        popup.history.truncate(next);
        popup.history.push(word.clone());
        popup.index = popup.history.len() - 1;
    }
    let request_id = popup.state.request_id + 1;
    popup.state = PopupState {
        request_id,
        status: "loading".into(),
        query: word,
        expanded,
        can_back: popup.index > 0,
        ..Default::default()
    };
    drop(popup);
    publish(app);
    request_id
}

fn fail(app: &AppHandle, id: u64, status: &str, message: String) {
    let mut popup = shared(app).popup.lock().unwrap();
    if !popup.update_if_current(id, |state| {
        state.status = status.into();
        state.message = message;
    }) {
        return;
    }
    drop(popup);
    publish(app);
}

async fn resolve(app: AppHandle, word: String, id: u64) {
    let cached = match shared(&app).storage.lock().unwrap().get(&word, now()) {
        Ok(value) => value,
        Err(error) => {
            fail(&app, id, "storage", error);
            return;
        }
    };
    let cache_hit = cached.as_ref().is_some_and(|item| item.fresh);
    let fetched = if cache_hit {
        Ok(cached.as_ref().unwrap().entries.clone())
    } else {
        shared(&app).dictionary.lookup(&word).await
    };
    let (entries, stale) = match fetched {
        Ok(entries) => (entries, false),
        Err(error) => match cached {
            Some(item) => (item.entries, true),
            None => {
                fail(&app, id, error.kind, error.message);
                return;
            }
        },
    };
    if !stale && !cache_hit {
        if let Err(error) = shared(&app)
            .storage
            .lock()
            .unwrap()
            .put(&word, &entries, now())
        {
            fail(&app, id, "storage", error);
            return;
        }
    }
    let saved = match shared(&app).storage.lock().unwrap().saved_entry(&word) {
        Ok(value) => value.is_some(),
        Err(error) => {
            fail(&app, id, "storage", error);
            return;
        }
    };
    let mut popup = shared(&app).popup.lock().unwrap();
    if !popup.update_if_current(id, |state| {
        state.status = "ready".into();
        state.entries = entries;
        state.stale = stale;
        state.saved = saved;
        state.message = if stale {
            "Offline copy. The dictionary could not refresh this entry.".into()
        } else {
            String::new()
        };
    }) {
        return;
    }
    drop(popup);
    publish(&app);
}

async fn lookup_inner(app: AppHandle, word: String, history: bool) -> Result<(), String> {
    let word = normalize(&word)?;
    let id = begin(&app, word.clone(), history);
    let settings = shared(&app).settings.lock().unwrap().clone();
    let (anchor, expanded) = {
        let popup = shared(&app).popup.lock().unwrap();
        (popup.anchor, popup.state.expanded)
    };
    let direction = popup::set_bounds(&app, anchor, expanded, &settings.size)?;
    shared(&app).popup.lock().unwrap().state.direction = direction;
    publish(&app);
    popup::show(&app)?;
    resolve(app, word, id).await;
    Ok(())
}

async fn activate(app: AppHandle) {
    let Some(window) = app.get_webview_window("popup") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.set_focus();
        return;
    }
    let foreground = native::foreground();
    match shared(&app).reader.read(foreground).await {
        Ok(selection) => {
            shared(&app).popup.lock().unwrap().anchor = selection.bounds;
            let _ = lookup_inner(app, selection.text, true).await;
        }
        Err(message) => {
            shared(&app).popup.lock().unwrap().anchor = None;
            let id = begin(&app, String::new(), false);
            let settings = shared(&app).settings.lock().unwrap().clone();
            let _ = popup::set_bounds(&app, None, false, &settings.size);
            fail(
                &app,
                id,
                if message.contains("timed out") {
                    "timeout"
                } else {
                    "unsupported"
                },
                message,
            );
            let _ = popup::show(&app);
        }
    }
}

#[tauri::command]
fn popup_state(app: AppHandle) -> PopupState {
    shared(&app).popup.lock().unwrap().state.clone()
}
#[tauri::command]
async fn lookup(app: AppHandle, word: String) -> Result<(), String> {
    lookup_inner(app, word, true).await
}
#[tauri::command]
async fn back(app: AppHandle) -> Result<(), String> {
    let word = {
        let mut popup = shared(&app).popup.lock().unwrap();
        if popup.index == 0 {
            return Ok(());
        }
        popup.index -= 1;
        popup.history[popup.index].clone()
    };
    lookup_inner(app, word, false).await
}
#[tauri::command]
fn close_popup(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("popup")
        .ok_or("Popup window is unavailable.")?
        .hide()
        .map_err(|e| e.to_string())?;
    app.state::<Hooks>()
        .visible
        .store(false, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
#[tauri::command]
async fn expand(app: AppHandle, expanded: bool, reduced_motion: bool) -> Result<(), String> {
    let settings = shared(&app).settings.lock().unwrap().clone();
    let anchor = shared(&app).popup.lock().unwrap().anchor;
    {
        shared(&app).popup.lock().unwrap().state.expanded = expanded;
    }
    publish(&app);
    let direction = popup::animate_bounds(
        &app,
        anchor,
        expanded,
        &settings.size,
        settings.reduced_motion || reduced_motion,
    )
    .await?;
    shared(&app).popup.lock().unwrap().state.direction = direction;
    publish(&app);
    Ok(())
}
#[tauri::command]
fn save_word(app: AppHandle, save: bool) -> Result<(), String> {
    let current = shared(&app).popup.lock().unwrap().state.clone();
    if current.status != "ready" {
        return Err("No definition to save.".into());
    }
    shared(&app)
        .storage
        .lock()
        .unwrap()
        .save(&current.query, &current.entries, save)?;
    shared(&app).popup.lock().unwrap().state.saved = save;
    publish(&app);
    let _ = app.emit("saved-changed", ());
    Ok(())
}
#[tauri::command]
fn saved_words(app: AppHandle) -> Result<Vec<String>, String> {
    shared(&app).storage.lock().unwrap().saved()
}
#[tauri::command]
fn open_saved(app: AppHandle, word: String) -> Result<(), String> {
    let word = normalize(&word)?;
    let entries = shared(&app)
        .storage
        .lock()
        .unwrap()
        .saved_entry(&word)?
        .ok_or("This word is no longer saved.")?;
    shared(&app).popup.lock().unwrap().anchor = None;
    let id = begin(&app, word.clone(), true);
    let settings = shared(&app).settings.lock().unwrap().clone();
    let expanded = shared(&app).popup.lock().unwrap().state.expanded;
    let direction = popup::set_bounds(&app, None, expanded, &settings.size)?;
    let mut popup = shared(&app).popup.lock().unwrap();
    if popup.state.request_id == id {
        popup.state.status = "ready".into();
        popup.state.entries = entries;
        popup.state.saved = true;
        popup.state.direction = direction;
    }
    drop(popup);
    publish(&app);
    popup::show(&app)
}
#[tauri::command]
fn clear_cache(app: AppHandle) -> Result<(), String> {
    shared(&app).storage.lock().unwrap().clear()
}
#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    shared(&app).settings.lock().unwrap().clone()
}
#[tauri::command]
fn shortcut_error(app: AppHandle) -> Option<String> {
    shared(&app).shortcut_error.lock().unwrap().clone()
}
#[tauri::command]
fn hook_error(app: AppHandle) -> Option<String> {
    app.state::<Hooks>().error.lock().unwrap().clone()
}
#[tauri::command]
fn update_settings(app: AppHandle, next: Settings) -> Result<(), String> {
    settings::update(&app, next)
}
#[tauri::command]
async fn pronunciation(app: AppHandle, url: String) -> Result<tauri::ipc::Response, String> {
    let current = shared(&app).popup.lock().unwrap().state.clone();
    if !current
        .entries
        .iter()
        .flat_map(|entry| &entry.phonetics)
        .any(|phonetic| phonetic.audio == url)
    {
        return Err("Audio is not part of the current entry.".into());
    }
    shared(&app)
        .dictionary
        .audio(&url)
        .await
        .map(tauri::ipc::Response::new)
}

fn open_settings(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window
            .show()
            .and_then(|_| window.set_focus())
            .map_err(|e| e.to_string())?;
    } else {
        WebviewWindowBuilder::new(
            app,
            "settings",
            WebviewUrl::App("index.html?settings".into()),
        )
        .title("DIO Settings")
        .inner_size(620.0, 640.0)
        .min_inner_size(520.0, 450.0)
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = open_settings(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            activate(app).await;
                        });
                    }
                })
                .build(),
        )
        .setup(|app| {
            use tauri::{
                menu::{Menu, MenuItem},
                tray::TrayIconBuilder,
            };
            let directory = app.path().app_data_dir()?;
            let storage = Storage::open(&directory).map_err(std::io::Error::other)?;
            let settings = storage.settings().map_err(std::io::Error::other)?;
            let dictionary = Dictionary::new().map_err(std::io::Error::other)?;
            app.manage(AppState {
                dictionary,
                reader: SelectionReader::new(),
                storage: Mutex::new(storage),
                settings: Mutex::new(settings.clone()),
                popup: Mutex::new(Popup::default()),
                shortcut_error: Mutex::new(None),
            });
            app.manage(Hooks::new(app.handle().clone(), settings.middle_click));
            if settings.keyboard_enabled {
                if let Err(error) = app.global_shortcut().register(settings.shortcut.as_str()) {
                    *app.state::<AppState>().shortcut_error.lock().unwrap() = Some(format!(
                        "Could not register {}: {error}. Choose another shortcut in Settings.",
                        settings.shortcut
                    ));
                }
            }
            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit DIO", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("DIO Dictionary")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "settings" => {
                        let _ = open_settings(app);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            popup_state,
            lookup,
            back,
            close_popup,
            expand,
            save_word,
            saved_words,
            open_saved,
            clear_cache,
            get_settings,
            shortcut_error,
            hook_error,
            update_settings,
            pronunciation
        ])
        .run(tauri::generate_context!())
        .expect("failed to run DIO");
}
