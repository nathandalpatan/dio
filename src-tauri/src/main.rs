#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod dictionary;
mod install_prefs;
mod local_dictionary;
mod model;
mod native;
mod popup;
mod pronunciation;
mod settings;
mod storage;
mod toast;
mod tts;
mod updates;

use dictionary::Dictionary;
use local_dictionary::LocalDictionary;
use model::{normalize, normalize_with, Entry, Limits, PopupState, Settings, WORD_LIMITS};
use native::{Hooks, SelectionReader};
use popup::Popup;
use pronunciation::Pronunciations;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use storage::{now, Storage};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tts::{model_store, Tts, TtsState};
use updates::Updates;

struct AppState {
    dictionary: Dictionary,
    local_dictionary: Mutex<LocalDictionary>,
    pronunciations: Mutex<Pronunciations>,
    reader: SelectionReader,
    storage: Mutex<Storage>,
    settings: Mutex<Settings>,
    popup: Mutex<Popup>,
    motion: AtomicU64,
    shortcut_error: Mutex<Option<String>>,
    capturing: AtomicBool,
    tts: Tts,
    model_dir: PathBuf,
    install: Mutex<Option<Arc<AtomicBool>>>,
}

fn shared(app: &AppHandle) -> &AppState {
    app.state::<AppState>().inner()
}
fn entry_word<'a>(query: &'a str, entries: &'a [Entry]) -> &'a str {
    entries
        .first()
        .filter(|entry| !entry.word.is_empty())
        .map_or(query, |entry| entry.word.as_str())
}
fn publish(app: &AppHandle) {
    popup::emit(app, &shared(app).popup.lock().unwrap().state.clone());
}

fn begin(app: &AppHandle, word: String, history: bool) -> u64 {
    stop_popup_speech(app);
    // The model unloads after a while unused; start loading it while the definition is found.
    shared(app).tts.warm();
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
        popup.record_word(word.clone());
    }
    let request_id = popup.state.request_id + 1;
    popup.state = PopupState {
        request_id,
        status: "loading".into(),
        query: word,
        expanded,
        can_back: popup.index > 0,
        can_forward: popup.can_forward(),
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
        state.expanded = false;
    }) {
        return;
    }
    let anchor = popup.anchor;
    drop(popup);
    let settings = shared(app).settings.lock().unwrap().clone();
    if let Ok(direction) = popup::set_bounds(app, anchor, false, &settings) {
        shared(app).popup.lock().unwrap().state.direction = direction;
    }
    publish(app);
}

async fn resolve(app: AppHandle, word: String, id: u64) {
    let local = match shared(&app).local_dictionary.lock().unwrap().special(&word) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("DIO local dictionary failed: {error}");
            None
        }
    };
    let local_hit = local.is_some();
    let cached = if local_hit {
        None
    } else {
        match shared(&app).storage.lock().unwrap().get(&word, now()) {
            Ok(value) => value,
            Err(error) => {
                fail(&app, id, "storage", error);
                return;
            }
        }
    };
    if let Some(entries) = local {
        show_ready(&app, id, &word, entries, false);
        return;
    }
    let cached = match cached {
        Some(item) if item.fresh => {
            show_ready(&app, id, &word, item.entries, false);
            return;
        }
        other => other.map(|item| item.entries),
    };
    // Show an older copy or the bundled entry straight away, then replace it with the online
    // entry when that arrives, so the network never holds up the first definition.
    let bundled = if cached.is_some() {
        None
    } else {
        match shared(&app).local_dictionary.lock().unwrap().words(&word) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("DIO local dictionary failed: {error}");
                None
            }
        }
    };
    if let Some(entries) = cached.as_ref().or(bundled.as_ref()) {
        if !show_ready(&app, id, &word, entries.clone(), false) {
            return;
        }
    }
    match shared(&app).dictionary.lookup(&word).await {
        Ok(entries) => {
            if let Err(error) = shared(&app)
                .storage
                .lock()
                .unwrap()
                .put(&word, &entries, now())
            {
                fail(&app, id, "storage", error);
                return;
            }
            show_ready(&app, id, &word, entries, false);
        }
        Err(_) if bundled.is_some() => {}
        Err(_) if cached.is_some() => {
            show_ready(&app, id, &word, cached.unwrap(), true);
        }
        Err(error) => fail(&app, id, error.kind, error.message),
    }
}

