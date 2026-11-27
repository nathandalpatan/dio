use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub shortcut: String,
    pub keyboard_enabled: bool,
    pub middle_click: bool,
    pub theme: String,
    pub size: String,
    /// compact, standard or large. Independent of the popup's size.
    pub text_size: String,
    pub expanded: bool,
    pub autoplay: bool,
    pub startup: bool,
    pub reduced_motion: bool,
    pub accent: String,
    pub tts_enabled: bool,
    pub voice: String,
    pub speed: f32,
    pub volume: f32,
    pub reader_enabled: bool,
    pub reader_shortcut: String,
    pub reader_max_chars: usize,
    /// Size the user dragged the compact popup to, in logical pixels. `None` sizes it automatically.
    pub popup_custom: Option<(f64, f64)>,
    /// Size the user dragged the expanded popup to, in logical pixels.
    pub expanded_custom: Option<(f64, f64)>,
    /// auto, notify or manual. `None` until the installer's choice or the default is adopted at startup.
    pub update_mode: Option<String>,
}
pub const UPDATE_MODES: [&str; 3] = ["auto", "notify", "manual"];
/// Smallest popup the user can resize to, in logical pixels. Matches the popup window's minimum size.
pub const MIN_POPUP: (f64, f64) = (300.0, 220.0);
impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "Ctrl+Alt+D".into(),
            keyboard_enabled: true,
            middle_click: false,
            theme: "system".into(),
            size: "standard".into(),
            text_size: "compact".into(),
            expanded: false,
            autoplay: false,
            startup: false,
            reduced_motion: false,
            accent: "us".into(),
            tts_enabled: true,
            voice: "af_heart".into(),
            speed: 1.0,
            volume: 1.0,
            reader_enabled: true,
            reader_shortcut: "Ctrl+Alt+R".into(),
            reader_max_chars: 2000,
            popup_custom: None,
            expanded_custom: None,
            update_mode: None,
        }
    }
}
impl Settings {
    pub fn update_mode(&self) -> &str {
        self.update_mode.as_deref().unwrap_or("auto")
    }
    pub fn validate(&self) -> Result<(), String> {
        if !["system", "light", "dark"].contains(&self.theme.as_str())
            || !["small", "standard", "large"].contains(&self.size.as_str())
            || !["compact", "standard", "large"].contains(&self.text_size.as_str())
        {
            return Err("Invalid appearance preference.".into());
        }
        for shortcut in [&self.shortcut, &self.reader_shortcut] {
            if shortcut.len() > 100 || !shortcut.contains('+') {
                return Err("Choose a shortcut with a modifier key.".into());
            }
        }
        if self.shortcut.eq_ignore_ascii_case(&self.reader_shortcut) {
            return Err("The dictionary and read-aloud shortcuts must be different.".into());
        }
        if !["us", "uk"].contains(&self.accent.as_str())
            || crate::tts::voice(&self.voice).is_none()
            || !(0.5..=2.0).contains(&self.speed)
            || !(0.0..=1.0).contains(&self.volume)
            || !(100..=10_000).contains(&self.reader_max_chars)
        {
            return Err("Invalid speech preference.".into());
        }
        let valid = |(width, height): (f64, f64)| {
            (MIN_POPUP.0..=4000.0).contains(&width) && (MIN_POPUP.1..=4000.0).contains(&height)
        };
        if !self.popup_custom.is_none_or(valid) || !self.expanded_custom.is_none_or(valid) {
            return Err("Invalid popup size.".into());
        }
        if !self
            .update_mode
            .as_deref()
            .is_none_or(|mode| UPDATE_MODES.contains(&mode))
        {
            return Err("Invalid update preference.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct License {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phonetic {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub audio: String,
    #[serde(default)]
    pub source_url: String,
    pub license: Option<License>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Pronunciation {
    pub ipa: String,
    pub dialect: String,
    pub source: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Definition {
    #[serde(default)]
    pub definition: String,
    #[serde(default)]
    pub example: String,
    #[serde(default)]
    pub synonyms: Vec<String>,
    #[serde(default)]
    pub antonyms: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meaning {
    #[serde(default)]
    pub part_of_speech: String,
    #[serde(default)]
    pub definitions: Vec<Definition>,
    #[serde(default)]
    pub synonyms: Vec<String>,
    #[serde(default)]
    pub antonyms: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    #[serde(default)]
    pub word: String,
    #[serde(default)]
    pub phonetic: String,
    #[serde(default)]
    pub phonetics: Vec<Phonetic>,
    #[serde(default)]
    pub meanings: Vec<Meaning>,
    #[serde(default)]
    pub origin: String,
    #[serde(default)]
    pub source_urls: Vec<String>,
    pub license: Option<License>,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Debug)]
pub struct Selection {
    pub text: String,
    pub bounds: Option<Rect>,
}
/// One WordNet sense with the other words that share it.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SynonymSense {
    pub part_of_speech: String,
    pub definition: String,
    pub synonyms: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PopupState {
    pub request_id: u64,
    pub status: String,
    pub query: String,
    pub entries: Vec<Entry>,
    pub message: String,
    pub stale: bool,
    pub expanded: bool,
    pub direction: String,
    pub can_back: bool,
    pub can_forward: bool,
    pub saved: bool,
    pub pronunciations: Vec<Pronunciation>,
    pub synonym_senses: Vec<SynonymSense>,
    pub reading: String,
}
impl Default for PopupState {
    fn default() -> Self {
        Self {
            request_id: 0,
            status: "idle".into(),
            query: String::new(),
            entries: vec![],
            message: String::new(),
            stale: false,
            expanded: false,
            direction: "se".into(),
            can_back: false,
            can_forward: false,
            saved: false,
            pronunciations: vec![],
            synonym_senses: vec![],
            reading: String::new(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub max_chars: usize,
    pub multiline: bool,
}
pub const WORD_LIMITS: Limits = Limits {
    max_chars: 128,
    multiline: false,
};
pub fn normalize(text: &str) -> Result<String, String> {
    normalize_with(text, WORD_LIMITS)
}
/// Double-clicking a word in some apps (Adobe Acrobat) also selects the punctuation after it.
/// A trailing period is kept when the word has another one, as in "e.g." or "U.S.".
pub fn trim_selected_word(text: &str) -> &str {
    const PUNCTUATION: &[char] = &[
        ',', ';', ':', '!', '?', '"', '“', '”', '‘', '’', '(', ')', '[', ']', '«', '»', '…',
    ];
    let text = text.trim().trim_matches(PUNCTUATION);
    let text = match text.strip_suffix('.') {
        Some(rest) if !rest.contains('.') => rest,
        _ => text,
    };
    text.trim_matches(PUNCTUATION).trim()
}
/// A terminal's word at a point can include its neighbours: spaces, bullets and box lines.
/// `+` and `#` stay for words such as C++ and C#; periods are settled by `trim_selected_word`.
pub fn trim_word_at_point(text: &str) -> &str {
    trim_selected_word(
        text.trim_matches(|c: char| !(c.is_alphanumeric() || matches!(c, '+' | '#' | '.'))),
    )
}
pub fn normalize_with(text: &str, limits: Limits) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Highlight a word before activating DIO.".into());
    }
    if text.chars().count() > limits.max_chars {
        return Err(if limits.multiline {
            format!(
                "Select at most {} characters to read aloud.",
                limits.max_chars
            )
        } else {
            format!(
                "Select a word or short expression of at most {} characters.",
                limits.max_chars
            )
        });
    }
    if text
        .chars()
        .any(|c| c.is_control() && !(limits.multiline && matches!(c, '\n' | '\r' | '\t')))
    {
        return Err(if limits.multiline {
            "The selection contains unsupported characters.".into()
        } else {
            "Select a word or short expression on one line.".into()
        });
    }
    Ok(text.to_owned())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalization_preserves_expressions() {
        assert_eq!(normalize("  ice cream  ").unwrap(), "ice cream");
    }
    #[test]
    fn selected_words_lose_surrounding_punctuation() {
        assert_eq!(trim_selected_word("acknowledged, "), "acknowledged");
        assert_eq!(trim_selected_word("“Bennet,”"), "Bennet");
        assert_eq!(trim_selected_word("(fortune)."), "fortune");
        assert_eq!(trim_selected_word("wife."), "wife");
        assert_eq!(trim_selected_word("e.g."), "e.g.");
        assert_eq!(trim_selected_word("C++"), "C++");
        assert_eq!(trim_selected_word("C#"), "C#");
        assert_eq!(trim_selected_word("rock 'n' roll"), "rock 'n' roll");
    }
    #[test]
    fn terminal_words_lose_their_neighbours() {
        assert_eq!(trim_word_at_point("· summary          "), "summary");
        assert_eq!(trim_word_at_point("it.                 "), "it");
        assert_eq!(trim_word_at_point("│ bisect "), "bisect");
        assert_eq!(trim_word_at_point("`idempotent`"), "idempotent");
        assert_eq!(trim_word_at_point("C++ "), "C++");
        assert_eq!(trim_word_at_point("e.g. "), "e.g.");
        assert_eq!(trim_word_at_point("  ──  "), "");
    }
    #[test]
    fn rejects_invalid_selection() {
        for value in [" ".to_string(), "x".repeat(129), "one\ntwo".into()] {
            assert!(normalize(&value).is_err());
        }
        assert!(normalize(&"é".repeat(128)).is_ok());
    }
    #[test]
    fn reading_limits_allow_paragraphs_but_stay_bounded() {
        let limits = Limits {
            max_chars: 200,
            multiline: true,
        };
        assert_eq!(
            normalize_with(" one\r\ntwo\tthree ", limits).unwrap(),
            "one\r\ntwo\tthree"
        );
        assert!(normalize_with(&"a".repeat(201), limits).is_err());
        assert!(normalize_with("bell\u{7}", limits).is_err());
        assert!(normalize("one\ntwo").is_err());
    }
    #[test]
    fn older_settings_files_gain_speech_defaults() {
        let settings: Settings = serde_json::from_str(
            r#"{"shortcut":"Ctrl+Alt+D","theme":"dark","pronunciation":"uk"}"#,
        )
        .unwrap();
        assert_eq!(settings.theme, "dark");
        assert_eq!(settings.voice, "af_heart");
        assert_eq!(settings.reader_shortcut, "Ctrl+Alt+R");
        assert!(settings.validate().is_ok());
    }
    #[test]
    fn speech_settings_are_validated() {
        let valid = Settings::default();
        for invalid in [
            Settings {
                reader_shortcut: "ctrl+alt+d".into(),
                ..valid.clone()
            },
            Settings {
                speed: 3.0,
                ..valid.clone()
            },
            Settings {
                volume: -0.1,
                ..valid.clone()
            },
            Settings {
                voice: "zz_unknown".into(),
                ..valid.clone()
            },
            Settings {
                accent: "au".into(),
                ..valid.clone()
            },
            Settings {
                reader_max_chars: 50_000,
                ..valid.clone()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }
    #[test]
    fn text_size_is_validated_and_defaults_to_compact() {
        let old: Settings = serde_json::from_str(r#"{"theme":"dark","size":"standard"}"#).unwrap();
        assert_eq!(old.text_size, "compact");
        for size in ["compact", "standard", "large"] {
            assert!(Settings {
                text_size: size.into(),
                ..Settings::default()
            }
            .validate()
            .is_ok());
        }
        assert!(Settings {
            text_size: "huge".into(),
            ..Settings::default()
        }
        .validate()
        .is_err());
    }
    #[test]
    fn update_mode_is_optional_and_validated() {
        let old: Settings = serde_json::from_str(r#"{"theme":"dark"}"#).unwrap();
        assert_eq!(old.update_mode, None);
        assert_eq!(old.update_mode(), "auto");
        for mode in UPDATE_MODES {
            let settings = Settings {
                update_mode: Some(mode.into()),
                ..Settings::default()
            };
            assert!(settings.validate().is_ok());
            assert_eq!(settings.update_mode(), mode);
        }
        assert!(Settings {
            update_mode: Some("sometimes".into()),
            ..Settings::default()
        }
        .validate()
        .is_err());
    }
    #[test]
    fn remembered_popup_sizes_are_validated() {
        let valid = Settings {
            popup_custom: Some((420.0, 360.0)),
            expanded_custom: Some((640.0, 480.0)),
            ..Settings::default()
        };
        assert!(valid.validate().is_ok());
        for size in [(100.0, 360.0), (420.0, f64::NAN), (5000.0, 400.0)] {
            assert!(Settings {
                popup_custom: Some(size),
                ..valid.clone()
            }
            .validate()
            .is_err());
        }
    }
    #[test]
    fn absent_provider_fields_are_optional() {
        let entry: Entry = serde_json::from_str(r#"{"word":"test","meanings":[{"partOfSpeech":"noun","definitions":[{"definition":"A trial."}]}]}"#).unwrap();
        assert!(entry.phonetics.is_empty());
        assert!(entry.license.is_none());
    }
}
