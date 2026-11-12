use crate::model::{Definition, Entry, License, Meaning};
use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;

pub struct LocalDictionary {
    db: Connection,
}

impl LocalDictionary {
    pub fn open(path: &Path) -> Result<Self, String> {
        let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("Cannot open local dictionary: {e}"))?;
        Ok(Self { db })
    }

    pub fn lookup(&self, word: &str) -> Result<Option<Vec<Entry>>, String> {
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
        Ok(None)
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
        let dictionary = LocalDictionary::open(&path).unwrap();
        for word in ["another", "instruction", "Blockers"] {
            let entries = dictionary.lookup(word).unwrap().unwrap();
            assert!(!entries[0].meanings[0].definitions[0].definition.is_empty());
        }
        let dio = dictionary.lookup("dio").unwrap().unwrap();
        assert_eq!(dio[0].word, "DIO");
        assert!(dio[0].meanings[0].definitions[0]
            .definition
            .contains("desktop dictionary"));
        assert!(dictionary.lookup("zzzznotawordzzzz").unwrap().is_none());
    }
}
