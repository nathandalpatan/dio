# DIO

A lightweight, system-wide dictionary for Windows. Highlight a word, press **Ctrl + Alt + D**, and get a definition without leaving your application.

Built with Rust and Tauri, DIO combines offline dictionary resources with online lookups in a fast, minimal, privacy-conscious desktop experience.

## Features

- **Instant lookups** — Look up selected text across supported Windows applications.
- **Offline dictionary** — Over 152,000 English headwords, available without an internet connection.
- **Acronym support** — Over 32,000 meanings across 16,000 acronyms, initialisms, and abbreviations.
- **Rich definitions** — Definitions, pronunciations, examples, synonyms, and word origins when available.
- **Saved vocabulary** — Bookmark words for future reference.
- **Customisable controls** — Configure keyboard shortcuts and optional middle-click activation.
- **Local-first privacy** — Offline lookups, clipboard preservation, and local data storage.

## Getting Started

1. Install DIO using the generated Windows installer.
2. Highlight a word or short expression in any supported application.
3. Press **Ctrl + Alt + D** to open the dictionary popup.
4. Select **More** to explore additional definitions, or **Listen** to hear pronunciation when available.

Press **Escape** or click outside to dismiss the popup.

DIO runs in the system tray. Right-click its icon to access Settings or quit the application.

## How It Works

DIO retrieves selected text using Windows UI Automation, with a clipboard fallback when necessary.

Lookups prioritise bundled SQLite dictionaries and cached entries before querying the [Free Dictionary API](https://dictionaryapi.dev/).

Settings, cached definitions, and saved vocabulary are stored locally. Clipboard contents are preserved whenever supported, and network requests are limited to online lookups and pronunciation audio.

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
```

## Platform Support

| Platform | Status |
|---|---|
| Windows | Supported |
| Linux | Planned |

DIO currently supports English dictionary entries and continuous text selections of up to 128 characters.

Build checks and API responses have been verified. Native interactions, audio playback, and mixed-DPI positioning require further on-device testing.

## Attribution

DIO uses dictionary resources from Open English WordNet, CloudBytes Academy, Wiktionary, and the Free Dictionary API.

- [English Dictionary Licence](src-tauri/resources/DICTIONARY-LICENSE.md)
- [Acronym Dictionary Licence](src-tauri/resources/ACRONYM-LICENSE.md)

Third-party content remains subject to its respective licensing and attribution requirements.
