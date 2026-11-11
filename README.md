# DIO

DIO is a Windows desktop dictionary. Highlight a word or short expression in an application that exposes its selection through Windows UI Automation, then press **Ctrl+Alt+D**. A popup shows the first English definition from the [Free Dictionary API](https://dictionaryapi.dev/). Use **More** for the remaining definitions, examples, related words, origins, and attribution when the provider supplies them.

## Install and use

Install the NSIS package from `src-tauri/target/release/bundle/nsis/` after building it. DIO runs in the system tray. Right-click its icon for **Settings** or **Quit DIO**.

1. Highlight one continuous selection of at most 128 characters.
2. Press **Ctrl+Alt+D**. If another application owns that global shortcut, open Settings from the tray and choose another binding.
3. Click **More**, **Listen**, a related word, or the star to expand, play pronunciation, navigate, or save the entry. Press **Escape** or click outside to dismiss. Press the shortcut again while the popup is visible to focus it.

The optional middle-click trigger passes the click through to the source application, so that application may also perform its normal middle-click action. It is disabled by default. Keyboard and middle-click triggers can be enabled independently.

## Privacy and data

DIO reads highlighted text through Windows UI Automation. It does not use the clipboard or simulate copying. It sends the selected text to `api.dictionaryapi.dev` for a definition. Available pronunciation audio is downloaded through the Rust backend from a restricted set of HTTPS hosts. Settings, successful lookup cache, and saved vocabulary are stored locally in the user's application data directory. Saved words remain after cache clearing or eviction. The cache is fresh for 30 days and holds at most 1,000 entries. If refreshing a cached entry fails, DIO displays the stale entry with an offline notice.

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

This release targets Windows and English dictionary entries. Selection works only where an application exposes highlighted text through UI Automation TextPattern. Password fields, empty selections, multiple disjoint selections, and selections longer than 128 characters are rejected. Some applications restrict accessibility access or do not expose selection bounds; DIO explains unsupported selections and uses the pointer as a positioning fallback. Global shortcut registration detects conflicts reported by Windows, not every application-local binding.

The API response and build checks have been verified. Interactive selection, focus preservation, middle-click pass-through, Escape, outside-click dismissal, audio playback, and mixed-DPI placement still need on-device testing. Linux integration is deferred.
