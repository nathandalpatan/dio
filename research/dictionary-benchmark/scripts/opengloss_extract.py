"""Download pinned OpenGloss Parquet files and extract benchmark definitions."""

import argparse
import csv
import hashlib
import json
import sys
import time
from collections import defaultdict
from pathlib import Path

import requests


REPOSITORIES = {
    "definitions": "mjbommar/opengloss-v2.4-definitions",
    "examples": "mjbommar/opengloss-v2.4-examples",
    "inflections": "mjbommar/opengloss-v2.4-inflections",
    "senses": "mjbommar/opengloss-v2.4-senses",
    "lexicon": "mjbommar/opengloss-v2.4-lexicon",
    "relations": "mjbommar/opengloss-v2.4-relations",
}
ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / ".cache"
sys.path.insert(0, str(CACHE / "vendor"))


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def request_json(url):
    response = requests.get(url, timeout=(20, 120))
    response.raise_for_status()
    return response.json()


def download(args):
    repository = REPOSITORIES[args.repo]
    revision = args.revision or request_json(
        f"https://huggingface.co/api/datasets/{repository}"
    )["sha"]
    metadata = request_json(
        f"https://huggingface.co/api/datasets/{repository}/revision/{revision}?blobs=true"
    )
    if metadata["sha"] != revision:
        raise ValueError("Resolved revision differs from requested revision")
    destination = CACHE / f"opengloss-v2.4-{args.repo}"
    manifest = {
        "repository": repository,
        "revision": revision,
        "files": [],
    }
    for sibling in metadata["siblings"]:
        name = sibling["rfilename"]
        if not name.endswith(".parquet"):
            continue
        path = destination / name
        path.parent.mkdir(parents=True, exist_ok=True)
        expected = (sibling.get("lfs") or {}).get("sha256")
        size = sibling.get("size")
        if not (path.exists() and path.stat().st_size == size and sha256(path) == expected):
            path.unlink(missing_ok=True)
            url = f"https://huggingface.co/datasets/{repository}/resolve/{revision}/{name}"
            print(f"downloading {name} ({size} bytes)", flush=True)
            with requests.get(url, stream=True, timeout=(20, 120)) as response:
                response.raise_for_status()
                with path.open("wb") as output:
                    for chunk in response.iter_content(chunk_size=1024 * 1024):
                        if chunk:
                            output.write(chunk)
            if path.stat().st_size != size or sha256(path) != expected:
                path.unlink(missing_ok=True)
                raise ValueError(f"Checksum or size mismatch: {name}")
        manifest["files"].append({"path": name, "bytes": size, "sha256": expected})
        print(f"verified {name}", flush=True)
    manifest_path = ROOT / f"opengloss_{args.repo}_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {manifest_path}", flush=True)


def parquet_rows(folder, columns=None):
    try:
        import pyarrow.parquet as pq
    except ImportError as error:
        raise SystemExit("Install pyarrow in the benchmark environment") from error
    shards = sorted(folder.glob("data/*.parquet"))
    if not shards:
        raise SystemExit(f"No Parquet files in {folder}")
    for shard in shards:
        parquet = pq.ParquetFile(shard)
        names = set(parquet.schema_arrow.names)
        selected = [name for name in columns if name in names] if columns else None
        for batch in parquet.iter_batches(batch_size=32768, columns=selected):
            yield from batch.to_pylist()


