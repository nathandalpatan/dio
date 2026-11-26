"""Create auditable automatic metrics and blind editorial packets from raw lookups.

No semantic or editorial score is inferred by this script. Optional ratings are
joined only when supplied explicitly through a reviewer-completed CSV.
"""

import argparse
import csv
import json
import math
import random
import re
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CONFIGURATIONS = (
    "dio_offline", "dio_online", "opengloss_standard",
    "opengloss_simplified", "hybrid",
)
CRITERIA = {
    "semantic_accuracy": 30,
    "readability": 20,
    "conciseness": 15,
    "common_meaning_prioritisation": 15,
    "vocabulary_coverage": 10,
    "usage_example_quality": 5,
    "editorial_style_consistency": 5,
}
SEED = 20261125
WORDS = re.compile(r"[^\W\d_]+(?:['’][^\W\d_]+)?", re.UNICODE)


def read_csv(path: Path) -> list[dict]:
    with path.open(encoding="utf-8-sig", newline="") as stream:
        return list(csv.DictReader(stream))


def read_jsonl(path: Path) -> list[dict]:
    if not path.exists():
        return []
    with path.open(encoding="utf-8") as stream:
        return [json.loads(line) for line in stream if line.strip()]


def write_jsonl(path: Path, rows: list[dict]) -> None:
    with path.open("w", encoding="utf-8", newline="\n") as stream:
        for row in rows:
            stream.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def flatten_entries(entries) -> list[dict]:
    """Preserve source ordering across DIO Entry and flat OpenGloss records."""
    flat = []
    if not isinstance(entries, list):
        return flat
    for entry in entries:
        if isinstance(entry, str):
            flat.append({"pos": "", "text": entry, "example": ""})
            continue
        if not isinstance(entry, dict):
            continue
        if "meanings" in entry:
            for meaning in entry.get("meanings") or []:
                if not isinstance(meaning, dict):
                    continue
                pos = meaning.get("partOfSpeech", meaning.get("part_of_speech", ""))
                for definition in meaning.get("definitions") or []:
                    if isinstance(definition, str):
                        flat.append({"pos": pos, "text": definition, "example": ""})
                    elif isinstance(definition, dict):
                        flat.append({"pos": pos,
                                     "text": definition.get("definition", definition.get("text", "")),
                                     "example": definition.get("example", "") or ""})
        elif "definitions" in entry:
            for definition in entry.get("definitions") or []:
                if isinstance(definition, str):
                    flat.append({"pos": entry.get("pos", ""), "text": definition, "example": ""})
                elif isinstance(definition, dict):
                    flat.append({"pos": entry.get("pos", definition.get("pos", "")),
                                 "text": definition.get("definition", definition.get("text", "")),
                                 "example": definition.get("example", "") or ""})
        elif "text" in entry or "definition" in entry:
            flat.append({"pos": entry.get("pos", ""),
                         "text": entry.get("definition", entry.get("text", "")),
                         "example": entry.get("example", "") or ""})
    return [item for item in flat if isinstance(item["text"], str) and item["text"].strip()]


def syllables(word: str) -> int:
    word = word.casefold().strip("'’")
    if not word:
        return 0
    groups = re.findall(r"[aeiouy]+", word)
    count = len(groups)
    if word.endswith("e") and not word.endswith(("le", "ye")) and count > 1:
        count -= 1
    return max(1, count)


def readability(text: str) -> tuple[str, str]:
    words = WORDS.findall(text)
    if not words:
        return "", ""
    sentences = max(1, len(re.findall(r"[.!?]+(?:\s|$)", text)))
    n = len(words)
    s = sum(syllables(word) for word in words)
    ease = 206.835 - 1.015 * n / sentences - 84.6 * s / n
    grade = 0.39 * n / sentences + 11.8 * s / n - 15.59
    return f"{ease:.2f}", f"{grade:.2f}"


def automated_row(item: dict, raw: dict | None) -> dict:
    raw = raw or {}
    defs = flatten_entries(raw.get("entries", raw.get("definitions")))
    first = defs[0] if defs else {"text": "", "pos": "", "example": ""}
    texts = [definition["text"] for definition in defs]
    examples = [definition["example"] for definition in defs if definition["example"]]
    normalized = [" ".join(WORDS.findall(text.casefold())) for text in texts]
    duplicates = sum(count - 1 for count in Counter(normalized).values() if count > 1)
    term = item["term"]
    circular_candidate = bool(first["text"] and re.search(r"(?<!\w)" + re.escape(term.casefold()) + r"(?!\w)", first["text"].casefold()))
    ease, grade = readability(first["text"])
    status = raw.get("status", "not_retrieved")
    return {
        "id": item["id"], "term": term, "category": item["category"],
        "configuration": raw.get("configuration", ""), "status": status,
        "route": raw.get("route", ""),
        "exact_headword": raw.get("exact_headword", ""),
        "matched_headword": raw.get("matched_headword") or "",
        "sense_count": len(defs), "example_count": len(examples),
        "first_pos": first["pos"], "first_definition": first["text"],
        "first_definition_chars": len(first["text"]),
        "first_definition_words": len(WORDS.findall(first["text"])),
        "median_definition_words": median([len(WORDS.findall(text)) for text in texts]),
        "flesch_ease_estimate": ease, "flesch_kincaid_grade_estimate": grade,
        "exact_duplicate_definition_count": duplicates,
        "circular_word_reuse_candidate": circular_candidate,
        "elapsed_ms": raw.get("elapsed_ms", ""),
        "error_kind": raw.get("error_kind") or "",
        "blind_id": "",
        **{criterion: "" for criterion in CRITERIA},
        "weighted_quality_0_100": "",
        "relevant_sense_context_1": "", "relevant_sense_context_2": "",
        "first_definition_context_1": "", "first_definition_context_2": "",
        "independent_reference_urls": "", "evaluator_type": "",
    }


