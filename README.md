# DIO

DIO is a Windows desktop dictionary. Highlight a word or short expression, then press **Ctrl+Alt+D**. DIO reads the selection through Windows UI Automation when available and otherwise tries a clipboard copy. A popup shows an English definition from its bundled local dictionary or, for words it does not contain, the [Free Dictionary API](https://dictionaryapi.dev/). Use **More** for remaining definitions, examples, related words, origins, and attribution when available.

## Install and use

Install the NSIS package from `src-tauri/target/release/bundle/nsis/` after building it. DIO runs in the system tray. Right-click its icon for **Settings** or **Quit DIO**.

1. Highlight one continuous selection of at most 128 characters.
2. Press **Ctrl+Alt+D**. If another application owns that global shortcut, open Settings from the tray and choose another binding.
3. Click **More**, **Listen**, a related word, or the star to expand, play pronunciation, navigate, or save the entry. Press **Escape** or click outside to dismiss. Select another word in an app and press the shortcut again to replace the result.

The optional middle-click trigger passes the click through to the source application, so that application may also perform its normal middle-click action. It is disabled by default. Keyboard and middle-click triggers can be enabled independently.

## Privacy and data

DIO reads highlighted text through Windows UI Automation, falling back to a simulated Ctrl+C when accessibility cannot read the selection. Before copying, it backs up supported clipboard formats and restores them if no other application changes the clipboard. If a format cannot be backed up, DIO skips the fallback. Cached and bundled definitions work offline. Only words missing from both are sent to `api.dictionaryapi.dev`. Available pronunciation audio is downloaded through the Rust backend from a restricted set of HTTPS hosts. Settings, successful online lookup cache, and saved vocabulary are stored locally in the user's application data directory. Saved words remain after cache clearing or eviction. The online cache is fresh for 30 days and holds at most 1,000 entries. If refreshing a cached entry fails, DIO displays the stale entry with an offline notice.

The bundled dictionary contains 152,459 English headwords from the Open English WordNet 2025+ rows of [CloudBytes Academy's v2 dataset](https://github.com/CloudBytes-Academy/English-Dictionary-Open-Source). DIO lowercases headwords, groups senses by part of speech, and keeps the first example for each sense. These entries have no pronunciation audio. Their attribution and license are in [DICTIONARY-LICENSE.md](src-tauri/resources/DICTIONARY-LICENSE.md). The SQLite index was generated with `python tools/build_local_dictionary.py <dictionary.jsonl.gz> src-tauri/resources/english.sqlite3` from [this pinned JSONL release](https://github.com/CloudBytes-Academy/English-Dictionary-Open-Source/blob/ef5562840cf57c7682fc63e7e35ddb616638f93b/v2/dictionary.jsonl.gz); the script checks its SHA-256 before building.

DIO also bundles 32,906 English acronym, initialism, and uppercase abbreviation senses for 16,637 terms from [Wiktionary extracts](https://kaikki.org/dictionary/). Select an uppercase term to prefer its meaning over an ordinary word with the same spelling; **More** shows additional meanings. Thousands of senses show both the expanded term and its definition from a [Wiktionary English dictionary release](https://github.com/alavanta/dictionary-releases). Common messaging meanings such as LOL, OMG, and POV are included and shown before specialist senses; other senses retain their published expansion. The index covers the published extracts rather than every acronym ever coined. Its attribution and source links are in [ACRONYM-LICENSE.md](src-tauri/resources/ACRONYM-LICENSE.md). Rebuild it with `python tools/build_acronym_dictionary.py <acronym.jsonl> <initialism.jsonl> <abbreviation.jsonl> src-tauri/resources/acronyms.sqlite3 <dict-wiktionary-en.db>`; the script checks each source against a pinned SHA-256.

The Free Dictionary API provides definitions and may provide source, entry license, audio source, and audio license metadata. DIO displays those fields when present. The API server's software license does **not** establish rights to every definition or audio file; check the entry's own attribution and license before reusing content.

## Build from source

Windows development requires the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/): the MSVC Rust toolchain, Microsoft C++ Build Tools with Desktop development with C++, and Microsoft Edge WebView2. Install Node.js and pnpm as well.

```powershell
pnpm install
pnpm tauri dev
```

To run checks and create the installer:

```powershell
pnpm test
pnpm build
cd src-tauri
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cd ..
pnpm tauri build
```

## Support and limits

This release targets Windows and English dictionary entries. Clipboard fallback requires an application that handles Ctrl+C and clipboard formats DIO can safely preserve; it is disabled for detected terminal windows. Password fields, empty selections, multiple disjoint selections, and selections longer than 128 characters are rejected. Some applications restrict accessibility or simulated input, or do not expose selection bounds; DIO uses the pointer as a positioning fallback. Global shortcut registration detects conflicts reported by Windows, not every application-local binding.

The API response and build checks have been verified. Interactive selection, focus preservation, middle-click pass-through, Escape, outside-click dismissal, audio playback, and mixed-DPI placement still need on-device testing. Linux integration is deferred.