/// Shows `entries` as the definition for request `id`, unless a newer lookup replaced it.
/// Returns whether the request is still current.
fn show_ready(app: &AppHandle, id: u64, word: &str, entries: Vec<Entry>, stale: bool) -> bool {
    let saved = match shared(app)
        .storage
        .lock()
        .unwrap()
        .saved_entry(entry_word(word, &entries))
    {
        Ok(value) => value.is_some(),
        Err(error) => {
            fail(app, id, "storage", error);
            return false;
        }
    };
    if !shared(app)
        .popup
        .lock()
        .unwrap()
        .update_if_current(id, |state| {
            state.status = "ready".into();
            state.entries = entries;
            state.stale = stale;
            state.saved = saved;
            state.message = if stale {
                "Offline copy. The dictionary could not refresh this entry.".into()
            } else {
                String::new()
            };
        })
    {
        return false;
    }
    publish(app);
    load_pronunciations(app, id);
    load_synonyms(app, id);
    prepare_word(
        app,
        entry_word(word, &shared(app).popup.lock().unwrap().state.entries).to_owned(),
    );
    true
}

/// The voice a looked-up word is pronounced in: the chosen voice, or one in the preferred accent.
fn word_voice(settings: &Settings) -> &'static tts::VoiceInfo {
    tts::voice_for_accent(&settings.voice, &settings.accent)
}

/// Synthesizes the word in the background so pressing the speaker plays it at once.
fn prepare_word(app: &AppHandle, word: String) {
    let settings = shared(app).settings.lock().unwrap().clone();
    let voice = word_voice(&settings);
    shared(app).tts.prepare(tts::Prepare {
        text: word,
        voice: voice.id.into(),
        british: voice.accent == "uk",
        speed: settings.speed,
    });
}

/// Resolves IPA after the definition is visible so pronunciation data never delays or blocks
/// a definition.
fn load_pronunciations(app: &AppHandle, id: u64) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (word, entries) = {
            let popup = shared(&app).popup.lock().unwrap();
            if popup.state.request_id != id || popup.state.status != "ready" {
                return;
            }
            let word = entry_word(&popup.state.query, &popup.state.entries).to_owned();
            (word, popup.state.entries.clone())
        };
        let found = shared(&app)
            .pronunciations
            .lock()
            .unwrap()
            .resolve(&word, &entries);
        if shared(&app)
            .popup
            .lock()
            .unwrap()
            .update_if_current(id, |state| state.pronunciations = found)
        {
            publish(&app);
        }
    });
}

/// Groups synonyms by WordNet sense for every lookup, since online entries rarely have their own.
fn load_synonyms(app: &AppHandle, id: u64) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let word = {
            let popup = shared(&app).popup.lock().unwrap();
            if popup.state.request_id != id || popup.state.status != "ready" {
                return;
            }
            entry_word(&popup.state.query, &popup.state.entries).to_owned()
        };
        let senses = match shared(&app)
            .local_dictionary
            .lock()
            .unwrap()
            .synonym_senses(&word)
        {
            Ok(senses) => senses,
            Err(error) => {
                eprintln!("DIO local dictionary failed: {error}");
                return;
            }
        };
        if shared(&app)
            .popup
            .lock()
            .unwrap()
            .update_if_current(id, |state| state.synonym_senses = senses)
        {
            publish(&app);
        }
    });
}

fn stop_popup_speech(app: &AppHandle) {
    shared(app).tts.stop(Some("word"));
    shared(app).tts.stop(Some("popup"));
}

