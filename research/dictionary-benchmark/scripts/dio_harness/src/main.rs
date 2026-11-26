#[path = "../../../../../src-tauri/src/dictionary.rs"]
mod dictionary;
#[path = "../../../../../src-tauri/src/local_dictionary.rs"]
mod local_dictionary;
#[path = "../../../../../src-tauri/src/model.rs"]
mod model;

// model::Settings references this function, but the lookup harness never uses speech settings.
mod tts {
    pub fn voice(_name: &str) -> Option<()> {
        Some(())
    }
}

use dictionary::Dictionary;
use local_dictionary::LocalDictionary;
use model::{normalize, Entry};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn csv_rows(input: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = input.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                if row.iter().any(|part| !part.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            '\r' if !quoted && chars.peek() == Some(&'\n') => {}
            _ => field.push(ch),
        }
    }
    if quoted {
        return Err("Unclosed quoted CSV field".into());
    }
    if !row.is_empty() || !field.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

fn headword_candidate(db: &Connection, word: &str) -> Result<Option<String>, String> {
    let lower = word.to_lowercase();
    let mut candidates = vec![lower.clone()];
    if let Some(stem) = lower.strip_suffix("ies") {
        candidates.push(format!("{stem}y"));
    }
    if let Some(stem) = lower
        .strip_suffix('s')
        .filter(|stem| stem.chars().count() >= 3)
    {
        candidates.push(stem.into());
    }
    if let Some(stem) = lower.strip_suffix("es") {
        candidates.push(stem.into());
    }
    for candidate in candidates {
        let found = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM definitions WHERE word=?1)",
                [&candidate],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|error| error.to_string())?;
        if found {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn source_label(entry: &Entry, route: &str) -> &'static str {
    if route == "special" || route == "offline" || route == "offline_fallback" {
        if entry
            .source_urls
            .iter()
            .any(|url| url.contains("wiktionary.org"))
        {
            "bundled_acronyms"
        } else {
            "bundled_oewn_or_special"
        }
    } else if entry
        .source_urls
        .iter()
        .any(|url| url.contains("simple.wiktionary.org"))
    {
        "simple_wiktionary"
    } else {
        "free_dictionary_api"
    }
}

fn record(
    item_id: &str,
    term: &str,
    configuration: &str,
    route: &str,
    entries: Vec<Entry>,
    error_kind: Option<&str>,
    error_message: Option<&str>,
    elapsed_ms: f64,
    matched_headword: Option<&str>,
    exact_headword: bool,
) -> Value {
    let source_urls: Vec<_> = entries
        .iter()
        .flat_map(|entry| entry.source_urls.iter().cloned())
        .collect();
    let source_order: Vec<_> = entries
        .iter()
        .map(|entry| source_label(entry, route))
        .collect();
    let status = if entries.is_empty() {
        if error_kind.is_some() {
            "error"
        } else {
            "missing"
        }
    } else {
        "found"
    };
    json!({
        "item_id": item_id,
        "term": term,
        "configuration": configuration,
        "status": status,
        "route": route,
        "entries": entries,
        "source_urls": source_urls,
        "source_order": source_order,
        "error_kind": error_kind,
        "error_message": error_message,
        "elapsed_ms": elapsed_ms,
        "matched_headword": matched_headword,
        "exact_headword": exact_headword,
        "retrieved_at_unix": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
    })
}

