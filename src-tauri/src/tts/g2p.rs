use misaki_rs::{Language, G2P};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static US: OnceLock<G2P> = OnceLock::new();
static GB: OnceLock<G2P> = OnceLock::new();
static CMUDICT: OnceLock<Mutex<Option<Connection>>> = OnceLock::new();

fn engine(british: bool) -> &'static G2P {
    if british {
        GB.get_or_init(|| G2P::new(Language::EnglishGB))
    } else {
        US.get_or_init(|| G2P::new(Language::EnglishUS))
    }
}

/// Opens the bundled pronunciation index so speech can read words Misaki's lexicon lacks.
pub fn set_database(path: &Path) {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok();
    let _ = CMUDICT.set(Mutex::new(connection));
}

/// Kokoro phonemes for a single word from Misaki's lexicon and rules, or None when Misaki
/// cannot resolve every part of it.
pub fn lexicon_phonemes(word: &str, british: bool) -> Option<String> {
    if !word.chars().any(char::is_alphabetic) {
        return None;
    }
    let (_, tokens) = engine(british).g2p(word).ok()?;
    let mut phonemes = String::new();
    for token in tokens {
        if token.text.chars().any(char::is_alphanumeric) {
            phonemes.push_str(token.phonemes.as_deref()?);
            phonemes.push_str(&token.whitespace);
        }
    }
    let phonemes = phonemes.trim();
    (!phonemes.is_empty() && !phonemes.contains('❓')).then(|| phonemes.to_owned())
}

/// Kokoro phonemes for running text. Words outside Misaki's lexicon use CMUdict, then
/// letter names, so speech never contains an unknown marker.
pub fn speech_phonemes(text: &str, british: bool) -> String {
    let Ok((_, tokens)) = engine(british).g2p(text) else {
        return String::new();
    };
    tokens
        .iter()
        .map(|token| {
            let phonemes = token
                .phonemes
                .clone()
                .filter(|value| !value.contains('❓'))
                .or_else(|| fallback(&token.text, british))
                .unwrap_or_default();
            let space = if token.whitespace.is_empty() { "" } else { " " };
            phonemes + space
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

fn fallback(word: &str, british: bool) -> Option<String> {
    let key = word.to_lowercase();
    let from_cmudict = CMUDICT.get().and_then(|database| {
        let database = database.lock().ok()?;
        crate::pronunciation::cmudict(database.as_ref()?, &key)
            .ok()?
            .first()
            .and_then(|arpabet| crate::pronunciation::arpabet_to_misaki(arpabet))
    });
    from_cmudict.or_else(|| spell(&key, british))
}

fn spell(word: &str, british: bool) -> Option<String> {
    let letters: Vec<&str> = word
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|letter| match letter {
            'a' => "ˈA",
            'b' => "bˈi",
            'c' => "sˈi",
            'd' => "dˈi",
            'e' => "ˈi",
            'f' => "ˈɛf",
            'g' => "ʤˈi",
            'h' => "ˈAʧ",
            'i' => "ˈI",
            'j' => "ʤˈA",
            'k' => "kˈA",
            'l' => "ˈɛl",
            'm' => "ˈɛm",
            'n' => "ˈɛn",
            'o' if british => "ˈQ",
            'o' => "ˈO",
            'p' => "pˈi",
            'q' => "kjˈu",
            'r' if british => "ˈɑː",
            'r' => "ˈɑɹ",
            's' => "ˈɛs",
            't' => "tˈi",
            'u' => "jˈu",
            'v' => "vˈi",
            'w' => "dˈʌbᵊljˌu",
            'x' => "ˈɛks",
            'y' => "wˈI",
            'z' if british => "zˈɛd",
            _ => "zˈi",
        })
        .collect();
    (!letters.is_empty()).then(|| letters.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_words_use_the_lexicon_and_unknown_words_do_not() {
        assert_eq!(
            lexicon_phonemes("serendipity", false).as_deref(),
            Some("sˌɛɹəndˈɪpəTi")
        );
        assert!(lexicon_phonemes("serendipity", true).is_some_and(|value| !value.contains('T')));
        assert!(lexicon_phonemes("qzxwv", false).is_none());
        assert!(lexicon_phonemes("123", false).is_none());
    }

    #[test]
    fn speech_never_contains_unknown_markers() {
        let phonemes = speech_phonemes("Say qzx now.", false);
        assert!(!phonemes.contains('❓'));
        assert!(phonemes.contains("kjˈu zˈi ˈɛks"));
        assert_eq!(spell("oz", true).as_deref(), Some("ˈQ zˈɛd"));
    }
}
