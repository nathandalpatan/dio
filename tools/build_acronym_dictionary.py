"""Build DIO's offline English acronym index from Kaikki/Wiktionary extracts."""

import hashlib
import json
import re
import sqlite3
import sys
from pathlib import Path


SOURCE_SHA256 = {
    "acronym": "1ce2a5c65d3427195d8226800d0b30a5892318cbd98d46251e793e92550b34fa",
    "initialism": "4876689c78c0b3d7e10a0c48f2ab0c016ad71e2b78dc9f9fc70f510f24e0d473",
    "abbreviation": "b70f32f87a7fac89e50a3c57b336e153468c68154b84ffe3d57ed13ed3545119",
}
EXPANDED_DICTIONARY_SHA256 = "28b00a03353acb833f4ece65aa62559e517d4df4c1ea36636cb7c9a768948a64"
EXPANSION = re.compile(r"^(?:Initialism|Acronym|Abbreviation) of (.+?)(?:[.;:]|$)")
COMMON_SENSES = (
    ("LOL", "interjection", "laugh out loud", "Laugh out loud: expresses amusement or gives a message a lighthearted tone."),
    ("LOL", "interjection", "lots of love", "Lots of love: an affectionate sign-off, especially in a message or letter."),
    ("OMG", "interjection", "oh my God", "Oh my God: expresses surprise, excitement, or shock."),
    ("POV", "noun", "point of view", "Point of view: a person's perspective, or the viewpoint from which a story or video is presented."),
    ("TLDR", "phrase", "too long; didn't read", "Too long; didn't read: introduces a brief summary of a longer text, or says it was skipped."),
    ("TL;DR", "phrase", "too long; didn't read", "Too long; didn't read: introduces a brief summary of a longer text, or says it was skipped."),
    ("IRL", "adverb", "in real life", "In real life: in the physical world rather than online or in a game."),
    ("DM", "noun", "direct message", "Direct message: a private message sent to someone on a social platform."),
    ("NGL", "phrase", "not gonna lie", "Not gonna lie: introduces a candid opinion or admission."),
    ("FYI", "phrase", "for your information", "For your information: introduces something the reader should know."),
    ("ICYMI", "adverb", "in case you missed it", "In case you missed it: introduces information being shared again for anyone who did not see it earlier."),
    ("TIL", "phrase", "today I learned", "Today I learned: introduces a fact the speaker has just discovered."),
    ("TTYL", "phrase", "talk to you later", "Talk to you later: a casual way to end a conversation for now."),
)


def add_common_senses(database: sqlite3.Connection) -> None:
    if "priority" not in {column[1] for column in database.execute("PRAGMA table_info(acronyms)")}:
        database.execute("ALTER TABLE acronyms ADD COLUMN priority INTEGER NOT NULL DEFAULT 0")
    for term, kind, expansion, explanation in COMMON_SENSES:
        existing = database.execute(
            "SELECT rowid FROM acronyms WHERE key=? AND instr(lower(definition), lower(?)) > 0 LIMIT 1",
            (term.lower(), expansion),
        ).fetchone()
        if existing:
            database.execute(
                "UPDATE acronyms SET explanation=?, priority=1 WHERE rowid=?",
                (explanation, existing[0]),
            )
        else:
            database.execute(
                "INSERT INTO acronyms (term, key, kind, definition, explanation, priority) "
                "VALUES (?, ?, ?, ?, ?, 1)",
                (term, term.lower(), kind, f"Initialism of {expansion}.", explanation),
            )


def enrich(database: sqlite3.Connection, dictionary_path: Path) -> int:
    if hashlib.sha256(dictionary_path.read_bytes()).hexdigest() != EXPANDED_DICTIONARY_SHA256:
        raise ValueError("expanded dictionary checksum does not match the pinned release")

    source = sqlite3.connect(f"file:{dictionary_path.resolve()}?mode=ro", uri=True)
    if "explanation" not in {column[1] for column in database.execute("PRAGMA table_info(acronyms)")}:
        database.execute("ALTER TABLE acronyms ADD COLUMN explanation TEXT")
    matched = 0
    try:
        query = (
            "SELECT s.glosses FROM lookup l JOIN senses s ON s.entry_id=l.entry_id "
            "WHERE l.surface=? ORDER BY l.rank, s.ord LIMIT 1"
        )
        for rowid, definition in database.execute("SELECT rowid, definition FROM acronyms"):
            match = EXPANSION.match(definition)
            if not match:
                continue
            expansion = re.sub(r"\s*\([^)]*\)$", "", match.group(1)).strip()
            for spelling in dict.fromkeys((expansion, expansion.lower(), expansion.title())):
                row = source.execute(query, (spelling,)).fetchone()
                if not row:
                    continue
                glosses = json.loads(row[0])
                explanation = next((gloss["text"] for gloss in glosses if gloss.get("lang") == "en"), None)
                if explanation and not explanation.startswith(("Alternative form of", "Synonym of", "Initialism of", "Acronym of", "Abbreviation of")):
                    display = f"{expansion[0].upper()}{expansion[1:]}: {explanation}"
                    database.execute("UPDATE acronyms SET explanation=? WHERE rowid=?", (display, rowid))
                    matched += 1
                    break
    finally:
        source.close()
    return matched


def build(acronyms: Path, initialisms: Path, abbreviations: Path, destination: Path, dictionary: Path) -> None:
    sources = (("acronym", acronyms), ("initialism", initialisms), ("abbreviation", abbreviations))
    for kind, source in sources:
        if hashlib.sha256(source.read_bytes()).hexdigest() != SOURCE_SHA256[kind]:
            raise ValueError(f"{kind} source checksum does not match the pinned extract")

    temporary = destination.with_suffix(".tmp")
    if temporary.exists():
        temporary.unlink()
    database = sqlite3.connect(temporary)
    try:
        database.execute("PRAGMA journal_mode=OFF")
        database.execute("PRAGMA synchronous=OFF")
        database.execute(
            "CREATE TABLE acronyms (term TEXT NOT NULL, key TEXT NOT NULL, "
            "kind TEXT NOT NULL, definition TEXT NOT NULL)"
        )
        for kind, source in sources:
            with source.open(encoding="utf-8") as rows:
                for line in rows:
                    entry = json.loads(line)
                    if entry.get("lang_code") != "en":
                        continue
                    term = entry["word"].strip()
                    if not term or len(term) > 128:
                        continue
                    if kind == "abbreviation":
                        letters = [char for char in term if char.isalpha()]
                        if len(letters) < 2 or not all(char.isupper() for char in letters):
                            continue
                    for sense in entry.get("senses", []):
                        if kind not in sense.get("tags", []):
                            continue
                        for definition in sense.get("glosses", []):
                            definition = definition.strip()
                            if definition:
                                database.execute(
                                    "INSERT INTO acronyms VALUES (?, ?, ?, ?)",
                                    (term, term.lower(), kind, definition),
                                )
        database.execute(
            "DELETE FROM acronyms WHERE rowid NOT IN "
            "(SELECT MIN(rowid) FROM acronyms GROUP BY term, definition)"
        )
        enrich(database, dictionary)
        add_common_senses(database)
        database.execute("CREATE INDEX acronyms_key ON acronyms(key)")
        database.commit()
        database.execute("VACUUM")
    finally:
        database.close()
    temporary.replace(destination)


if __name__ == "__main__":
    build(*(Path(arg) for arg in sys.argv[1:6]))
