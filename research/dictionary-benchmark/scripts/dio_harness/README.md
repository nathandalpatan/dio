# DIO lookup harness

This standalone Rust program imports DIO's current `dictionary.rs`, `local_dictionary.rs`, and `model.rs` directly. It does not edit or launch the application. Its offline mode calls `special` and `words`; its online mode calls `special`, then the live parallel provider lookup, then the bundled fallback on provider error. The online mode starts with no persisted cache. It preserves `Entry` order, examples, relations, source URLs, and license fields in JSONL.

From the repository root:

```powershell
cargo run --offline --manifest-path research/dictionary-benchmark/scripts/dio_harness/Cargo.toml -- offline research/dictionary-benchmark/benchmark_words.csv research/dictionary-benchmark/dio_offline.jsonl 0
cargo run --offline --manifest-path research/dictionary-benchmark/scripts/dio_harness/Cargo.toml -- online research/dictionary-benchmark/benchmark_words.csv research/dictionary-benchmark/dio_online.jsonl 1000
```

The optional fourth argument caps the number of CSV data rows, for smoke tests. The online delay is in milliseconds, applied between words. Each record is flushed immediately and includes `route`, `status`, source and license metadata, elapsed lookup time, and any provider error. `offline_fallback` means an online provider failed and DIO returned bundled definitions; it must not be counted as a live online definition. `exact_headword` tests the bundled OEWN headword table, independently of the fallback route.

The program reads the checked-out production source at build time. Record the repository commit and working-tree state with any results so a later build does not silently test changed code. Its timing includes Rust lookup and SQLite or network work, not Tauri UI rendering, cache reads, or pronunciation resolution.
