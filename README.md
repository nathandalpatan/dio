# DIO

DIO is a Windows desktop dictionary. Highlight a word or short expression, then press **Ctrl+Alt+D**. DIO reads the selection through Windows UI Automation when available and otherwise tries a clipboard copy. A popup shows an English definition from its bundled local dictionary or, for words it does not contain, the [Free Dictionary API](https://dictionaryapi.dev/), with its IPA pronunciation under the headword. Use **More** for remaining definitions, pronunciation variants, examples, related words, origins, and attribution when available.

DIO can also speak with [Kokoro-82M](https://huggingface.co/hexgrad/Kokoro-82M), an open-weight voice model that runs on your computer. It pronounces the displayed word, reads definitions, and reads text you select in other applications when you press **Ctrl+Alt+R**.

## Install and use

Install the NSIS package from `src-tauri/target/release/bundle/nsis/` after building it. DIO runs in the system tray. Right-click its icon for **Settings** or **Quit DIO**.

1. Highlight one continuous selection of at most 128 characters.
2. Press **Ctrl+Alt+D**. If another application owns that global shortcut, open Settings from the tray and choose another binding.
3. Click **More**, the speaker beside the IPA, a related word, or the star to expand, play pronunciation, navigate, or save the entry. Press **Escape** or click outside to dismiss. Select another word in an app and press the shortcut again to replace the result.

### Pronunciation and speech

The IPA line shows the accent chosen in **Settings → Pronunciation** (US or UK) when a source provides it. The accent tag appears only when the source labels the accent. Hover over a transcription to see its source. Transcriptions marked **≈** were generated from Kokoro's pronunciation rules because no dictionary had the word, and are not dictionary transcriptions.

To turn on speech, open **Settings → Speech** and click **Install voice**. This downloads the model once (about 95 MB). After that:

- The speaker plays a recording in your accent when one exists. Otherwise Kokoro speaks the word with a voice in that accent.
- The play button in the popup footer reads the word and its definition. While speech is playing, the footer offers pause, resume, and stop.
- To read text in another app, select up to 2,000 characters (configurable) and press **Ctrl+Alt+R**. The popup shows what is being read, without taking focus from the app. Press the shortcut again, use the stop button, or choose **Stop reading** from the tray to stop. Selections over the limit are refused rather than cut off.

Voice, speed, volume, accent, the read-aloud shortcut, and the selection limit are in Settings. The two shortcuts must differ.

The optional middle-click trigger passes the click through to the source application, so that application may also perform its normal middle-click action. It is disabled by default. Keyboard and middle-click triggers can be enabled independently.

## Privacy and data

DIO reads highlighted text through Windows UI Automation, falling back to a simulated Ctrl+C when accessibility cannot read the selection. Before copying, it backs up supported clipboard formats and restores them if no other application changes the clipboard. If a format cannot be backed up, DIO skips the fallback. Cached and bundled definitions work offline. Only words missing from both are sent to `api.dictionaryapi.dev`. Available pronunciation audio is downloaded through the Rust backend from a restricted set of HTTPS hosts. Settings, successful online lookup cache, and saved vocabulary are stored locally in the user's application data directory. Saved words remain after cache clearing or eviction. The online cache is fresh for 30 days and holds at most 1,000 entries. If refreshing a cached entry fails, DIO displays the stale entry with an offline notice.

The bundled dictionary contains 152,459 English headwords from the Open English WordNet 2025+ rows of [CloudBytes Academy's v2 dataset](https://github.com/CloudBytes-Academy/English-Dictionary-Open-Source). DIO lowercases headwords, groups senses by part of speech, and keeps the first example for each sense. These entries have no pronunciation audio. Their attribution and license are in [DICTIONARY-LICENSE.md](src-tauri/resources/DICTIONARY-LICENSE.md). The SQLite index was generated with `python tools/build_local_dictionary.py <dictionary.jsonl.gz> src-tauri/resources/english.sqlite3` from [this pinned JSONL release](https://github.com/CloudBytes-Academy/English-Dictionary-Open-Source/blob/ef5562840cf57c7682fc63e7e35ddb616638f93b/v2/dictionary.jsonl.gz); the script checks its SHA-256 before building.

DIO also bundles 32,906 English acronym, initialism, and uppercase abbreviation senses for 16,637 terms from [Wiktionary extracts](https://kaikki.org/dictionary/). Select an uppercase term to prefer its meaning over an ordinary word with the same spelling; **More** shows additional meanings. Thousands of senses show both the expanded term and its definition from a [Wiktionary English dictionary release](https://github.com/alavanta/dictionary-releases). Common messaging meanings such as LOL, OMG, and POV are included and shown before specialist senses; other senses retain their published expansion. The index covers the published extracts rather than every acronym ever coined. Its attribution and source links are in [ACRONYM-LICENSE.md](src-tauri/resources/ACRONYM-LICENSE.md). Rebuild it with `python tools/build_acronym_dictionary.py <acronym.jsonl> <initialism.jsonl> <abbreviation.jsonl> src-tauri/resources/acronyms.sqlite3 <dict-wiktionary-en.db>`; the script checks each source against a pinned SHA-256.

Speech runs locally. Text you read aloud is held in memory only while it plays. It is never logged, saved, or sent anywhere. Read-aloud uses the same selection path as lookups, including clipboard backup and restore, password-field refusal, and the terminal restriction. The only speech-related network traffic is the model download, which starts only when you click **Install voice**. It goes to `huggingface.co` and its `*.hf.co` download hosts, from a pinned commit, and every file is checked against a SHA-256 checksum built into DIO before it is used. Models are stored under `%APPDATA%\app.dio.dictionary\kokoro\`. DIO checks them again before loading. A damaged model is deleted and Settings offers to reinstall it. **Remove voice** deletes the model.

### Pronunciation sources

DIO resolves pronunciation separately from definitions, after the definition is shown, so missing pronunciation data never blocks a definition. Sources are tried in this order:

1. IPA returned by the Free Dictionary API with the entry. It is labelled US or UK only when its paired audio file is named that way.
2. Bundled Wiktionary transcriptions tagged General American/US or Received Pronunciation/UK. Untagged ones are shown without an accent.
3. Bundled [CMUdict](https://github.com/cmusphinx/cmudict) entries converted from ARPABET to American IPA. These are used only when no US transcription was found.
4. Kokoro's Misaki lexicon and rules, marked **≈**. These are used only when nothing above matched and every part of the word is in Misaki's lexicon. Unknown words get no IPA rather than a guess.

`pronunciations.sqlite3` (15 MB) was generated with `python tools/build_pronunciation_dictionary.py <kaikki.org-dictionary-English.jsonl.gz> <cmudict.dict> src-tauri/resources/pronunciations.sqlite3`. The script checks pinned SHA-256 values for the [Kaikki English export](https://kaikki.org/dictionary/English/index.html) and CMUdict. Attribution and licenses (Wiktionary CC BY-SA 4.0, CMUdict BSD-2-Clause) are in [PRONUNCIATION-LICENSE.md](src-tauri/resources/PRONUNCIATION-LICENSE.md). Lookups are cached in memory for the 256 most recent words.

The Free Dictionary API provides definitions and may provide source, entry license, audio source, and audio license metadata. DIO displays those fields when present. The API server's software license does **not** establish rights to every definition or audio file; check the entry's own attribution and license before reusing content.

## Speech engine

| Component | Responsibility |
|---|---|
| `src-tauri/src/pronunciation.rs` | Source priority, de-duplication, accent labels, ARPABET-to-IPA and Misaki-to-IPA conversion, lookup cache |
| `src-tauri/src/tts/mod.rs` | Speech state machine (`notInstalled`, `downloading`, `loading`, `ready`, `synthesizing`, `playing`, `paused`, `error`), single synthesis worker, cancellation, pause, resume, stop |
| `src-tauri/src/tts/g2p.rs` | Misaki text-to-phoneme conversion. Unknown words fall back to CMUdict, then to spelled-out letters |
| `src-tauri/src/tts/kokoro.rs` | ONNX Runtime session, Kokoro tokenizer, voice styles, audio output through rodio (WASAPI) |
| `src-tauri/src/tts/model_store.rs` | Pinned download, SHA-256 verification, damage detection, removal |

DIO runs Kokoro in-process with Rust and ONNX Runtime rather than as a Python sidecar like [Kokoro-Clipboard-TTS](https://github.com/seanbud/Kokoro-Clipboard-TTS). A sidecar would bundle a PyInstaller build of PyTorch (gigabytes), take seconds to start, and need a loopback server, health checks, and orphan-process cleanup. The Rust path adds about 17 MB of ONNX Runtime and starts in under a second, with no extra process or IPC surface. [pguso/kokoro](https://github.com/pguso/kokoro) was evaluated as a library. It was not used for two reasons: it always compiles a C file derived from eSpeak NG (GPL-3.0), and it enables CUDA, DirectML, and CoreML providers unconditionally. DIO instead uses its approach directly, with these dependencies:

- [`ort`](https://github.com/pykeio/ort) 2.0.0-rc.15 with `load-dynamic`, MIT/Apache-2.0. It loads the bundled `onnxruntime.dll`, so DIO still starts if the DLL is missing and only speech is unavailable. Pyke's static Windows binaries were rejected because they need a newer MSVC STL and link DirectML.dll at load time.
- Microsoft ONNX Runtime 1.31.1, CPU only, MIT, from the official NuGet package via `tools/fetch_onnxruntime.py` (pinned SHA-256). Telemetry is disabled.
- [`misaki-rs`](https://github.com/MicheleYin/misaki-rs) 0.10, MIT, a port of Kokoro's own G2P with embedded US and UK lexicons, built without eSpeak NG. DIO excludes eSpeak NG (GPL-3.0) so that it stays permissively licensed.
- [`rodio`](https://github.com/RustAudio/rodio) 0.23, MIT/Apache-2.0, playback only, and `sha2`, MIT/Apache-2.0.
- Kokoro-82M v1.0, the 8-bit ONNX export `model_quantized.onnx`, Apache-2.0, plus six voices: Heart, Bella, Michael, Fenrir (US) and Emma, George (UK). These are downloaded and not bundled. See [SPEECH-LICENSE.md](src-tauri/resources/SPEECH-LICENSE.md).

Synthesis runs on a dedicated thread, never on the Tauri main thread, and the model is loaded on first use and reused. A new request supersedes the previous one: it increments a generation counter so stale audio is dropped, and the popup's own speech stops when a new lookup starts or the popup is closed. At most two sentences are synthesized ahead of playback. The audio device is released when playback ends. The model is unloaded after ten idle minutes or when speech is turned off. On a 2026 desktop CPU without a GPU, the model loads in about 0.7 s and synthesizes about four times faster than real time.

## Build from source

Windows development requires the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/): the MSVC Rust toolchain, Microsoft C++ Build Tools with Desktop development with C++, and Microsoft Edge WebView2. Install Node.js and pnpm as well. `src-tauri/resources/onnxruntime.dll` is checked in. To refresh it, run `python tools/fetch_onnxruntime.py src-tauri/resources`.

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
cargo test --release -- --ignored   # downloads the real model and synthesizes on the CPU
cd ..
pnpm tauri build
```

### Manual QA

These need a real desktop and speakers:

1. Look up a bundled word offline (for example *tomato*). The US IPA appears under the headword. Switch **Accent** to UK and the UK transcription is shown first. **More** lists both with their sources.
2. Look up a word only the API knows. Its IPA and recording play. A nonsense word still shows its definition or "not found" with no IPA.
3. In **Settings → Speech**, install with the network off (an error appears and Install is offered again). Then install with the network on. Cancel a download midway and confirm nothing partial is left in `%APPDATA%\app.dio.dictionary\kokoro`.
4. Corrupt `model_quantized.onnx`, then click the speaker. DIO reports the damage, deletes the model, and offers to reinstall.
5. Speak a word and a definition. Pause, resume, and stop. Look up another word while speaking, and the old speech stops.
6. Select about 1,500 characters in Notepad, Edge, and Word and press **Ctrl+Alt+R**. Focus stays in the app, and the clipboard holds what it held before. Press the shortcut again to stop. Try a selection over the limit, a password field, and a terminal window.
7. Confirm **Ctrl+Alt+D** still looks up words, and that setting both shortcuts to the same keys is refused.
8. Unplug or disable the audio device during playback. An error is shown, and speech works again on the next request.
9. Quit DIO from the tray and confirm in Task Manager that no `dio.exe` process remains.

## Support and limits

This release targets Windows and English dictionary entries. Clipboard fallback requires an application that handles Ctrl+C and clipboard formats DIO can safely preserve. Terminals receive Ctrl+Insert instead, because Ctrl+C would interrupt the running program; this works in Windows Terminal, while the classic console window is refused. Password fields, empty selections, multiple disjoint selections, and selections longer than 128 characters (or the read-aloud limit) are rejected. Some applications restrict accessibility or simulated input, or do not expose selection bounds; DIO uses the pointer as a positioning fallback. Global shortcut registration detects conflicts reported by Windows, not every application-local binding.

Speech covers English only. Without eSpeak NG, words outside Misaki's lexicon and CMUdict are spelled out letter by letter, and get no IPA. Sentence splitting is simple, so abbreviations such as "Dr." can add a pause. The first word after startup waits about a second while the model loads. Accent labels follow the source data, so many words show only one accent or an unlabelled transcription. Escape and outside clicks hide the read-aloud popup but do not stop the audio.

The API response and build checks have been verified. Interactive selection, focus preservation, middle-click pass-through, Escape, outside-click dismissal, audio playback, and mixed-DPI placement still need on-device testing. Linux integration is deferred.