/// Closing the popup with its button or Escape also stops reading aloud. A click elsewhere only
/// hides it, so the reading can go on while you work in the app.
fn stop_closed_popup_speech(app: &AppHandle) {
    stop_popup_speech(app);
    shared(app).tts.stop(Some("reader"));
}

fn dio_has_focus(app: &AppHandle, foreground: isize) -> bool {
    ["popup", "settings"].into_iter().any(|label| {
        app.get_webview_window(label)
            .and_then(|window| window.hwnd().ok())
            .is_some_and(|hwnd| hwnd.0 as isize == foreground)
    })
}

fn excerpt(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() > 140 {
        format!("{}…", text.chars().take(140).collect::<String>().trim_end())
    } else {
        text
    }
}

/// Turns the final capture error into a message that tells the user what to do next.
fn capture_message(error: &str) -> String {
    if error.starts_with("Select ")
        || error.starts_with("DIO can't read selections")
        || error == "DIO does not read password fields."
    {
        return error.to_owned();
    }
    match error {
        clipboard::TERMINAL_NO_SELECTION => {
            "The terminal has no selection. If the app in it handles the mouse itself, like Claude Code, hold Shift while you select, then try again."
        }
        "Copy timed out." | "Copy did not provide text." => {
            "The app didn't copy the selection. Make sure the text is still highlighted and try again."
        }
        "Clipboard formats cannot be safely preserved."
        | "Clipboard formats cannot be enumerated."
        | "Clipboard format cannot be read." => {
            "Your clipboard holds something DIO can't safely restore, such as a screenshot. Copy some text first, then try again."
        }
        "Copy input was blocked." => {
            "Windows blocked DIO from copying in this app. It may be running as administrator."
        }
        "The selection or clipboard changed."
        | "The selected application lost focus."
        | "The active application changed. Try again."
        | "The active application changed. Select the word again." => {
            "The app or clipboard changed while DIO was reading. Try again."
        }
        _ => "Couldn't capture the selected text. Try highlighting it again.",
    }
    .into()
}

fn show_reader_message(app: &AppHandle, message: String) {
    shared(app).popup.lock().unwrap().anchor = None;
    let id = begin(app, String::new(), false);
    fail(app, id, "unsupported", message);
    let _ = popup::show_without_focus(app);
}

/// Reads the current selection aloud, or stops reading when the reader is already speaking.
async fn read_aloud(app: AppHandle) {
    let state = shared(&app).tts.state();
    if state.active() && state.owner == "reader" {
        shared(&app).tts.stop(Some("reader"));
        return;
    }
    let Some(window) = app.get_webview_window("popup") else {
        return;
    };
    let foreground = native::foreground();
    if dio_has_focus(&app, foreground) {
        return;
    }
    if let Err(message) = shared(&app).tts.available() {
        show_reader_message(&app, message);
        return;
    }
    if shared(&app).capturing.swap(true, Ordering::AcqRel) {
        return;
    }
    let settings = shared(&app).settings.lock().unwrap().clone();
    let limits = Limits {
        max_chars: settings.reader_max_chars,
        multiline: true,
    };
    let owner = window.hwnd().ok().map(|hwnd| hwnd.0 as isize).unwrap_or(0);
    let result = shared(&app).reader.read(foreground, owner, limits).await;
    shared(&app).capturing.store(false, Ordering::Release);
    let selection = match result {
        Ok(selection) => selection,
        Err(error) => {
            show_reader_message(&app, capture_message(&error));
            return;
        }
    };
    shared(&app).popup.lock().unwrap().anchor = selection.bounds;
    let id = begin(&app, String::new(), false);
    let mut popup = shared(&app).popup.lock().unwrap();
    popup.update_if_current(id, |state| {
        state.status = "reading".into();
        state.reading = excerpt(&selection.text);
        state.expanded = false;
    });
    let anchor = popup.anchor;
    drop(popup);
    if let Ok(direction) = popup::set_bounds(&app, anchor, false, &settings) {
        shared(&app).popup.lock().unwrap().state.direction = direction;
    }
    publish(&app);
    let _ = popup::show_without_focus(&app);
    let voice = tts::voice(&settings.voice).unwrap_or(&tts::VOICES[0]);
    if let Err(message) = shared(&app).tts.speak(tts::Request {
        text: selection.text,
        voice: voice.id.into(),
        british: voice.accent == "uk",
        speed: settings.speed,
        volume: settings.volume,
        owner: "reader".into(),
    }) {
        fail(&app, id, "unsupported", message);
    }
}

