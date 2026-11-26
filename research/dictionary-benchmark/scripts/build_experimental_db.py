"""Build isolated OpenGloss SQLite definition databases from pinned Parquet shards."""

import argparse
import json
import sqlite3
import sys
import time
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--shards", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--mode", choices=("neutral", "neutral-grade5"), required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--vendor", type=Path)
    parser.add_argument("--metrics", type=Path, required=True)
    args = parser.parse_args()
    if args.vendor:
        sys.path.insert(0, str(args.vendor.resolve()))
    import pyarrow.parquet as parquet

    shards = sorted(args.shards.glob("*.parquet"))
    if not shards:
        raise FileNotFoundError(f"No Parquet shards in {args.shards}")
    if args.output.exists():
        raise FileExistsError(args.output)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    started = time.perf_counter()
    connection = sqlite3.connect(args.output)
    connection.execute("PRAGMA journal_mode=OFF")
    connection.execute("PRAGMA synchronous=OFF")
    connection.execute(
        "CREATE TABLE definitions ("
        "word TEXT NOT NULL, pos TEXT NOT NULL, sense_id TEXT NOT NULL, "
        "sense_index INTEGER NOT NULL, reading_level TEXT NOT NULL, "
        "definition TEXT NOT NULL, example TEXT NOT NULL, "
        "tier TEXT NOT NULL, qa_flags TEXT NOT NULL)"
    )
    connection.execute("CREATE TABLE metadata (name TEXT PRIMARY KEY, value TEXT NOT NULL)")
    metadata = {
        "source": "mjbommar/opengloss-v2.4-definitions",
        "source_revision": args.source_revision,
        "mode": args.mode,
        "included_rows": "neutral/plain canonical only; grade_5/plain when mode is neutral-grade5",
        "normalization": "headword.casefold().replace('_', ' ')",
        "examples": "not included; requires companion examples dataset",
    }
    connection.executemany(
        "INSERT INTO metadata VALUES (?, ?)", metadata.items()
    )
    source_rows = 0
    kept_neutral = 0
    kept_grade5 = 0
    qa_flagged = 0
    fields = [
        "sense_id", "headword", "pos", "sense_index", "tier",
        "reading_level", "register", "text", "is_canonical", "qa_flags",
    ]
    insert = (
        "INSERT INTO definitions "
        "(word,pos,sense_id,sense_index,reading_level,definition,example,tier,qa_flags) "
        "VALUES (?,?,?,?,?,?,?,?,?)"
    )
    try:
        for shard in shards:
            file = parquet.ParquetFile(shard)
            for batch in file.iter_batches(batch_size=8192, columns=fields):
                source_rows += batch.num_rows
                rows = []
                for row in batch.to_pylist():
                    neutral = (
                        row["is_canonical"] is True
                        and row["reading_level"] == "neutral"
                        and row["register"] == "plain"
                    )
                    grade5 = (
                        args.mode == "neutral-grade5"
                        and row["reading_level"] == "grade_5"
                        and row["register"] == "plain"
                    )
                    if not (neutral or grade5) or not row["text"]:
                        continue
                    if not row["headword"] or not row["sense_id"]:
                        continue
                    flags = row["qa_flags"] or []
                    qa_flagged += bool(flags)
                    kept_neutral += neutral
                    kept_grade5 += grade5
                    rows.append((
                        row["headword"].casefold().replace("_", " "),
                        row["pos"] or "",
                        row["sense_id"],
                        row["sense_index"] or 0,
                        row["reading_level"],
                        row["text"],
                        "",
                        row["tier"] or "",
                        json.dumps(flags, ensure_ascii=False),
                    ))
                connection.executemany(insert, rows)
            connection.commit()
        connection.execute(
            "CREATE INDEX definitions_word_level_order "
            "ON definitions(word, reading_level, sense_index)"
        )
        connection.execute(
            "CREATE INDEX definitions_sense ON definitions(sense_id)"
        )
        connection.commit()
        connection.execute("VACUUM")
    finally:
        connection.close()
    elapsed = time.perf_counter() - started
    result = {
        "database": str(args.output.resolve()),
        "source_revision": args.source_revision,
        "mode": args.mode,
        "shards": [str(p.resolve()) for p in shards],
        "source_rows_scanned": source_rows,
        "neutral_rows": kept_neutral,
        "grade5_rows": kept_grade5,
        "qa_flagged_rows": qa_flagged,
        "build_seconds": elapsed,
        "database_bytes": args.output.stat().st_size,
        "python_version": sys.version,
        "pyarrow_version": __import__("pyarrow").__version__,
        "sqlite_version": sqlite3.sqlite_version,
    }
    args.metrics.parent.mkdir(parents=True, exist_ok=True)
    args.metrics.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "mode": args.mode,
        "neutral_rows": kept_neutral,
        "grade5_rows": kept_grade5,
        "database_bytes": result["database_bytes"],
        "build_seconds": elapsed,
    }))


if __name__ == "__main__":
    main()
