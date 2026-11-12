# Acronym dictionary attribution

`acronyms.sqlite3` contains 32,906 English acronym, initialism, and uppercase
abbreviation senses for 16,637 distinct terms. DIO extracted their plain-text glosses from the
[acronym](https://kaikki.org/dictionary/English/tags/om/acronym/index.html) and
[initialism](https://kaikki.org/dictionary/English/tags/nY/initialism/index.html) and
[abbreviation](https://kaikki.org/dictionary/English/tags/Tx/abbreviation/index.html)
JSONL exports of English Wiktionary, published by Tatu Ylonen's
[Wiktextract/Kaikki](https://kaikki.org/dictionary/) project. The exports were
based on the English Wiktionary dump dated 2026-09-02.

For thousands of senses, DIO adds the expanded term's definition from the
[`dict-wiktionary-en.db` release](https://github.com/alavanta/dictionary-releases)
built from Wiktionary through Wiktextract. The compressed release has SHA-256
`52aedb7edb1cef76433092497ed3c5d4b460fa7bd2948a4348bb065952be4165`;
the decompressed database has SHA-256
`28b00a03353acb833f4ece65aa62559e517d4df4c1ea36636cb7c9a768948a64`.

Copyright Wiktionary contributors. The extracted text and this modified subset
are available under [Creative Commons Attribution-ShareAlike 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
Each DIO entry links to its Wiktionary source page. DIO filtered the extracts to
English acronym and initialism senses plus uppercase abbreviations with at least
two letters, removed duplicate glosses, matched expanded terms to their first
English definition where available, and indexed
the remaining terms for offline lookup. A small set of common messaging senses
was added or clarified from their Wiktionary entries and placed before less
common meanings. The source exports are not included in
the application bundle; `tools/build_acronym_dictionary.py` records their
checksums and rebuilds the index.