async fn lookup_inner(app: AppHandle, word: String, history: bool) -> Result<(), String> {
    let word = normalize(&word)?;
    let id = begin(&app, word.clone(), history);
    let settings = shared(&app).settings.lock().unwrap().clone();
    let (anchor, expanded) = {
        let popup = shared(&app).popup.lock().unwrap();
        (popup.anchor, popup.state.expanded)
    };
    let direction = popup::set_bounds(&app, anchor, expanded, &settings)?;
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
    let foreground = native::foreground();
    if dio_has_focus(&app, foreground) {
        return;
    }
    if shared(&app).capturing.swap(true, Ordering::AcqRel) {
        return;
    }
    let owner = window.hwnd().ok().map(|hwnd| hwnd.0 as isize).unwrap_or(0);
    let result = shared(&app)
        .reader
        .read(foreground, owner, WORD_LIMITS)
        .await;
    shared(&app).capturing.store(false, Ordering::Release);
    match result {
        Ok(selection) => {
            shared(&app).popup.lock().unwrap().anchor = selection.bounds;
            let word = model::trim_selected_word(&selection.text).to_owned();
            let _ = lookup_inner(app.clone(), word, true).await;
        }
        Err(error) => {
            shared(&app).popup.lock().unwrap().anchor = None;
            let id = begin(&app, String::new(), false);
            fail(&app, id, "unsupported", capture_message(&error));
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
    let word = shared(&app).popup.lock().unwrap().back_word();
    if let Some(word) = word {
        lookup_inner(app, word, false).await
    } else {
        close_popup(app)
    }
}
#[tauri::command]
async fn forward(app: AppHandle) -> Result<(), String> {
    let word = shared(&app).popup.lock().unwrap().forward_word();
    if let Some(word) = word {
        lookup_inner(app, word, false).await
    } else {
        Ok(())
    }
}
#[tauri::command]
fn close_popup(app: AppHandle) -> Result<(), String> {
    stop_closed_popup_speech(&app);
    popup::hide(
        &app.get_webview_window("popup")
            .ok_or("Popup window is unavailable.")?,
    )?;
    app.state::<Hooks>()
        .visible
        .store(false, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
#[tauri::command]
async fn expand(app: AppHandle, expanded: bool, reduced_motion: bool) -> Result<(), String> {
    let settings = shared(&app).settings.lock().unwrap().clone();
    let (direction, request_id) = {
        let mut popup = shared(&app).popup.lock().unwrap();
        popup.state.expanded = expanded;
        (popup.state.direction.clone(), popup.state.request_id)
    };
    publish(&app);
    let completed = popup::animate_bounds(
        &app,
        expanded,
        &settings,
        settings.reduced_motion || reduced_motion,
        &direction,
    )
    .await?;
    if completed {
        let popup = shared(&app).popup.lock().unwrap();
        if popup.state.request_id == request_id && popup.state.expanded == expanded {
            drop(popup);
            publish(&app);
        }
    }
    Ok(())
}
/// Remembers the size the user just dragged the popup to, for the compact or expanded popup.
#[tauri::command]
fn remember_popup_size(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("popup")
        .ok_or("Popup window is unavailable.")?;
    let size = window
        .outer_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(window.scale_factor().map_err(|e| e.to_string())?);
    let dragged = Some((size.width.round(), size.height.round()));
    let mut next = shared(&app).settings.lock().unwrap().clone();
    if shared(&app).popup.lock().unwrap().state.expanded {
        next.expanded_custom = dragged;
    } else {
        next.popup_custom = dragged;
    }
    settings::update(&app, next)
}
#[tauri::command]
fn save_word(app: AppHandle, save: bool) -> Result<(), String> {
    let current = shared(&app).popup.lock().unwrap().state.clone();
    if current.status != "ready" {
        return Err("No definition to save.".into());
    }
    shared(&app).storage.lock().unwrap().save(
        entry_word(&current.query, &current.entries),
        &current.entries,
        save,
    )?;
    shared(&app).popup.lock().unwrap().state.saved = save;
    publish(&app);
    let _ = app.emit("saved-changed", ());
    Ok(())
}
#[tauri::command]
fn saved_words(app: AppHandle) -> Result<Vec<storage::SavedWord>, String> {
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
    let direction = popup::set_bounds(&app, None, expanded, &settings)?;
    let mut popup = shared(&app).popup.lock().unwrap();
    if popup.state.request_id == id {
        popup.state.status = "ready".into();
        popup.state.entries = entries;
        popup.state.saved = true;
        popup.state.direction = direction;
    }
    drop(popup);
    publish(&app);
    load_pronunciations(&app, id);
    load_synonyms(&app, id);
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

#[tauri::command]
async fn show_settings(app: AppHandle) -> Result<(), String> {
    open_settings(&app, None)
}
#[tauri::command]
fn tts_state(app: AppHandle) -> TtsState {
    shared(&app).tts.state()
}
#[tauri::command]
fn tts_voices() -> &'static [tts::VoiceInfo] {
    tts::VOICES
}
/// Speaks the displayed word in the preferred accent, or popup text in the chosen voice.
#[tauri::command]
fn tts_speak(app: AppHandle, text: String, owner: String) -> Result<(), String> {
    if !["word", "popup"].contains(&owner.as_str()) {
        return Err("Unknown speech request.".into());
    }
    let settings = shared(&app).settings.lock().unwrap().clone();
    let text = normalize_with(
        &text,
        Limits {
            max_chars: settings.reader_max_chars,
            multiline: true,
        },
    )?;
    let voice = if owner == "word" {
        word_voice(&settings)
    } else {
        tts::voice(&settings.voice).unwrap_or(&tts::VOICES[0])
    };
    shared(&app).tts.speak(tts::Request {
        text,
        voice: voice.id.into(),
        british: voice.accent == "uk",
        speed: settings.speed,
        volume: settings.volume,
        owner,
    })
}
#[tauri::command]
fn tts_pause(app: AppHandle) {
    shared(&app).tts.pause();
}
#[tauri::command]
fn tts_resume(app: AppHandle) {
    shared(&app).tts.resume();
}
#[tauri::command]
fn tts_stop(app: AppHandle, owner: Option<String>) {
    shared(&app).tts.stop(owner.as_deref());
}
#[tauri::command]
async fn tts_install(app: AppHandle) -> Result<(), String> {
    let cancel = {
        let mut install = shared(&app).install.lock().unwrap();
        if install.is_some() {
            return Err("The voice model is already downloading.".into());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *install = Some(cancel.clone());
        cancel
    };
    let state = shared(&app);
    state.tts.set_downloading(0.0);
    let mut reported = -1;
    let result = match model_store::client() {
        Ok(client) => {
            model_store::install(
                &client,
                model_store::BASE_URL,
                &state.model_dir,
                &model_store::assets(),
                &cancel,
                |done, total| {
                    let percent = (done * 100 / total.max(1)) as i64;
                    if percent != reported {
                        reported = percent;
                        state.tts.set_downloading(percent as f32 / 100.0);
                    }
                },
            )
            .await
        }
        Err(error) => Err(error),
    };
    *state.install.lock().unwrap() = None;
    match result {
        Ok(()) => {
            state.tts.set_installed(true, String::new());
            state.tts.warm();
            Ok(())
        }
        Err(error) => {
            state.tts.set_installed(false, error.clone());
            Err(error)
        }
    }
}
#[tauri::command]
fn tts_cancel_install(app: AppHandle) {
    if let Some(cancel) = shared(&app).install.lock().unwrap().as_ref() {
        cancel.store(true, Ordering::Release);
    }
}
#[tauri::command]
async fn tts_remove(app: AppHandle) -> Result<(), String> {
    if shared(&app).install.lock().unwrap().is_some() {
        return Err("Wait for the download to finish or cancel it first.".into());
    }
    shared(&app).tts.set_installed(false, String::new());
    model_store::remove(&shared(&app).model_dir)
}

fn create_tts(app: &AppHandle, model_dir: PathBuf, settings: &Settings) -> Tts {
    let runtime = app
        .path()
        .resolve(
            "resources/onnxruntime.dll",
            tauri::path::BaseDirectory::Resource,
        )
        .unwrap_or_default();
    let events = app.clone();
    let loader = app.clone();
    let dir = model_dir.clone();
    Tts::new(
        model_store::installed(&model_dir, &model_store::assets()),
        settings.tts_enabled,
        Box::new(move |state| {
            let _ = events.emit("tts-state", state);
        }),
        Box::new(move || {
            if model_store::verify(&dir, &model_store::assets()).is_err() {
                let _ = model_store::remove(&dir);
                let message =
                    "The voice model was damaged and has been removed. Install it again in Settings.";
                shared(&loader).tts.set_installed(false, message.into());
                return Err(message.into());
            }
            tts::kokoro::Kokoro::load(&dir, &runtime)
                .map(|engine| Box::new(engine) as Box<dyn tts::Backend>)
        }),
        Box::new(tts::kokoro::open_speakers),
    )
}

fn is_binding(shortcut: &Shortcut, binding: &str) -> bool {
    binding
        .parse::<Shortcut>()
        .is_ok_and(|parsed| parsed.id() == shortcut.id())
}

/// Opens Settings, on `section` when given (a notification opens it on Updates).
fn open_settings(app: &AppHandle, section: Option<&str>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window
            .show()
            .and_then(|_| window.set_focus())
            .map_err(|e| e.to_string())?;
        if let Some(section) = section {
            let _ = window.emit("settings-section", section);
        }
    } else {
        let url = match section {
            Some(section) => format!("index.html?settings&section={section}"),
            None => "index.html?settings".into(),
        };
        WebviewWindowBuilder::new(app, "settings", WebviewUrl::App(url.into()))
            .title("DIO Settings")
            .inner_size(880.0, 600.0)
            .min_inner_size(720.0, 480.0)
            .build()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn update_mode(app: &AppHandle) -> String {
    shared(app)
        .settings
        .lock()
        .unwrap()
        .update_mode()
        .to_owned()
}

/// What the user is doing, so a downloaded update never restarts DIO while it is in use.
fn update_activity(app: &AppHandle) -> updates::Activity {
    let state = shared(app);
    let popup_visible = app.state::<Hooks>().visible.load(Ordering::Relaxed)
        || app
            .get_webview_window("popup")
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false);
    updates::Activity {
        popup_visible,
        speaking: state.tts.state().active(),
        busy: state.capturing.load(Ordering::Acquire) || state.install.lock().unwrap().is_some(),
        idle_seconds: native::idle_seconds(),
    }
}

/// Sends update states to the Settings window and shows update notifications.
struct UpdateSink(AppHandle);
impl updates::Sink for UpdateSink {
    fn state(&self, state: &updates::UpdateState) {
        let _ = self.0.emit("update-state", state);
    }
    fn toast(&self, toast: updates::Toast) {
        let app = self.0.clone();
        // WinRT notifications need COM, which the main thread has.
        let _ = self.0.run_on_main_thread(move || {
            let clicked = app.clone();
            let result = toast::show(&app.config().identifier, &toast, move |arguments| {
                on_toast_click(&clicked, &arguments)
            });
            if let Err(error) = result {
                eprintln!("Could not show the update notification: {error}");
            }
        });
    }
}

fn on_toast_click(app: &AppHandle, arguments: &str) {
    match arguments {
        toast::VIEW_UPDATE => {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                let _ = open_settings(&handle, Some("updates"));
            });
        }
        toast::LATER => app.state::<Arc<Updates>>().remind_later(),
        _ => {}
    }
}

#[tauri::command]
fn update_state(updates: tauri::State<'_, Arc<Updates>>) -> updates::UpdateState {
    updates.state()
}
#[tauri::command]
fn check_for_updates(app: AppHandle, updates: tauri::State<'_, Arc<Updates>>) {
    updates.inner().check(&update_mode(&app), false);
}
#[tauri::command]
fn download_update(updates: tauri::State<'_, Arc<Updates>>) {
    updates.inner().download(false);
}
#[tauri::command]
fn cancel_update(updates: tauri::State<'_, Arc<Updates>>) {
    updates.cancel();
}
#[tauri::command]
fn retry_update(app: AppHandle, updates: tauri::State<'_, Arc<Updates>>) {
    updates.inner().retry(&update_mode(&app));
}
#[tauri::command]
fn postpone_update(updates: tauri::State<'_, Arc<Updates>>) {
    updates.postpone();
}
/// Restarts into the downloaded update at the user's request. DIO exits if the installer starts.
#[tauri::command]
fn install_update(app: AppHandle, updates: tauri::State<'_, Arc<Updates>>) {
    shared(&app).tts.stop(None);
    updates.install();
}
#[tauri::command]
fn remind_later(updates: tauri::State<'_, Arc<Updates>>) {
    updates.remind_later();
}
#[tauri::command]
fn dismiss_update(updates: tauri::State<'_, Arc<Updates>>) {
    updates.dismiss();
}
#[tauri::command]
fn open_release_notes(version: Option<String>) -> Result<(), String> {
    toast::open_url(&toast::release_notes_url(version.as_deref())?)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = open_settings(app, None);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let reader = {
                            let settings = shared(app).settings.lock().unwrap();
                            settings.reader_enabled
                                && is_binding(shortcut, &settings.reader_shortcut)
                        };
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if reader {
                                read_aloud(app).await;
                            } else {
                                activate(app).await;
                            }
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
            let (mut settings, settings_error) = match storage.settings() {
                Ok(settings) => (settings, None),
                Err(error) => (
                    Settings::default(),
                    Some(format!(
                        "{error} DIO is using default settings until you change one."
                    )),
                ),
            };
            // A settings file that failed to load is left alone rather than overwritten with defaults.
            if settings_error.is_none() {
                let registry = install_prefs::Registry::new(&app.config().identifier);
                let adoption = install_prefs::adopt(settings.update_mode.as_deref(), &registry);
                if adoption.changed {
                    settings.update_mode = Some(adoption.mode.clone());
                }
                if !adoption.changed || storage.write_settings(&settings).is_ok() {
                    let _ =
                        install_prefs::remember(&registry, &adoption.mode, adoption.from_installer);
                }
            }
            let dictionary = Dictionary::new().map_err(std::io::Error::other)?;
            let local_path = app.path().resolve(
                "resources/english.sqlite3",
                tauri::path::BaseDirectory::Resource,
            )?;
            let acronym_path = app.path().resolve(
                "resources/acronyms.sqlite3",
                tauri::path::BaseDirectory::Resource,
            )?;
            let local_dictionary =
                LocalDictionary::open(&local_path, &acronym_path).map_err(std::io::Error::other)?;
            let pronunciation_path = app.path().resolve(
                "resources/pronunciations.sqlite3",
                tauri::path::BaseDirectory::Resource,
            )?;
            tts::g2p::set_database(&pronunciation_path);
            let model_dir = model_store::directory(&directory);
            app.manage(AppState {
                dictionary,
                local_dictionary: Mutex::new(local_dictionary),
                pronunciations: Mutex::new(Pronunciations::open(&pronunciation_path)),
                reader: SelectionReader::new(),
                storage: Mutex::new(storage),
                settings: Mutex::new(settings.clone()),
                popup: Mutex::new(Popup::default()),
                motion: AtomicU64::new(0),
                shortcut_error: Mutex::new(None),
                capturing: AtomicBool::new(false),
                tts: create_tts(app.handle(), model_dir.clone(), &settings),
                model_dir,
                install: Mutex::new(None),
            });
            shared(app.handle()).tts.warm();
            app.manage(Hooks::new(app.handle().clone(), settings.middle_click));
            let updates = Updates::new(
                &app.package_info().version.to_string(),
                directory.join("update.json"),
                Arc::new(updates::PluginSource::new(app.handle().clone())),
                Arc::new(UpdateSink(app.handle().clone())),
                Arc::new(storage::now),
            );
            app.manage(updates.clone());
            let (mode_app, activity_app) = (app.handle().clone(), app.handle().clone());
            updates::schedule(
                updates,
                move || update_mode(&mode_app),
                move || update_activity(&activity_app),
            );
            let mut errors: Vec<String> = settings::bindings(&settings)
                .into_iter()
                .filter_map(|binding| {
                    app.global_shortcut()
                        .register(binding.as_str())
                        .err()
                        .map(|error| format!("Could not register {binding}: {error}."))
                })
                .collect();
            if !errors.is_empty() {
                errors.push("Choose another shortcut in Settings.".into());
            }
            errors.extend(settings_error);
            if !errors.is_empty() {
                *app.state::<AppState>().shortcut_error.lock().unwrap() = Some(errors.join(" "));
            }
            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let stop_item = MenuItem::with_id(app, "stop", "Stop reading", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit DIO", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &stop_item, &quit_item])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("DIO Dictionary")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "settings" => {
                        let _ = open_settings(app, None);
                    }
                    "stop" => shared(app).tts.stop(None),
                    "quit" => {
                        shared(app).tts.stop(None);
                        app.exit(0)
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            popup_state,
            lookup,
            back,
            forward,
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
            remember_popup_size,
            pronunciation,
            show_settings,
            tts_state,
            tts_voices,
            tts_speak,
            tts_pause,
            tts_resume,
            tts_stop,
            tts_install,
            tts_cancel_install,
            tts_remove,
            update_state,
            check_for_updates,
            download_update,
            cancel_update,
            retry_update,
            postpone_update,
            install_update,
            remind_later,
            dismiss_update,
            open_release_notes
        ])
        .run(tauri::generate_context!())
        .expect("failed to run DIO");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_failures_explain_what_to_do() {
        assert!(capture_message("Copy timed out.").contains("didn't copy"));
        assert!(capture_message(clipboard::TERMINAL_NO_SELECTION).contains("hold Shift"));
        assert!(
            capture_message("Clipboard formats cannot be safely preserved.").contains("screenshot")
        );
        assert!(capture_message("Copy input was blocked.").contains("administrator"));
        assert_eq!(
            capture_message("Select at most 2000 characters to read aloud."),
            "Select at most 2000 characters to read aloud."
        );
        assert!(capture_message("DIO can't read selections in the classic console. Use Windows Terminal or select the text elsewhere.").contains("Windows Terminal"));
        assert_eq!(
            capture_message("Accessibility timed out."),
            "Couldn't capture the selected text. Try highlighting it again."
        );
    }

    #[test]
    fn name_aliases_use_one_saved_key() {
        let resources = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources");
        let dictionary = LocalDictionary::open(
            &resources.join("english.sqlite3"),
            &resources.join("acronyms.sqlite3"),
        )
        .unwrap();
        for (alias, canonical) in [
            ("Noah Pickering", "Noah (George) Pickering"),
            ("Noah George Pickering", "Noah (George) Pickering"),
            ("Andrei Vergara", "John (Andrei) Vergara"),
            ("John Andrei Vergara", "John (Andrei) Vergara"),
        ] {
            let entries = dictionary.special(alias).unwrap().unwrap();
            assert_eq!(entry_word(alias, &entries), canonical);
        }
    }
}