fn write_record(writer: &mut BufWriter<File>, value: Value) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, &value).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4
        || args.len() > 6
        || !["offline", "online", "both"].contains(&args[1].as_str())
    {
        return Err("Usage: dio-dictionary-harness <offline|online|both> <benchmark_words.csv> <output.jsonl> [delay_ms=1000] [max_rows]".into());
    }
    let mode = args[1].as_str();
    let delay_ms: u64 = args.get(4).map(|v| v.parse()).transpose()?.unwrap_or(1000);
    let max_rows: usize = args
        .get(5)
        .map(|v| v.parse())
        .transpose()?
        .unwrap_or(usize::MAX);
    let source = fs::read_to_string(&args[2])?;
    let mut rows = csv_rows(&source)?;
    if rows.is_empty() {
        return Err("Empty CSV".into());
    }
    let headers = rows.remove(0);
    let id_index = headers
        .iter()
        .position(|field| field == "id")
        .ok_or("Missing id column")?;
    let term_index = headers
        .iter()
        .position(|field| field == "term" || field == "word")
        .ok_or("Missing term column")?;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .ok_or("Cannot locate DIO root")?;
    let dictionary_path = root.join("src-tauri/resources/english.sqlite3");
    let acronym_path = root.join("src-tauri/resources/acronyms.sqlite3");
    let local = LocalDictionary::open(&dictionary_path, &acronym_path)?;
    let headwords = Connection::open_with_flags(dictionary_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let online = if mode == "offline" {
        None
    } else {
        Some(Dictionary::new()?)
    };
    let mut writer = BufWriter::new(File::create(&args[3])?);

    for (index, row) in rows.into_iter().take(max_rows).enumerate() {
        let item_id = row.get(id_index).ok_or("CSV row lacks id")?;
        let term = row.get(term_index).ok_or("CSV row lacks term")?;
        let normalized = match normalize(term) {
            Ok(value) => value,
            Err(message) => {
                for configuration in match mode {
                    "both" => vec!["dio_offline", "dio_online"],
                    "offline" => vec!["dio_offline"],
                    _ => vec!["dio_online"],
                } {
                    write_record(
                        &mut writer,
                        record(
                            item_id,
                            term,
                            configuration,
                            "invalid_selection",
                            vec![],
                            Some("invalidSelection"),
                            Some(&message),
                            0.0,
                            None,
                            false,
                        ),
                    )?;
                }
                continue;
            }
        };
        let matched = headword_candidate(&headwords, &normalized)?;
        let exact = matched.as_deref() == Some(normalized.to_lowercase().as_str());
        if mode == "offline" || mode == "both" {
            let started = Instant::now();
            let special = local.special(&normalized);
            let (route, result) = match special {
                Ok(Some(entries)) => ("special", Ok(Some(entries))),
                Ok(None) => ("offline", local.words(&normalized)),
                Err(error) => ("special", Err(error)),
            };
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            let (entries, error) = match result {
                Ok(Some(entries)) => (entries, None),
                Ok(None) => (vec![], None),
                Err(message) => (vec![], Some(message)),
            };
            write_record(
                &mut writer,
                record(
                    item_id,
                    term,
                    "dio_offline",
                    route,
                    entries,
                    error.as_ref().map(|_| "storage"),
                    error.as_deref(),
                    elapsed,
                    matched.as_deref(),
                    exact,
                ),
            )?;
        }
        if let Some(online) = &online {
            if index > 0 && delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            let started = Instant::now();
            let (route, entries, error_kind, error_message) = match local.special(&normalized) {
                Ok(Some(entries)) => ("special", entries, None, None),
                Err(message) => ("special", vec![], Some("storage"), Some(message)),
                Ok(None) => match online.lookup(&normalized).await {
                    Ok(entries) => ("online", entries, None, None),
                    Err(error) => match local.words(&normalized) {
                        Ok(Some(entries)) => (
                            "offline_fallback",
                            entries,
                            Some(error.kind),
                            Some(error.message),
                        ),
                        Ok(None) => (
                            "online_error",
                            vec![],
                            Some(error.kind),
                            Some(error.message),
                        ),
                        Err(local_error) => {
                            ("offline_error", vec![], Some("storage"), Some(local_error))
                        }
                    },
                },
            };
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            write_record(
                &mut writer,
                record(
                    item_id,
                    term,
                    "dio_online",
                    route,
                    entries,
                    error_kind,
                    error_message.as_deref(),
                    elapsed,
                    matched.as_deref(),
                    exact,
                ),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod harness_tests {
    use super::csv_rows;

    #[test]
    fn reads_quoted_csv_contexts() {
        let rows = csv_rows("id,term,context\r\n1,bank,\"She went to the bank, then left.\"\r\n2,\"say \"\"hello\"\"\",plain\r\n").unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1][2], "She went to the bank, then left.");
        assert_eq!(rows[2][1], "say \"hello\"");
    }
}
