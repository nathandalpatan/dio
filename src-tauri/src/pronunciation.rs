use crate::model::{Entry, Pronunciation};
use rusqlite::{Connection, OpenFlags};
use std::collections::{HashMap, VecDeque};
use std::path::Path;

const CACHE_SIZE: usize = 256;
const PER_DIALECT: usize = 2;

pub struct Pronunciations {
    db: Option<Connection>,
    cache: HashMap<String, Vec<Pronunciation>>,
    order: VecDeque<String>,
}

impl Pronunciations {
    pub fn open(path: &Path) -> Self {
        let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| eprintln!("DIO pronunciation dictionary unavailable: {error}"))
            .ok();
        Self::with_connection(db)
    }

    fn with_connection(db: Option<Connection>) -> Self {
        Self {
            db,
            cache: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// Dictionary-sourced variants first, then Wiktionary, then CMUdict (US only when no
    /// sourced US transcription exists), then G2P output only when nothing else is known.
    pub fn resolve(&mut self, word: &str, entries: &[Entry]) -> Vec<Pronunciation> {
        let mut result = from_entries(entries);
        for item in self.lookup(word) {
            if item.source == "cmudict" && result.iter().any(|known| known.dialect == "us") {
                continue;
            }
            push_unique(&mut result, item);
        }
        if result.is_empty() {
            result.extend(generated(word));
        }
        result
    }

    fn lookup(&mut self, word: &str) -> Vec<Pronunciation> {
        let key = word.trim().to_lowercase();
        if let Some(hit) = self.cache.get(&key) {
            let hit = hit.clone();
            self.order.retain(|item| item != &key);
            self.order.push_back(key);
            return hit;
        }
        let found = match self.query(&key) {
            Ok(found) => found,
            Err(error) => {
                eprintln!("DIO pronunciation lookup failed: {error}");
                return vec![];
            }
        };
        if self.order.len() >= CACHE_SIZE {
            if let Some(oldest) = self.order.pop_front() {
                self.cache.remove(&oldest);
            }
        }
        self.cache.insert(key.clone(), found.clone());
        self.order.push_back(key);
        found
    }

    fn query(&self, key: &str) -> Result<Vec<Pronunciation>, String> {
        let Some(db) = &self.db else {
            return Ok(vec![]);
        };
        let mut found = Vec::new();
        let mut statement = db
            .prepare_cached("SELECT ipa, dialect FROM wiktionary WHERE key=?1 ORDER BY rank")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([key], |row| Ok((row.get::<_, String>(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (ipa, dialect): (String, String) = row.map_err(|e| e.to_string())?;
            if found
                .iter()
                .filter(|item: &&Pronunciation| item.dialect == dialect)
                .count()
                < PER_DIALECT
            {
                push_unique(
                    &mut found,
                    Pronunciation {
                        ipa: delimited(&ipa),
                        dialect,
                        source: "wiktionary".into(),
                    },
                );
            }
        }
        if !found.iter().any(|item| item.dialect == "us") {
            for arpabet in cmudict(db, key)?.into_iter().take(PER_DIALECT) {
                if let Some(ipa) = arpabet_to_ipa(&arpabet) {
                    push_unique(
                        &mut found,
                        Pronunciation {
                            ipa: format!("/{ipa}/"),
                            dialect: "us".into(),
                            source: "cmudict".into(),
                        },
                    );
                }
            }
        }
        Ok(found)
    }
}

pub fn cmudict(db: &Connection, key: &str) -> Result<Vec<String>, String> {
    let mut statement = db
        .prepare_cached("SELECT arpabet FROM cmudict WHERE key=?1 ORDER BY rank")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([key], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// The same transcription may be attested for both accents; an unlabelled copy of a labelled
/// transcription adds nothing.
fn push_unique(list: &mut Vec<Pronunciation>, item: Pronunciation) {
    let key = comparable(&item.ipa);
    if list.iter().any(|known| {
        comparable(&known.ipa) == key && (known.dialect == item.dialect || item.dialect.is_empty())
    }) {
        return;
    }
    if !item.dialect.is_empty() {
        list.retain(|known| !(known.dialect.is_empty() && comparable(&known.ipa) == key));
    }
    list.push(item);
}

fn comparable(ipa: &str) -> String {
    ipa.chars()
        .filter(|c| !matches!(c, '/' | '[' | ']' | '.' | ' '))
        .collect()
}

fn delimited(ipa: &str) -> String {
    let ipa = ipa.trim();
    if ipa.starts_with('/') || ipa.starts_with('[') {
        ipa.to_owned()
    } else {
        format!("/{ipa}/")
    }
}

pub fn dialect_from_audio(url: &str) -> &'static str {
    let file = url.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
    let mut parts = file.split(['_', '.', '-']);
    match parts.find(|part| matches!(*part, "us" | "uk" | "gb")) {
        Some("us") => "us",
        Some(_) => "uk",
        None => "",
    }
}

fn from_entries(entries: &[Entry]) -> Vec<Pronunciation> {
    let mut result = Vec::new();
    for entry in entries {
        for phonetic in &entry.phonetics {
            if !phonetic.text.trim().is_empty() {
                push_unique(
                    &mut result,
                    Pronunciation {
                        ipa: delimited(&phonetic.text),
                        dialect: dialect_from_audio(&phonetic.audio).into(),
                        source: "dictionary".into(),
                    },
                );
            }
        }
        if !entry.phonetic.trim().is_empty() {
            push_unique(
                &mut result,
                Pronunciation {
                    ipa: delimited(&entry.phonetic),
                    dialect: String::new(),
                    source: "dictionary".into(),
                },
            );
        }
    }
    result
}

fn generated(word: &str) -> Vec<Pronunciation> {
    if word.chars().count() > 40 {
        return vec![];
    }
    [("us", false), ("uk", true)]
        .into_iter()
        .filter_map(|(dialect, british)| {
            let phonemes = crate::tts::g2p::lexicon_phonemes(word, british)?;
            Some(Pronunciation {
                ipa: format!("/{}/", misaki_to_ipa(&phonemes, british)),
                dialect: dialect.into(),
                source: "generated".into(),
            })
        })
        .collect()
}

const VOWELS: &[(&str, &str, &str)] = &[
    // ARPABET, IPA, Misaki (Kokoro phoneme set)
    ("AA", "ɑ", "ɑ"),
    ("AE", "æ", "æ"),
    ("AH", "ʌ", "ʌ"),
    ("AO", "ɔ", "ɔ"),
    ("AW", "aʊ", "W"),
    ("AY", "aɪ", "I"),
    ("EH", "ɛ", "ɛ"),
    ("ER", "ɝ", "ɜɹ"),
    ("EY", "eɪ", "A"),
    ("IH", "ɪ", "ɪ"),
    ("IY", "i", "i"),
    ("OW", "oʊ", "O"),
    ("OY", "ɔɪ", "Y"),
    ("UH", "ʊ", "ʊ"),
    ("UW", "u", "u"),
];
const CONSONANTS: &[(&str, &str, &str)] = &[
    ("B", "b", "b"),
    ("CH", "tʃ", "ʧ"),
    ("D", "d", "d"),
    ("DH", "ð", "ð"),
    ("F", "f", "f"),
    ("G", "ɡ", "ɡ"),
    ("HH", "h", "h"),
    ("JH", "dʒ", "ʤ"),
    ("K", "k", "k"),
    ("L", "l", "l"),
    ("M", "m", "m"),
    ("N", "n", "n"),
    ("NG", "ŋ", "ŋ"),
    ("P", "p", "p"),
    ("R", "ɹ", "ɹ"),
    ("S", "s", "s"),
    ("SH", "ʃ", "ʃ"),
    ("T", "t", "t"),
    ("TH", "θ", "θ"),
    ("V", "v", "v"),
    ("W", "w", "w"),
    ("Y", "j", "j"),
    ("Z", "z", "z"),
    ("ZH", "ʒ", "ʒ"),
];
// English onsets for placing stress marks at syllable starts (maximal onset principle).
const ONSETS: &[&[&str]] = &[
    &["P", "R"],
    &["P", "L"],
    &["B", "R"],
    &["B", "L"],
    &["T", "R"],
    &["D", "R"],
    &["K", "R"],
    &["K", "L"],
    &["G", "R"],
    &["G", "L"],
    &["F", "R"],
    &["F", "L"],
    &["TH", "R"],
    &["SH", "R"],
    &["S", "P"],
    &["S", "T"],
    &["S", "K"],
    &["S", "M"],
    &["S", "N"],
    &["S", "L"],
    &["S", "W"],
    &["S", "F"],
    &["T", "W"],
    &["D", "W"],
    &["K", "W"],
    &["G", "W"],
    &["TH", "W"],
    &["P", "Y"],
    &["B", "Y"],
    &["K", "Y"],
    &["G", "Y"],
    &["F", "Y"],
    &["V", "Y"],
    &["M", "Y"],
    &["HH", "Y"],
    &["S", "P", "R"],
    &["S", "P", "L"],
    &["S", "T", "R"],
    &["S", "K", "R"],
    &["S", "K", "W"],
    &["S", "P", "Y"],
    &["S", "K", "Y"],
];

enum Phone<'a> {
    Vowel {
        ipa: &'a str,
        misaki: &'a str,
        stress: u8,
        base: &'a str,
    },
    Consonant {
        ipa: &'a str,
        misaki: &'a str,
        base: &'a str,
    },
}

fn parse(arpabet: &str) -> Option<Vec<Phone<'static>>> {
    let phones = arpabet
        .split_whitespace()
        .map(|symbol| {
            let base = symbol.trim_end_matches(|c: char| c.is_ascii_digit());
            if let Some((name, ipa, misaki)) = VOWELS.iter().find(|(name, ..)| *name == base) {
                let stress = symbol[base.len()..].parse().unwrap_or(0);
                Some(Phone::Vowel {
                    ipa: match (base, stress) {
                        ("AH", 0) => "ə",
                        ("ER", 0) => "ɚ",
                        _ => ipa,
                    },
                    misaki: match (base, stress) {
                        ("AH", 0) => "ə",
                        ("ER", 0) => "əɹ",
                        _ => misaki,
                    },
                    stress,
                    base: name,
                })
            } else {
                CONSONANTS
                    .iter()
                    .find(|(name, ..)| *name == base)
                    .map(|(name, ipa, misaki)| Phone::Consonant {
                        ipa,
                        misaki,
                        base: name,
                    })
            }
        })
        .collect::<Option<Vec<_>>>()?;
    phones
        .iter()
        .any(|phone| matches!(phone, Phone::Vowel { .. }))
        .then_some(phones)
}

/// Converts a CMUdict ARPABET transcription into broad American IPA, placing stress marks
/// before the syllable onset.
pub fn arpabet_to_ipa(arpabet: &str) -> Option<String> {
    let phones = parse(arpabet)?;
    let mut marks = vec![""; phones.len()];
    let mut previous_vowel = None;
    for (index, phone) in phones.iter().enumerate() {
        let Phone::Vowel { stress, .. } = phone else {
            continue;
        };
        let mark = match stress {
            1 => "ˈ",
            2 => "ˌ",
            _ => "",
        };
        let start = match previous_vowel {
            None => 0,
            Some(previous) => {
                let cluster: Vec<&str> = phones[previous + 1..index]
                    .iter()
                    .map(|phone| match phone {
                        Phone::Consonant { base, .. } => *base,
                        Phone::Vowel { base, .. } => base,
                    })
                    .collect();
                let onset = (0..=cluster.len().min(3))
                    .rev()
                    .find(|length| {
                        let suffix = &cluster[cluster.len() - length..];
                        *length == 0
                            || (*length == 1 && suffix[0] != "NG")
                            || ONSETS.contains(&suffix)
                    })
                    .unwrap_or(0);
                index - onset
            }
        };
        marks[start] = mark;
        previous_vowel = Some(index);
    }
    Some(
        phones
            .iter()
            .zip(marks)
            .map(|(phone, mark)| {
                let ipa = match phone {
                    Phone::Vowel { ipa, .. } | Phone::Consonant { ipa, .. } => *ipa,
                };
                format!("{mark}{ipa}")
            })
            .collect(),
    )
}

/// Converts ARPABET into Kokoro's Misaki phoneme set, with stress before the vowel as Misaki
/// writes it.
pub fn arpabet_to_misaki(arpabet: &str) -> Option<String> {
    Some(
        parse(arpabet)?
            .iter()
            .map(|phone| match phone {
                Phone::Vowel { misaki, stress, .. } => format!(
                    "{}{misaki}",
                    match stress {
                        1 => "ˈ",
                        2 => "ˌ",
                        _ => "",
                    }
                ),
                Phone::Consonant { misaki, .. } => misaki.to_string(),
            })
            .collect(),
    )
}

/// Expands Misaki's single-letter diphthongs and Kokoro-specific symbols into IPA.
pub fn misaki_to_ipa(phonemes: &str, british: bool) -> String {
    let mut result = String::new();
    for c in phonemes.chars() {
        match c {
            'A' => result.push_str("eɪ"),
            'I' => result.push_str("aɪ"),
            'W' => result.push_str("aʊ"),
            'Y' => result.push_str("ɔɪ"),
            'O' => result.push_str("oʊ"),
            'Q' => result.push_str("əʊ"),
            'T' if !british => result.push_str("t̬"),
            'ᵊ' => result.push('ə'),
            'ʤ' => result.push_str("dʒ"),
            'ʧ' => result.push_str("tʃ"),
            c if c.is_alphabetic() || matches!(c, 'ˈ' | 'ˌ' | 'ː' | '\u{303}') => result.push(c),
            _ => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Phonetic;

    fn store() -> Pronunciations {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE wiktionary (key TEXT, ipa TEXT, dialect TEXT, rank INTEGER);
             CREATE TABLE cmudict (key TEXT, arpabet TEXT, rank INTEGER);
             INSERT INTO wiktionary VALUES ('tomato', '/təˈmeɪ.toʊ/', 'us', 0);
             INSERT INTO wiktionary VALUES ('tomato', '/təˈmɑː.təʊ/', 'uk', 1);
             INSERT INTO cmudict VALUES ('tomato', 'T AH0 M EY1 T OW2', 0);
             INSERT INTO cmudict VALUES ('serendipity', 'S EH2 R AH0 N D IH1 P IH0 T IY0', 0);
             INSERT INTO wiktionary VALUES ('naïve', '/naɪˈiːv/', 'uk', 0);",
        )
        .unwrap();
        Pronunciations::with_connection(Some(db))
    }

    #[test]
    fn converts_arpabet_with_syllable_stress() {
        for (arpabet, ipa) in [
            ("S EH2 R AH0 N D IH1 P IH0 T IY0", "ˌsɛɹənˈdɪpɪti"),
            ("K AH0 N S T R AH1 K T", "kənˈstɹʌkt"),
            ("EH1 K S T R AH0", "ˈɛkstɹə"),
            ("B ER1 D", "ˈbɝd"),
            ("B AH1 T ER0", "ˈbʌtɚ"),
            ("AH0 B AW1 T", "əˈbaʊt"),
            ("CH OY1 S", "ˈtʃɔɪs"),
            ("S IH1 NG ER0", "ˈsɪŋɚ"),
        ] {
            assert_eq!(arpabet_to_ipa(arpabet).as_deref(), Some(ipa), "{arpabet}");
        }
        assert!(arpabet_to_ipa("XX Y").is_none());
        assert!(arpabet_to_ipa("S T").is_none());
    }

    #[test]
    fn converts_arpabet_to_kokoro_phonemes() {
        assert_eq!(
            arpabet_to_misaki("S EH2 R AH0 N D IH1 P IH0 T IY0").as_deref(),
            Some("sˌɛɹəndˈɪpɪti")
        );
        assert_eq!(arpabet_to_misaki("B ER1 D OW0").as_deref(), Some("bˈɜɹdO"));
    }

    #[test]
    fn expands_misaki_symbols_into_ipa() {
        assert_eq!(misaki_to_ipa("sˌɛɹəndˈɪpəTi", false), "sˌɛɹəndˈɪpət̬i");
        assert_eq!(misaki_to_ipa("hˈQm ❓.", true), "hˈəʊm");
        assert_eq!(misaki_to_ipa("ʤˈAʧ", false), "dʒˈeɪtʃ");
    }

    #[test]
    fn reads_dialect_only_from_labelled_audio() {
        assert_eq!(dialect_from_audio("https://x/en/serendipity-us.mp3"), "us");
        assert_eq!(dialect_from_audio("https://x/en/tomato-uk.mp3"), "uk");
        assert_eq!(dialect_from_audio("https://x/En-gb-tomato.ogg"), "uk");
        assert_eq!(dialect_from_audio("https://x/en/tomato-au.mp3"), "");
        assert_eq!(dialect_from_audio("https://us.example/campus.mp3"), "");
        assert_eq!(dialect_from_audio(""), "");
    }

    #[test]
    fn prioritizes_sources_and_replaces_unlabelled_duplicates() {
        let entries = vec![Entry {
            phonetics: vec![
                Phonetic {
                    text: "/təˈmeɪ.toʊ/".into(),
                    audio: "https://x/tomato-us.mp3".into(),
                    ..Default::default()
                },
                Phonetic {
                    text: "/təˈmɑːtəʊ/".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }];
        let result = store().resolve("Tomato", &entries);
        assert_eq!(
            result
                .iter()
                .map(|item| (item.dialect.as_str(), item.source.as_str()))
                .collect::<Vec<_>>(),
            [("us", "dictionary"), ("uk", "wiktionary")]
        );
        assert_eq!(result[1].ipa, "/təˈmɑː.təʊ/");
    }

    #[test]
    fn falls_back_to_cmudict_as_us_only() {
        let result = store().resolve("serendipity", &[]);
        assert_eq!(
            result,
            [Pronunciation {
                ipa: "/ˌsɛɹənˈdɪpɪti/".into(),
                dialect: "us".into(),
                source: "cmudict".into(),
            }]
        );
    }

    #[test]
    fn preserves_unicode_and_never_invents_an_accent() {
        let result = store().resolve("NAÏVE", &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].ipa, "/naɪˈiːv/");
        assert_eq!(result[0].dialect, "uk");
        let entries = vec![Entry {
            phonetic: "t̬ə.ˈmeɪ̯".into(),
            ..Default::default()
        }];
        let result = Pronunciations::with_connection(None).resolve("tomato", &entries);
        assert_eq!(result[0].ipa, "/t̬ə.ˈmeɪ̯/");
        assert_eq!(result[0].dialect, "");
    }

    #[test]
    fn caches_lookups_with_a_bounded_size() {
        let mut store = store();
        assert_eq!(store.lookup("tomato").len(), 2);
        store.db = None;
        assert_eq!(store.lookup("TOMATO").len(), 2);
        for index in 0..CACHE_SIZE {
            store.lookup(&format!("word{index}"));
        }
        assert_eq!(store.cache.len(), CACHE_SIZE);
        assert!(store.lookup("tomato").is_empty());
    }

    #[test]
    fn missing_database_yields_empty_or_generated_results() {
        let mut store = Pronunciations::open(Path::new("Z:/missing/pronunciations.sqlite3"));
        assert!(store.lookup("serendipity").is_empty());
        let generated = store.resolve("serendipity", &[]);
        assert!(generated
            .iter()
            .all(|item| item.source == "generated" && !item.ipa.contains('❓')));
        assert!(store.resolve("qzxwv", &[]).is_empty());
    }
}
