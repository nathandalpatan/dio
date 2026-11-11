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
        if self.shortcut.len() > 100 || !self.shortcut.contains('+') {
            return Err("Choose a shortcut with a modifier key.".into());
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
        }
    }
}
pub fn normalize(text: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Highlight a word before activating DIO.".into());
    }
    if text.chars().count() > 128 {
        return Err("Select a word or short expression of at most 128 characters.".into());
    }
    if text.chars().any(char::is_control) {
        return Err("Select a word or short expression on one line.".into());
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
    fn absent_provider_fields_are_optional() {
        let entry: Entry = serde_json::from_str(r#"{"word":"test","meanings":[{"partOfSpeech":"noun","definitions":[{"definition":"A trial."}]}]}"#).unwrap();
        assert!(entry.phonetics.is_empty());
        assert!(entry.license.is_none());
    }
}
