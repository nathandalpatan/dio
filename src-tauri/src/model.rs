use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub shortcut: String,
    pub keyboard_enabled: bool,
    pub middle_click: bool,
    pub theme: String,
    pub size: String,
    pub expanded: bool,
    pub autoplay: bool,
    pub pronunciation: String,
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
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "Ctrl+Alt+D".into(),
            keyboard_enabled: true,
            middle_click: false,
            theme: "system".into(),
            size: "standard".into(),
            expanded: false,
            autoplay: false,
            pronunciation: "first".into(),
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
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !["system", "light", "dark"].contains(&self.theme.as_str())
            || !["small", "standard", "large"].contains(&self.size.as_str())
            || !["first", "us", "uk"].contains(&self.pronunciation.as_str())
        {
            return Err("Invalid appearance or pronunciation preference.".into());
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
    pub saved: bool,
    pub pronunciations: Vec<Pronunciation>,
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
            saved: false,
            pronunciations: vec![],
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
        let settings: Settings =
            serde_json::from_str(r#"{"shortcut":"Ctrl+Alt+D","theme":"dark"}"#).unwrap();
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
    fn absent_provider_fields_are_optional() {
        let entry: Entry = serde_json::from_str(r#"{"word":"test","meanings":[{"partOfSpeech":"noun","definitions":[{"definition":"A trial."}]}]}"#).unwrap();
        assert!(entry.phonetics.is_empty());
        assert!(entry.license.is_none());
    }
}