def median(values: list[int]) -> str:
    if not values:
        return ""
    ordered = sorted(values)
    middle = len(ordered) // 2
    result = ordered[middle] if len(ordered) % 2 else (ordered[middle - 1] + ordered[middle]) / 2
    return f"{result:g}"


def load_ratings(path: Path | None, valid_ids: set[str]) -> dict[str, dict]:
    if path is None or not path.exists():
        return {}
    ratings = {}
    for row in read_csv(path):
        blind_id = row.get("blind_id", "")
        if blind_id not in valid_ids:
            raise ValueError(f"Unknown blind_id in ratings: {blind_id}")
        if blind_id in ratings:
            raise ValueError(f"Duplicate rating: {blind_id}")
        for criterion in CRITERIA:
            value = row.get(criterion, "").strip()
            if value:
                score = float(value)
                if not math.isfinite(score) or not 0 <= score <= 5:
                    raise ValueError(f"Invalid {criterion} score for {blind_id}: {value}")
        ratings[blind_id] = row
    return ratings


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, default=ROOT / "benchmark_words.csv")
    parser.add_argument("--raw", type=Path, default=ROOT / "raw_results.jsonl")
    parser.add_argument("--ratings", type=Path, default=None)
    parser.add_argument("--out", type=Path, default=ROOT / "benchmark_results.csv")
    args = parser.parse_args()
    corpus = read_csv(args.corpus)
    if len(corpus) != 500 or len({item["term"].casefold() for item in corpus}) != 500:
        raise ValueError("Expected fixed 500-item unique corpus")
    raw_rows = read_jsonl(args.raw)
    by_key = {}
    for raw in raw_rows:
        key = (raw.get("item_id"), raw.get("configuration"))
        if key in by_key:
            raise ValueError(f"Duplicate raw result: {key}")
        by_key[key] = raw
    rows, packets, order_packets, keys = [], [], [], []
    rng = random.Random(SEED)
    for item in corpus:
        configurations = list(CONFIGURATIONS)
        rng.shuffle(configurations)
        for configuration in configurations:
            raw = by_key.get((item["id"], configuration))
            row = automated_row(item, raw)
            row["configuration"] = configuration
            if raw and raw.get("status") == "found":
                blind_id = f"P{len(packets) + 1:05d}"
                row["blind_id"] = blind_id
                definitions = flatten_entries(raw.get("entries", raw.get("definitions")))
                shuffled = list(definitions)
                rng.shuffle(shuffled)
                packets.append({"blind_id": blind_id, "term": item["term"],
                                "category": item["category"], "context_1": item["context_1"],
                                "context_2": item["context_2"],
                                "definitions_random_order": shuffled})
                order_packets.append({"blind_id": blind_id, "term": item["term"],
                                      "context_1": item["context_1"],
                                      "context_2": item["context_2"],
                                      "definitions_original_order": definitions})
                keys.append({"blind_id": blind_id, "item_id": item["id"],
                             "configuration": configuration,
                             "original_order": definitions})
            rows.append(row)
    valid_ids = {packet["blind_id"] for packet in packets}
    ratings = load_ratings(args.ratings, valid_ids)
    for row in rows:
        rating = ratings.get(row["blind_id"])
        if not rating:
            continue
        for criterion in CRITERIA:
            row[criterion] = rating.get(criterion, "").strip()
        if all(row[criterion] != "" for criterion in CRITERIA):
            row["weighted_quality_0_100"] = f"{sum(float(row[c]) * w for c, w in CRITERIA.items()) / 5:.2f}"
        for field in ("relevant_sense_context_1", "relevant_sense_context_2",
                      "first_definition_context_1", "first_definition_context_2",
                      "independent_reference_urls", "evaluator_type"):
            row[field] = rating.get(field, "").strip()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    write_jsonl(ROOT / "blinded_packets.jsonl", packets)
    write_jsonl(ROOT / "blinded_order_packets.jsonl", order_packets)
    write_jsonl(ROOT / "blind_key.jsonl", keys)
    template = ROOT / "ratings_template.csv"
    with template.open("w", newline="", encoding="utf-8") as stream:
        fields = ["blind_id", *CRITERIA, "relevant_sense_context_1",
                  "relevant_sense_context_2", "first_definition_context_1",
                  "first_definition_context_2", "independent_reference_urls",
                  "evaluator_type", "notes"]
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        writer.writerows({"blind_id": packet["blind_id"]} for packet in packets)
    print(f"Wrote {len(rows)} results, {len(packets)} blind packets, {len(ratings)} ratings")


if __name__ == "__main__":
    main()
