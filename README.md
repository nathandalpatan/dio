# DIO

<<<<<<< HEAD
DIO is a Windows desktop dictionary. Highlight a word or short expression, then press **Ctrl+Alt+D**. DIO reads the selection through Windows UI Automation when available and otherwise tries a clipboard copy. A popup shows an English definition from its bundled local dictionary or, for words it does not contain, the [Free Dictionary API](https://dictionaryapi.dev/), with its IPA pronunciation under the headword. Use **More** for remaining definitions, pronunciation variants, examples, related words, origins, and attribution when available.

DIO can also speak with [Kokoro-82M](https://huggingface.co/hexgrad/Kokoro-82M), an open-weight voice model that runs on your computer. It pronounces the displayed word, reads definitions, and reads text you select in other applications when you press **Ctrl+Alt+R**.
=======
A lightweight, system-wide dictionary for Windows. Highlight a word, press **Ctrl + Alt + D**, and get a definition without leaving your application.
>>>>>>> origin/main

Built with Rust and Tauri, DIO combines offline dictionary resources with online lookups in a fast, minimal, privacy-conscious desktop experience.

## Features

<<<<<<< HEAD
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
=======
- **Instant lookups** — Look up selected text across supported Windows applications.
- **Offline dictionary** — Over 152,000 English headwords, available without an internet connection.
- **Acronym support** — Over 32,000 meanings across 16,000 acronyms, initialisms, and abbreviations.
- **Rich definitions** — Definitions, pronunciations, examples, synonyms, and word origins when available.
- **Saved vocabulary** — Bookmark words for future reference.
- **Customisable controls** — Configure keyboard shortcuts and optional middle-click activation.
- **Local-first privacy** — Offline lookups, clipboard preservation, and local data storage.
>>>>>>> origin/main

## Getting Started

1. Install DIO using the generated Windows installer.
2. Highlight a word or short expression in any supported application.
3. Press **Ctrl + Alt + D** to open the dictionary popup.
4. Select **More** to explore additional definitions, or **Listen** to hear pronunciation when available.

Press **Escape** or click outside to dismiss the popup.

DIO runs in the system tray. Right-click its icon to access Settings or quit the application.

## How It Works

<<<<<<< HEAD
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
=======
DIO retrieves selected text using Windows UI Automation, with a clipboard fallback when necessary.

Lookups prioritise bundled SQLite dictionaries and cached entries before querying the [Free Dictionary API](https://dictionaryapi.dev/).

Settings, cached definitions, and saved vocabulary are stored locally. Clipboard contents are preserved whenever supported, and network requests are limited to online lookups and pronunciation audio.
>>>>>>> origin/main

## Tech Stack

- **Rust & Tauri** — Desktop application and native Windows integration.
- **SQLite** — Embedded offline dictionaries.
- **Windows UI Automation** — System-wide text selection.
- **Free Dictionary API** — Online definitions and pronunciation.

## Development

### Prerequisites

Rust (MSVC), Node.js, pnpm, Microsoft C++ Build Tools, and WebView2.

See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for setup instructions.

### Run Locally

```bash
git clone https://github.com/nathandalpatan/dio.git
cd dio
pnpm install
pnpm tauri dev
```

### Build

```bash
pnpm tauri build
```

The Windows installer is generated in:

`src-tauri/target/release/bundle/nsis/`

### Tests

```bash
pnpm test
pnpm build

cd src-tauri
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
<<<<<<< HEAD
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
=======
```

## Platform Support

| Platform | Status |
|---|---|
| Windows | Supported |
| Linux | Planned |
>>>>>>> origin/main

DIO currently supports English dictionary entries and continuous text selections of up to 128 characters.

Build checks and API responses have been verified. Native interactions, audio playback, and mixed-DPI positioning require further on-device testing.

## Attribution

DIO uses dictionary resources from Open English WordNet, CloudBytes Academy, Wiktionary, and the Free Dictionary API.

- [English Dictionary Licence](src-tauri/resources/DICTIONARY-LICENSE.md)
- [Acronym Dictionary Licence](src-tauri/resources/ACRONYM-LICENSE.md)

Third-party content remains subject to its respective licensing and attribution requirements.
