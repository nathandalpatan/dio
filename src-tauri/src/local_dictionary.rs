use crate::model::{Definition, Entry, License, Meaning};
use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;

pub struct LocalDictionary {
    db: Connection,
    acronyms: Connection,
}

impl LocalDictionary {
    pub fn open(path: &Path, acronym_path: &Path) -> Result<Self, String> {
        let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("Cannot open local dictionary: {e}"))?;
        let acronyms = Connection::open_with_flags(acronym_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("Cannot open acronym dictionary: {e}"))?;
        Ok(Self { db, acronyms })
    }

    pub fn lookup(&self, word: &str) -> Result<Option<Vec<Entry>>, String> {
        if let Some((name, description)) = [
            (
                &["Connor Knoetze"][..],
                "Connor Knoetze",
                "An avid biker and talented coder, currently between roles, whose portfolio website remains a work in progress after roughly $500 spent on AI tokens.",
            ),
            (
                &["Noah Pickering", "Noah George Pickering", "Noah (George) Pickering"][..],
                "Noah (George) Pickering",
                "A Fortnite pro and academic weapon, equally at home securing a Victory Royale and acing an exam.",
            ),
            (
                &["John Vergara", "Andrei Vergara", "John Andrei Vergara", "John (Andrei) Vergara"][..],
                "John (Andrei) Vergara",
                "A talented videographer, photographer, motion designer and editor, with a creative eye and an unapologetically chud streak.",
            ),
        ]
        .into_iter()
        .find(|(names, _, _)| names.iter().any(|name| word.eq_ignore_ascii_case(name)))
        .map(|(_, name, description)| (name, description))
        {
            return Ok(Some(vec![Entry {
                word: name.into(),
                meanings: vec![Meaning {
                    part_of_speech: "proper noun".into(),
                    definitions: vec![Definition {
                        definition: description.into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }]));
        }
        if word.eq_ignore_ascii_case("dio") {
            return Ok(Some(vec![Entry {
                word: "DIO".into(),
                meanings: vec![Meaning {
                    part_of_speech: "noun".into(),
                    definitions: vec![Definition {
                        definition: "A small desktop dictionary that turns highlighted words into instant definitions.".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }]));
        }
        if word.chars().any(char::is_alphabetic)
            && word
                .chars()
                .filter(|c| c.is_alphabetic())
                .all(char::is_uppercase)
        {
            if let Some(entry) = self.lookup_acronym(word)? {
                return Ok(Some(vec![entry]));
            }
        }
        let normalized = word.to_lowercase();
        let mut candidates = vec![normalized.clone()];
        if let Some(stem) = normalized.strip_suffix("ies") {
            candidates.push(format!("{stem}y"));
        }
        if let Some(stem) = normalized.strip_suffix("s") {
            candidates.push(stem.to_string());
        }
        if let Some(stem) = normalized.strip_suffix("es") {
            candidates.push(stem.to_string());
        }
        for candidate in candidates {
            if let Some(entry) = self.lookup_exact(&candidate, word)? {
                return Ok(Some(vec![entry]));
            }
        }
        Ok(self.lookup_acronym(word)?.map(|entry| vec![entry]))
    }

    fn lookup_acronym(&self, word: &str) -> Result<Option<Entry>, String> {
        let mut statement = self
            .acronyms
            .prepare("SELECT term, kind, COALESCE(explanation, definition) FROM acronyms WHERE key=?1 ORDER BY priority DESC, term=?2 DESC, explanation IS NULL, rowid")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![word.to_lowercase(), word], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut term = None;
        let mut meanings: Vec<Meaning> = Vec::new();
        for row in rows {
            let (spelling, kind, definition) = row.map_err(|e| e.to_string())?;
            if term.is_none() {
                term = Some(spelling);
            }
            if let Some(meaning) = meanings
                .iter_mut()
                .find(|meaning| meaning.part_of_speech == kind)
            {
                meaning.definitions.push(Definition {
                    definition,
                    ..Default::default()
                });
            } else {
                meanings.push(Meaning {
                    part_of_speech: kind,
                    definitions: vec![Definition {
                        definition,
                        ..Default::default()
                    }],
                    ..Default::default()
                });
            }
        }
        let Some(term) = term else { return Ok(None) };
        let mut source = reqwest::Url::parse("https://en.wiktionary.org/wiki/").expect("fixed URL");
        source
            .path_segments_mut()
            .expect("base URL")
            .pop_if_empty()
            .push(&term);
        Ok(Some(Entry {
            word: term,
            meanings,
            source_urls: vec![source.into()],
            license: Some(License {
                name: "Wiktionary contributors · CC BY-SA 4.0".into(),
                url: "https://creativecommons.org/licenses/by-sa/4.0/".into(),
            }),
            ..Default::default()
        }))
    }

    fn lookup_exact(&self, candidate: &str, display_word: &str) -> Result<Option<Entry>, String> {
        let mut statement = self
            .db
            .prepare("SELECT pos, definition, example, synset_id FROM definitions WHERE word=?1")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([candidate], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut meanings: Vec<Meaning> = Vec::new();
        for row in rows {
            let (part_of_speech, definition, example, synset_id) =
                row.map_err(|e| e.to_string())?;
            let synonyms = if synset_id.is_empty() {
                Vec::new()
            } else {
                let mut statement = self
                    .db
                    .prepare("SELECT DISTINCT word FROM definitions WHERE synset_id=?1 AND word<>?2 LIMIT 12")
                    .map_err(|e| e.to_string())?;
                let words = statement
                    .query_map(params![synset_id, candidate], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<String>, _>>()
                    .map_err(|e| e.to_string())?;
                words
            };
            let definition = Definition {
                definition,
                example,
                synonyms,
                ..Default::default()
            };
            if let Some(meaning) = meanings
                .iter_mut()
                .find(|meaning| meaning.part_of_speech == part_of_speech)
            {
                meaning.definitions.push(definition);
            } else {
                meanings.push(Meaning {
                    part_of_speech,
                    definitions: vec![definition],
                    ..Default::default()
                });
            }
        }
        if meanings.is_empty() {
            return Ok(None);
        }
        Ok(Some(Entry {
            word: display_word.to_string(),
            meanings,
            source_urls: vec!["https://github.com/CloudBytes-Academy/English-Dictionary-Open-Source/tree/ef5562840cf57c7682fc63e7e35ddb616638f93b/v2".into()],
            license: Some(License {
                name: "Open English WordNet Team · CC BY 4.0".into(),
                url: "https://github.com/CloudBytes-Academy/English-Dictionary-Open-Source/blob/ef5562840cf57c7682fc63e7e35ddb616638f93b/v2/LICENSE-DATA.md".into(),
            }),
            ..Default::default()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_dictionary_defines_words_offline() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/english.sqlite3");
        let acronyms = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/acronyms.sqlite3");
        let dictionary = LocalDictionary::open(&path, &acronyms).unwrap();
        for word in ["another", "instruction", "Blockers"] {
            let entries = dictionary.lookup(word).unwrap().unwrap();
            assert!(!entries[0].meanings[0].definitions[0].definition.is_empty());
        }
        let dio = dictionary.lookup("dio").unwrap().unwrap();
        assert_eq!(dio[0].word, "DIO");
        assert!(dio[0].meanings[0].definitions[0]
            .definition
            .contains("desktop dictionary"));
        let connor = dictionary.lookup("connor knoetze").unwrap().unwrap();
        assert_eq!(connor[0].word, "Connor Knoetze");
        assert!(connor[0].meanings[0].definitions[0]
            .definition
            .contains("portfolio website"));
        for alias in [
            "NOAH PICKERING",
            "Noah George Pickering",
            "Noah (George) Pickering",
        ] {
            let entry = dictionary.lookup(alias).unwrap().unwrap();
            assert_eq!(entry[0].word, "Noah (George) Pickering");
            assert!(entry[0].meanings[0].definitions[0]
                .definition
                .contains("Victory Royale"));
        }
        for alias in [
            "john vergara",
            "Andrei Vergara",
            "John Andrei Vergara",
            "John (Andrei) Vergara",
        ] {
            let entry = dictionary.lookup(alias).unwrap().unwrap();
            assert_eq!(entry[0].word, "John (Andrei) Vergara");
            assert!(entry[0].meanings[0].definitions[0]
                .definition
                .contains("chud streak"));
        }
        let api = dictionary.lookup("API").unwrap().unwrap();
        assert_eq!(api[0].word, "API");
        assert!(api[0].meanings[0].definitions[0]
            .definition
            .contains("software components to interact"));
        assert!(api[0].meanings[0]
            .definitions
            .iter()
            .any(|definition| definition
                .definition
                .to_lowercase()
                .contains("application programming interface")));
        assert!(api[0].meanings[0].definitions.len() > 1);
        assert_eq!(dictionary.lookup("api").unwrap().unwrap()[0].word, "API");
        let ram = dictionary.lookup("ram").unwrap().unwrap();
        assert_eq!(ram[0].word, "ram");
        let ram_acronym = dictionary.lookup("RAM").unwrap().unwrap();
        assert_eq!(ram_acronym[0].word, "RAM");
        assert!(ram_acronym[0].meanings[0].definitions[0]
            .definition
            .to_lowercase()
            .contains("random-access memory"));
        assert!(dictionary.lookup("FYI").unwrap().is_some());
        assert!(dictionary.lookup("FAQ").unwrap().is_some());
        assert!(dictionary.lookup("2FA").unwrap().is_some());
        let gpu = dictionary.lookup("GPU").unwrap().unwrap();
        assert!(gpu[0].meanings[0].definitions[0]
            .definition
            .contains("creation of images"));
        for spelling in ["LOL", "LoL", "lol"] {
            let entries = dictionary.lookup(spelling).unwrap().unwrap();
            assert_eq!(entries[0].word, "LOL");
            assert!(entries[0].meanings[0].definitions[0]
                .definition
                .starts_with("Laugh out loud:"));
            assert!(entries[0].meanings.iter().any(|meaning| meaning
                .definitions
                .iter()
                .any(|definition| definition.definition.contains("League of Legends"))));
        }
        for (term, expected) in [
            ("OMG", "Oh my God:"),
            ("POV", "Point of view:"),
            ("TLDR", "Too long; didn't read:"),
            ("IRL", "In real life:"),
            ("DM", "Direct message:"),
        ] {
            let entry = dictionary.lookup(term).unwrap().unwrap();
            assert!(entry[0].meanings[0].definitions[0]
                .definition
                .starts_with(expected));
        }
        assert!(dictionary.lookup("zzzznotawordzzzz").unwrap().is_none());
    }
}
