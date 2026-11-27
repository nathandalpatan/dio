# Changelog

Release notes for DIO, newest first. This file is the single source for the GitHub release, the notes DIO shows when it updates, and the [changelog page](https://nathandalpatan.github.io/dio/changelog/).

Each release has a `Summary:` line and any of these sections: New features, Improvements, Bug fixes, Breaking changes. Write `Unreleased` instead of a date until the release is tagged.

## [0.2.0] - 2026-11-27

Summary: DIO now keeps itself up to date, and the popup can be resized, shows synonyms by meaning and looks up words you click.

### New features

- **Automatic updates:** DIO can install new versions in the background, tell you when one is available, or wait for you to check. Choose during installation or in **Settings → Updates**. Every update is verified against DIO's signing key, and your settings, saved words and voice model are kept.
- **Resizable popup:** drag any edge of the popup to resize it. DIO remembers the size you choose.
- **Text size:** choose compact, standard or large text in **Settings → Appearance**.
- **Click to look up:** click a word in a definition or example to look it up.
- **Forward history:** after going back, the popup's **Forward** button returns to the next word.
- **Changelog:** see what changed in each version on the website.

### Improvements

- Synonyms are grouped by meaning, and the synonyms of the shown sense appear under the definition.
- Offline entries appear at once, and word audio is prepared while you read.
- Expanded entries keep their section tabs.
- Saved words are listed as dated rows you can search.
- DIO has a new monogram icon.

## [0.1.1] - 2026-11-21

Summary: Lookups now work in Adobe Acrobat and in terminal apps such as Claude Code.

### Improvements

- **Claude Code and other terminal apps:** **Ctrl + Alt + D** looks up the word under the mouse pointer when the terminal has no selection. Read aloud still needs a terminal selection, so hold **Shift** while selecting; the popup now says so.

### Bug fixes

- **Read aloud:** the popup's close button and Escape now close the Ctrl + Alt + R popup and stop the reading.
- **Adobe Acrobat and other PDF readers:** lookups now work, and punctuation that a double-click picks up with the word is left out.

## [0.1.0] - 2026-11-20

Summary: The first release of DIO, a pop-up dictionary for Windows.

### New features

- 152,000 English words and 32,000 acronym meanings work offline.
- Plain-English definitions from Simple English Wiktionary, with fuller meanings on **Expand**.
- US and UK IPA, and local speech with Kokoro that never leaves your computer.
- Read any selection aloud with **Ctrl + Alt + R**.
- Works in browsers, Adobe Acrobat and other PDF readers, Windows Terminal and Steam.
