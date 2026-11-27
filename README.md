# DIO

A lightweight, system-wide dictionary for Windows. Highlight a word, press **Ctrl + Alt + D**, and get a definition and its pronunciation without leaving your application.

Built with Rust and Tauri, DIO combines offline dictionary resources with online lookups in a fast, minimal, privacy-conscious desktop experience. It can also speak with [Kokoro-82M](https://huggingface.co/hexgrad/Kokoro-82M), an open-weight voice model that runs entirely on your computer.

## Features

- **Instant lookups**: look up selected text across supported Windows applications, including Windows Terminal.
- **Offline dictionary**: over 152,000 English headwords, available without an internet connection.
- **Acronym support**: over 32,000 meanings across 16,000 acronyms, initialisms, and abbreviations.
- **IPA pronunciation**: US and UK transcriptions under the headword, labelled with their source.
- **Local speech**: hear words and definitions read by Kokoro, with pause, resume, stop, voice, speed, and volume controls.
- **Read aloud**: select text in any app and press **Ctrl + Alt + R** to hear it.
- **Rich definitions**: definitions, examples, synonyms, and word origins when available.
- **Saved vocabulary**: bookmark words and search your saved list for future reference.
- **Customisable controls**: configure both shortcuts and optional middle-click activation.
- **Updates**: install new versions automatically, be notified, or check yourself. Every update is signature-checked, and settings, saved words, and the voice model are kept.
- **Local-first privacy**: offline fallback dictionary, local speech, clipboard preservation, and local data storage.

## Getting Started

1. Download `DIO-setup.exe` from the [website](https://nathandalpatan.github.io/dio/) or the [latest release](https://github.com/nathandalpatan/dio/releases/latest) and run it. The installer is not code-signed yet, so if Windows SmartScreen warns you, click **More info**, then **Run anyway**.
2. Highlight a word or short expression in any supported application.
3. Press **Ctrl + Alt + D** to open the dictionary popup.
4. Select **Expand** for additional definitions and pronunciation variants. Click the speaker beside the IPA to hear the word.

Press **Escape** or click outside to dismiss the popup.
Use the popup's **Back** and **Forward** buttons to revisit words opened from a lookup. Looking up a different word after going back replaces the forward history.

DIO runs in the system tray. Right-click its icon to open Settings, stop reading, or quit the application.

### Speech

Open **Settings → Speech** and click **Install voice**. This downloads the model once (about 95 MB). After that:

- The speaker plays a recording in your preferred accent when one exists. Otherwise Kokoro speaks the word in a matching voice.
- The play button in the popup footer reads the word and its definition.
- To read text in another app, select up to 2,000 characters (configurable) and press **Ctrl + Alt + R**. The popup shows what is being read without taking focus from the app. Press the shortcut again to stop. Longer selections are refused rather than cut off.

Accent, voice, speed, volume, the read-aloud shortcut, and the selection limit are all in Settings. The two shortcuts must differ.

## How It Works

DIO retrieves selected text using Windows UI Automation. When that fails, it falls back to a clipboard copy: it sends **Ctrl + C**, or **Ctrl + Insert** in terminals, where Ctrl + C would interrupt the running program. Clipboard contents are backed up and restored. The classic console window and password fields are refused.

Some terminal apps, such as Claude Code, handle the mouse themselves, so Windows Terminal has no selection to read. There, **Ctrl + Alt + D** looks up the word under the mouse pointer instead, so double-click a word and leave the pointer on it. Read-aloud still needs a terminal selection: hold **Shift** while selecting.

Lookups use cached entries first. Otherwise DIO asks [Simple English Wiktionary](https://simple.wiktionary.org/), which writes definitions in plain English for learners, and the [Free Dictionary API](https://dictionaryapi.dev/) at the same time. The compact popup shows the Simple English definitions when they exist, and **Expand** adds the Free Dictionary API's fuller list of meanings below them. Other words use the Free Dictionary API's definitions. Words are pronounced by the Kokoro voice when it is installed, with the Free Dictionary API's recordings as the fallback. The bundled SQLite dictionary is used when offline or when neither has an entry. Names, DIO itself and ALL-CAPS acronyms always come from the bundled data.

Pronunciation is resolved after the definition is shown, so missing data never blocks a definition. Sources are tried in this order:

1. IPA from the Free Dictionary API.
2. Bundled Wiktionary transcriptions tagged US or UK.
3. Bundled CMUdict entries converted to American IPA.
4. Kokoro's pronunciation rules, marked **≈**. These are not dictionary transcriptions.

Accents are labelled only when the source labels them, and unknown words get no IPA rather than a guess.

Speech runs in-process on the CPU through ONNX Runtime. There is no Python runtime, no background server, and no extra process. Text you read aloud stays in memory only while it plays. The only speech-related network traffic is the model download, which you start yourself. It comes from a pinned Hugging Face commit, and each file is verified against a SHA-256 checksum built into DIO. A damaged model is detected and removed. DIO uses Kokoro's own text-to-phoneme rules (misaki) without eSpeak NG, which keeps it permissively licensed.

Settings, cached definitions, saved vocabulary, and the voice model are stored locally. To look for updates, DIO fetches a small file from GitHub Releases at most once a day, or only when you click **Check for updates** if you chose to check manually.

## Tech Stack

- **Rust & Tauri**: desktop application and native Windows integration.
- **SQLite**: embedded offline dictionaries and pronunciation index.
- **Windows UI Automation**: system-wide text selection.
- **ONNX Runtime, Kokoro-82M & misaki-rs**: local speech synthesis.
- **rodio**: audio playback.
- **Simple English Wiktionary**: plain-English online definitions.
- **Free Dictionary API**: online definitions and pronunciation.

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

### Release

Pushes and pull requests to `main` run `.github/workflows/ci.yml`, which checks the version files against `CHANGELOG.md` and runs the tests. It never publishes.

To release:

1. Set the same version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Add its entry to `CHANGELOG.md` with a `Summary:` line and dated `## [X.Y.Z] - YYYY-MM-DD`. This one entry becomes the GitHub release notes, the notes DIO shows when it updates, and the website's changelog.
3. Run `node tools/changelog.mjs check X.Y.Z --release`, commit to `main`, then tag and push:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

`.github/workflows/release.yml` checks the tag, versions and changelog, runs the tests, then builds, signs and publishes the installer, its signature, `latest.json` for the updater, and a `DIO-setup.exe` copy for the website. A tag with a suffix such as `vX.Y.Z-rc.1` publishes a prerelease, which installed copies and the website ignore.

One-time setup for signing:

```bash
pnpm tauri signer generate -w ~/.tauri/dio.key
```

Put the public key in `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`. Create a `release` environment in the repository settings with yourself as a required reviewer, and add the private key and its password there as `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Keep a backup of the key: without it, installed copies cannot be updated.

The installer's update preference page is part of `src-tauri/installer/installer.nsi`, a copy of Tauri's NSIS template with DIO's changes marked `; DIO:`. Re-apply them to the new upstream template when upgrading the Tauri CLI.

### Website

The download page lives in `site/` and renders the Context design from `prototypes/design/`; `site/changelog/` renders `CHANGELOG.md`. Run `pnpm site:build` to build it. `.github/workflows/pages.yml` publishes it to GitHub Pages when the site or design changes on `main`.

### Tests

```bash
pnpm test
pnpm build

cd src-tauri
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --release -- --ignored   # downloads the real model and synthesizes on the CPU
```

### Data and Runtime

- `src-tauri/resources/pronunciations.sqlite3` is built with `python tools/build_pronunciation_dictionary.py <kaikki-English.jsonl.gz> <cmudict.dict> <out.sqlite3>`, which checks pinned source checksums.
- `src-tauri/resources/onnxruntime.dll` (ONNX Runtime 1.31.1, CPU) is refreshed with `python tools/fetch_onnxruntime.py src-tauri/resources`.

## Platform Support

| Platform | Status |
|---|---|
| Windows | Supported |
| Linux | Planned |

DIO currently supports English. Lookups accept continuous selections of up to 128 characters; read-aloud accepts up to the configured limit.

Known limitations:

- Words outside Kokoro's lexicon and CMUdict are spelled out when spoken and get no IPA.
- The first speech after launch waits about a second while the model loads.
- Apps running as administrator, games, and remote desktop windows may block selection capture.

Build checks, API responses, and CPU-only synthesis have been verified. Native interactions, audio playback, terminal capture, and mixed-DPI positioning require further on-device testing.

## Attribution

DIO uses dictionary resources from Open English WordNet, CloudBytes Academy, Wiktionary, Simple English Wiktionary, CMUdict, and the Free Dictionary API, and speech from Kokoro-82M, misaki, and ONNX Runtime.

- [English Dictionary Licence](src-tauri/resources/DICTIONARY-LICENSE.md)
- [Acronym Dictionary Licence](src-tauri/resources/ACRONYM-LICENSE.md)
- [Pronunciation Data Licence](src-tauri/resources/PRONUNCIATION-LICENSE.md)
- [Speech Licence](src-tauri/resources/SPEECH-LICENSE.md)
- [ONNX Runtime Licence](src-tauri/resources/ONNXRUNTIME-LICENSE.txt)

Third-party content remains subject to its respective licensing and attribution requirements.
