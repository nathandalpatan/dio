"""Build DIO's bundled SQLite dictionary from CloudBytes v2 JSONL."""

import gzip
import hashlib
import json
import sqlite3
import sys
from pathlib import Path


SOURCE_SHA256 = "b8fa47f251ee422e2023748bbdc026ff9df741acd250d067dd64e41877a60526"


def build(source: Path, destination: Path) -> None:
    if hashlib.sha256(source.read_bytes()).hexdigest() != SOURCE_SHA256:
        raise ValueError("Dictionary source checksum does not match the pinned release")

    temporary = destination.with_suffix(".tmp")
    if temporary.exists():
        temporary.unlink()
    database = sqlite3.connect(temporary)
    try:
        database.execute("PRAGMA journal_mode=OFF")
        database.execute("PRAGMA synchronous=OFF")
        database.execute(
            "CREATE TABLE definitions (word TEXT NOT NULL, pos TEXT NOT NULL, "
            "definition TEXT NOT NULL, example TEXT NOT NULL, synset_id TEXT NOT NULL)"
        )
        with gzip.open(source, "rt", encoding="utf-8") as rows:
            for line in rows:
                row = json.loads(line)
                if row["source"] != "oewn" or not row["definition"].strip():
                    continue
                database.execute(
                    "INSERT INTO definitions VALUES (?, ?, ?, ?, ?)",
                    (
                        row["word"].casefold().replace("_", " "),
                        row["pos"] or "",
                        row["definition"],
                        row["examples"][0] if row["examples"] else "",
                        row["synset_id"] or "",
                    ),
                )
        database.execute("CREATE INDEX definitions_word ON definitions(word)")
        database.execute("CREATE INDEX definitions_synset ON definitions(synset_id)")
        database.commit()
        database.execute("VACUUM")
    finally:
        database.close()
    temporary.replace(destination)


if __name__ == "__main__":
    build(Path(sys.argv[1]), Path(sys.argv[2]))