def extract(args):
    corpus = list(csv.DictReader(args.corpus.open(encoding="utf-8-sig", newline="")))
    terms = defaultdict(list)
    for item in corpus:
        terms[item["term"].casefold()].append(item)
    levels = {"neutral": "opengloss_standard", "grade_5": "opengloss_simplified"}
    matching = defaultdict(list)
    scanned = 0
    started = time.perf_counter()
    for row in parquet_rows(CACHE / "opengloss-v2.4-definitions", [
        "sense_id", "lexeme_id", "headword", "pos", "sense_index", "domain", "tier",
        "reading_level", "register", "text", "readability_grade", "is_canonical", "qa_flags",
    ]):
        scanned += 1
        if row["headword"].casefold() not in terms or row["register"] != "plain":
            continue
        level = row["reading_level"]
        if level not in levels or (level == "neutral" and not row["is_canonical"]):
            continue
        for item in terms[row["headword"].casefold()]:
            matching[(item["id"], levels[level])].append((scanned, row))
    revision = json.loads((ROOT / "opengloss_definitions_manifest.json").read_text(encoding="utf-8"))["revision"]
    url = f"https://huggingface.co/datasets/{REPOSITORIES['definitions']}/tree/{revision}"
    with args.output.open("w", encoding="utf-8") as output:
        for item in corpus:
            for configuration in levels.values():
                rows = matching.get((item["id"], configuration), [])
                meanings = []
                for _, row in sorted(rows, key=lambda pair: pair[0]):
                    pos = row["pos"]
                    if not meanings or meanings[-1]["partOfSpeech"] != pos:
                        meanings.append({"partOfSpeech": pos, "definitions": [], "synonyms": [], "antonyms": []})
                    meanings[-1]["definitions"].append({
                        "definition": row["text"], "example": "", "synonyms": [], "antonyms": [],
                        "sense_id": row["sense_id"], "sense_index": row["sense_index"],
                        "qa_flags": row["qa_flags"], "tier": row["tier"], "domain": row["domain"],
                    })
                record = {
                    "item_id": item["id"], "term": item["term"], "configuration": configuration,
                    "status": "found" if meanings else "missing", "entries": [{
                        "word": rows[0][1]["headword"], "meanings": meanings, "sourceUrls": [url],
                        "license": {"name": "OpenGloss CC BY 4.0", "url": "https://creativecommons.org/licenses/by/4.0/"},
                    }] if meanings else [], "elapsed_ms": None, "source_urls": [url],
                    "error_kind": None, "error_message": None,
                    "matched_headword": rows[0][1]["headword"] if rows else None,
                    "reading_level": "neutral" if configuration.endswith("standard") else "grade_5",
                    "sense_ids": [row["sense_id"] for _, row in rows], "revision": revision,
                    "examples_loaded": False,
                }
                output.write(json.dumps(record, ensure_ascii=False) + "\n")
    print(f"scanned {scanned} definition rows in {time.perf_counter() - started:.1f}s", flush=True)
    print(f"wrote {args.output}", flush=True)


def inspect(args):
    try:
        import pyarrow.parquet as pq
    except ImportError as error:
        raise SystemExit("Install pyarrow in the benchmark environment") from error
    for name in args.repos:
        folder = CACHE / f"opengloss-v2.4-{name}"
        shards = sorted(folder.glob("data/*.parquet"))
        total = 0
        for shard in shards:
            parquet = pq.ParquetFile(shard)
            total += parquet.metadata.num_rows
            print(json.dumps({"repo": name, "file": shard.name, "bytes": shard.stat().st_size,
                              "rows": parquet.metadata.num_rows, "schema": str(parquet.schema_arrow)}, ensure_ascii=False))
        print(json.dumps({"repo": name, "files": len(shards), "rows": total}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    get = commands.add_parser("download")
    get.add_argument("--repo", choices=REPOSITORIES, required=True)
    get.add_argument("--revision", help="Immutable Hugging Face dataset commit SHA")
    get.set_defaults(func=download)
    pick = commands.add_parser("extract")
    pick.add_argument("--corpus", type=Path, default=ROOT / "benchmark_words.csv")
    pick.add_argument("--output", type=Path, default=CACHE / "opengloss_records.jsonl")
    pick.set_defaults(func=extract)
    scan = commands.add_parser("inspect")
    scan.add_argument("repos", nargs="+", choices=REPOSITORIES)
    scan.set_defaults(func=inspect)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
