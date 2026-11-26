"""Measure index storage by copying a SQLite DB, dropping indexes, and vacuuming."""

import argparse
import json
import sqlite3
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("database", type=Path)
    parser.add_argument("indexless_copy", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = args.database.resolve()
    destination = args.indexless_copy.resolve()
    if source == destination:
        raise ValueError("Indexless copy must not be the source DB")
    if destination.exists():
        raise FileExistsError(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)

    with sqlite3.connect(f"file:{source.as_posix()}?mode=ro", uri=True) as original:
        with sqlite3.connect(destination) as copy:
            original.backup(copy)
    with sqlite3.connect(destination) as copy:
        names = [row[0] for row in copy.execute(
            "SELECT name FROM sqlite_master WHERE type='index' "
            "AND name NOT LIKE 'sqlite_autoindex_%' ORDER BY name"
        )]
        for name in names:
            copy.execute(f'DROP INDEX "{name.replace(chr(34), chr(34) * 2)}"')
        copy.commit()
        copy.execute("VACUUM")
    result = {
        "source": str(source),
        "original_bytes": source.stat().st_size,
        "indexless_copy": str(destination),
        "indexless_bytes": destination.stat().st_size,
        "removed_index_names": names,
        "index_storage_delta_bytes": source.stat().st_size - destination.stat().st_size,
        "method": "SQLite backup, DROP INDEX, VACUUM; delta includes page-layout effects",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
