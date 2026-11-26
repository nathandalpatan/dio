# Existing DIO dictionary audit

## How a selected word becomes a popup definition

### Takeaway

DIO validates the selected text, resolves special bundled entries and a 30-day online cache first, then asks Simple English Wiktionary and Free Dictionary API in parallel. A stale cache or bundled dictionary supplies the fallback. The popup displays the first returned definition as its primary meaning.

The audited checkout was commit `f719e285f2b2abecbd41100a78af2ec3a7a6c6e6` with pre-existing uncommitted popup and UI changes. The research harness imports the current source modules from this working tree.

### Cited findings

- The Windows selection reader tries UI Automation and falls back to clipboard capture; the activation path trims selected punctuation, and `normalize` rejects empty, multiline, control-character, or over-128-character selections. [native.rs](../../src-tauri/src/native.rs), [main.rs](../../src-tauri/src/main.rs#L380), [model.rs](../../src-tauri/src/model.rs#L196)
- `resolve` checks special local entries before cache, then a fresh cache, then `Dictionary::lookup`; a failed online lookup uses a stale cache or `LocalDictionary::words`. Successful fresh online results are cached. [main.rs](../../src-tauri/src/main.rs#L106), [storage.rs](../../src-tauri/src/storage.rs#L9)
- Special entries include named people, `DIO`, and all-caps terms found in the bundled acronym table. They bypass online providers and cache. [local_dictionary.rs](../../src-tauri/src/local_dictionary.rs#L20), [main.rs](../../src-tauri/src/main.rs#L106)
- The online module fetches Simple English Wiktionary wikitext and Free Dictionary API concurrently. A parsed Simple entry precedes all API entries; when Simple fails, the API alone determines success or error. The Simple parser recognizes part-of-speech headings, top-level definitions, examples, and limited `syn`/`ant` templates. [dictionary.rs](../../src-tauri/src/dictionary.rs#L27), [dictionary.rs](../../src-tauri/src/dictionary.rs#L200)
- Online requests use a 5-second connection timeout and 10-second overall timeout, reject redirects, and cap each response body at 2 MB. The Free Dictionary API path retries 502, 503, and 504 responses twice after short delays; Simple Wiktionary errors resolve as absent entries. [dictionary.rs](../../src-tauri/src/dictionary.rs#L14), [dictionary.rs](../../src-tauri/src/dictionary.rs#L43), [dictionary.rs](../../src-tauri/src/dictionary.rs#L87)
- Cache freshness is 30 days and eviction keeps at most 1,000 recently used entries. Saved vocabulary is separate. [storage.rs](../../src-tauri/src/storage.rs#L9), [storage.rs](../../src-tauri/src/storage.rs#L64)
- The compact popup selects the first entry and its first nonempty part of speech, then shows that meaning's first definition. Expanded mode displays later senses in the returned order, initially five per part of speech. Source URLs and license names appear in expanded attribution. [main.tsx](../../src/main.tsx#L118), [main.tsx](../../src/main.tsx#L167)
- In a live `date` lookup, the Simple entry carried a `simple.wiktionary.org` URL and CC BY-SA 4.0 attribution, followed by Free Dictionary API entries pointing to `en.wiktionary.org` with CC BY-SA 3.0 metadata. The parser retains those fields in `Entry` values; the popup renders whatever metadata providers supply. [dio_online.jsonl](dio_online.jsonl), [dictionary.rs](../../src-tauri/src/dictionary.rs#L70), [main.tsx](../../src/main.tsx#L167)

### Inferences

- First-definition accuracy depends jointly on source sense ordering, DIO's local part-of-speech ordering, and the popup's selection of the first entry. A source can contain the intended sense yet fail to show it first.
- A cache-empty benchmark is needed to observe the live online pipeline; a normal DIO session may return a cached response without contacting either provider.

### Gaps

- UI Automation and clipboard capture were inspected in code, not exercised across third-party Windows applications in this benchmark.
- The benchmark harness reuses the parser and lookup modules but does not invoke Tauri window rendering, pronunciation loading, saved vocabulary, or a persisted cache.

## What the bundled data contains

### Takeaway

The bundled SQLite database is a pinned OEWN-only projection of a CloudBytes v2 export. It has broad traditional headword coverage but limited modern, technical, idiomatic, and irregular-form coverage.

### Cited findings

- `build_local_dictionary.py` verifies a fixed SHA-256 on the compressed source, imports only rows with `source == "oewn"` and nonblank definitions, lowercases headwords, keeps the first example and synset ID, and builds word and synset indexes. [build_local_dictionary.py](../../tools/build_local_dictionary.py)
- The SQLite schema contains no frequency rank, source-version metadata, per-row license, inflection table, or ordered sense ID. The runtime supplies one hard-coded OEWN attribution URL for bundled definitions. [english.sqlite3](../../src-tauri/resources/english.sqlite3), [local_dictionary.rs](../../src-tauri/src/local_dictionary.rs#L225)
- Read-only SQLite queries on [english.sqlite3](../../src-tauri/resources/english.sqlite3) found 34,484,224 bytes, 212,659 definition rows, 152,459 distinct headwords, 120,564 nonempty synset IDs, and 61,598 rows with a nonempty example. Schema: `definitions(word TEXT, pos TEXT, definition TEXT, example TEXT, synset_id TEXT)`, indexed by `word` and `synset_id`.
- The bundled data notice identifies OEWN 2025+ with CC BY 4.0 and the Princeton WordNet notice. It also describes the CloudBytes conversion from source senses. [DICTIONARY-LICENSE.md](../../src-tauri/resources/DICTIONARY-LICENSE.md)
- Exact database probes found no rows for `nuanced`, `gaslighting`, `doomscrolling`, `rizz`, `delulu`, `mutex`, `break the ice`, `ran`, `went`, `children`, `brought`, or `jandals`. `idempotent` has one mathematical meaning; `polymorphism` has genetic, chemical, and biological meanings but no computing meaning. The database itself contains these omissions. [english.sqlite3](../../src-tauri/resources/english.sqlite3)
- The 500-item corpus produced 388 nonempty and 112 missing offline responses, with 377 exact OEWN headword hits. The lowest nonempty counts were idioms 19/50, modern slang 26/50, irregular forms 29/50, and regional vocabulary 31/50. Four lowercase words (`arvo`, `ran`, `met`, `wore`) returned unrelated uppercase acronyms, so nonempty-response coverage overstates relevant-sense coverage. These are corpus-conditional, not general population coverage estimates. [benchmark_words.csv](benchmark_words.csv), [dio_offline.jsonl](dio_offline.jsonl)

### Inferences

- Missing modern and technical meanings cannot be fixed by changing popup order alone. Additional licensed data or editorial curation would be needed.
- The CloudBytes export's source order is not evidence of common-meaning priority; the importer writes rows in source order without a frequency field.

### Gaps

- The original compressed CloudBytes export was not present locally, so this audit verified the build code and bundled result, not a fresh rebuild against the upstream archive.
- These counts do not establish the correctness or frequency of every imported OEWN sense.

## Processing, ordering, morphology, and presentation failures

### Takeaway

Local lookup has no frequency-ranked sense ordering and handles only simple plural suffixes. Some wrong displays arise from processing, while others are missing or damaged source data.

### Cited findings

- Local lookup queries `definitions WHERE word=?1` without `ORDER BY`, groups senses by part of speech, and sorts parts of speech by descending sense count. It derives synonyms from matching synset IDs. Its only candidate transformations are `ies → y`, final `s` removal, and `es` removal. [local_dictionary.rs](../../src-tauri/src/local_dictionary.rs#L83), [local_dictionary.rs](../../src-tauri/src/local_dictionary.rs#L166)
- The first displayed offline `bank` definition is the riverbank sense although the financial sense exists second. This is an ordering failure in the returned entries, not missing sense coverage. [dio_offline.jsonl](dio_offline.jsonl)
- Lowercase `arvo`, `ran`, `met`, and `wore` miss all OEWN candidate forms, then `LocalDictionary::words` unconditionally searches the acronym table and returns unrelated uppercase acronym entries. `ran` becomes `RAN` (Royal Australian Navy / radio access network). This is an incorrect fallback caused by lookup processing. [local_dictionary.rs](../../src-tauri/src/local_dictionary.rs#L83), [dio_offline.jsonl](dio_offline.jsonl)
- The four nonexact OEWN hits in this corpus are `sunnies→sunny`, `cheers→cheer`, `crisps→crisp`, and `trainers→trainer`. Their contextual sentences intend sunglasses, thanks, snack food, and footwear, while the returned first definitions describe sunny weather, encouragement, clarity, and a person who trains. `crisps→crisp` does have a later potato-slice sense, so its first-definition accuracy fails while relevant-sense coverage may succeed. These are stemmer and ordering mismatches, not exact-headword coverage. [benchmark_words.csv](benchmark_words.csv), [dio_offline.jsonl](dio_offline.jsonl)
- `went`, `children`, and `brought` remain missing offline because neither exact rows nor irregular-lemma rules exist. `better` has exact rows, but its glosses contain U+FFFD replacement characters around `good` and `well`; that is damaged bundled data, not popup rendering. [english.sqlite3](../../src-tauri/resources/english.sqlite3), [dio_offline.jsonl](dio_offline.jsonl)
- The raw database's first `run` row is a baseball score, but DIO's part-of-speech count sort displays the common verb sense first. This shows why raw row order and actual popup order must be evaluated separately. [english.sqlite3](../../src-tauri/resources/english.sqlite3), [dio_offline.jsonl](dio_offline.jsonl)
- The current popup implementation has pre-existing uncommitted changes, including navigation and display behavior. This research did not modify production files. [main.tsx](../../src/main.tsx), [popup.rs](../../src-tauri/src/popup.rs)

### Inferences

- For polysemous words, even a correct source definition can be buried by the absence of common-sense ranking. A score for relevant-sense coverage must therefore be separate from first-definition accuracy.
- A fix to irregular inflections should also guard against unrelated acronym fallthrough for lowercase words.

### Gaps

- Whether any particular ordering is globally most common requires independent usage-frequency evidence; the cited examples establish contextual mismatches only.
- Online coverage and ordering will be assessed from the live harness output, not inferred from provider documentation.
