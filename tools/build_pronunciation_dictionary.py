"""Build DIO's offline IPA index from Kaikki/Wiktionary and CMUdict."""

import gzip
import hashlib
import json
import re
import sqlite3
import sys
import unicodedata
from pathlib import Path


KAIKKI_SHA256 = "4d03b6cba283744756c57ff661c2c6cbfe34fcf8ff513a966576cf7d97e91a99"
CMUDICT_SHA256 = "81917843c7f44ce2b094ac63873c2c7a4cf802040792c455ba3ca406891c3d22"
DIALECTS = {
    "us": {"US", "General-American"},
    "uk": {"UK", "Received-Pronunciation"},
}
EXCLUDED = {
    "rare", "obsolete", "archaic", "dated", "nonstandard", "proscribed", "Early", "Modern",
    "Middle", "dialectal", "colloquial", "informal", "humorous", "nonce-word",
}
PER_DIALECT = 3
ARPABET = re.compile(r"^[A-Z]{1,2}[0-2]?( [A-Z]{1,2}[0-2]?)*$")


def checksum(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def dialect(tags: list[str]) -> str | None:
    """Return us/uk when exactly one standard accent is tagged, "" when untagged."""
    if not tags:
        return ""
    if set(tags) & EXCLUDED:
        return None
    matches = [name for name, markers in DIALECTS.items() if set(tags) & markers]
    return matches[0] if len(matches) == 1 else None


def wiktionary(database: sqlite3.Connection, path: Path) -> int:
    seen: dict[str, list[tuple[str, str]]] = {}
    with gzip.open(path, "rt", encoding="utf-8") as source:
        for line in source:
            entry = json.loads(line)
            if entry.get("lang_code") != "en" or not entry.get("sounds"):
                continue
            key = unicodedata.normalize("NFC", entry.get("word", "").strip().lower())
            if not key or len(key) > 64:
                continue
            known = seen.setdefault(key, [])
            for sound in entry["sounds"]:
                ipa = unicodedata.normalize("NFC", sound.get("ipa", "").strip())
                name = dialect(sound.get("tags", []))
                if not (ipa.startswith("/") and ipa.endswith("/")) or "-" in ipa or name is None:
                    continue
                if (ipa, name) in known or sum(1 for _, d in known if d == name) >= PER_DIALECT:
                    continue
                known.append((ipa, name))
    rows = [
        (key, ipa, name, rank)
        for key, values in seen.items()
        for rank, (ipa, name) in enumerate(values)
    ]
    database.executemany("INSERT INTO wiktionary VALUES (?, ?, ?, ?)", rows)
    return len(rows)


def cmudict(database: sqlite3.Connection, path: Path) -> int:
    ranks: dict[str, int] = {}
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        word, _, arpabet = line.partition(" ")
        arpabet = arpabet.split("#", 1)[0].strip()
        key = re.sub(r"\(\d+\)$", "", word).lower()
        if not key or not ARPABET.match(arpabet):
            continue
        rank = ranks.get(key, 0)
        ranks[key] = rank + 1
        rows.append((key, arpabet, rank))
    database.executemany("INSERT INTO cmudict VALUES (?, ?, ?)", rows)
    return len(rows)


def main() -> None:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: build_pronunciation_dictionary.py <kaikki-English.jsonl.gz> <cmudict.dict> <out.sqlite3>"
        )
    kaikki, cmu, output = map(Path, sys.argv[1:])
    if checksum(kaikki) != KAIKKI_SHA256:
        raise ValueError("Kaikki extract checksum does not match the pinned release")
    if checksum(cmu) != CMUDICT_SHA256:
        raise ValueError("CMUdict checksum does not match the pinned commit")
    output.unlink(missing_ok=True)
    database = sqlite3.connect(output)
    database.executescript(
        "CREATE TABLE wiktionary (key TEXT NOT NULL, ipa TEXT NOT NULL, dialect TEXT NOT NULL, rank INTEGER NOT NULL);"
        "CREATE TABLE cmudict (key TEXT NOT NULL, arpabet TEXT NOT NULL, rank INTEGER NOT NULL);"
    )
    wiktionary_rows = wiktionary(database, kaikki)
    cmudict_rows = cmudict(database, cmu)
    database.executescript(
        "CREATE INDEX wiktionary_key ON wiktionary(key, rank);"
        "CREATE INDEX cmudict_key ON cmudict(key, rank);"
    )
    database.commit()
    database.execute("VACUUM")
    database.close()
    print(f"{wiktionary_rows} Wiktionary and {cmudict_rows} CMUdict pronunciations")


if __name__ == "__main__":
    main()
