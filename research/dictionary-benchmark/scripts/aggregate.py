"""Aggregate measured benchmark outcomes; omit unobserved editorial statistics."""

import argparse
import csv
import json
import random
import statistics
from collections import Counter, defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SEED = 20261125


def load(path: Path) -> list[dict]:
    with path.open(encoding="utf-8-sig", newline="") as stream:
        return list(csv.DictReader(stream))


def wilson(successes: int, n: int) -> list[float] | None:
    if n == 0:
        return None
    z = 1.959963984540054
    p = successes / n
    denominator = 1 + z * z / n
    centre = (p + z * z / (2 * n)) / denominator
    spread = z * ((p * (1 - p) + z * z / (4 * n)) / n) ** 0.5 / denominator
    return [round(centre - spread, 4), round(centre + spread, 4)]


def numeric(rows: list[dict], key: str) -> list[float]:
    return [float(row[key]) for row in rows if row.get(key, "") not in ("", None)]


def describe(rows: list[dict]) -> dict:
    n = len(rows)
    counts = Counter(row["status"] for row in rows)
    found = counts["found"]
    exact = sum(row.get("exact_headword", "").casefold() == "true" for row in rows)
    words = numeric([row for row in rows if row["status"] == "found"], "first_definition_words")
    grades = numeric([row for row in rows if row["status"] == "found"], "flesch_kincaid_grade_estimate")
    scores = numeric(rows, "weighted_quality_0_100")
    return {
        "items": n, "status_counts": dict(counts),
        "retrieved_definition_rate": round(found / n, 4) if n else None,
        "retrieved_definition_rate_95pct_wilson": wilson(found, n),
        "exact_headword_rate": round(exact / n, 4) if n else None,
        "exact_headword_rate_95pct_wilson": wilson(exact, n),
        "median_first_definition_words": statistics.median(words) if words else None,
        "first_definition_word_quantiles": quantiles(words),
        "median_flesch_kincaid_estimate": statistics.median(grades) if grades else None,
        "ratings_count": len(scores),
        "mean_weighted_quality_0_100": round(statistics.mean(scores), 2) if scores else None,
    }


def quantiles(values: list[float]) -> dict | None:
    if not values:
        return None
    ordered = sorted(values)
    return {f"p{p}": round(ordered[round((len(ordered) - 1) * p / 100)], 2)
            for p in (10, 25, 50, 75, 90)}


def paired_bootstrap(rows: list[dict], a: str, b: str, draws: int = 2000) -> dict | None:
    by_item = defaultdict(dict)
    for row in rows:
        if row["weighted_quality_0_100"]:
            by_item[row["id"]][row["configuration"]] = row
    pairs = [(observed[a], observed[b]) for observed in by_item.values() if a in observed and b in observed]
    if not pairs:
        return None
    strata = defaultdict(list)
    for left, right in pairs:
        strata[left["category"]].append(float(left["weighted_quality_0_100"]) - float(right["weighted_quality_0_100"]))
    n = sum(map(len, strata.values()))
    estimate = sum(sum(values) for values in strata.values()) / n
    rng = random.Random(SEED)
    samples = []
    for _ in range(draws):
        total = 0.0
        for values in strata.values():
            total += sum(rng.choice(values) for _ in values)
        samples.append(total / n)
    samples.sort()
    return {"a": a, "b": b, "paired_items": n,
            "mean_difference_a_minus_b": round(estimate, 2),
            "stratified_bootstrap_95pct": [round(samples[int(.025 * draws)], 2),
                                           round(samples[int(.975 * draws) - 1], 2)]}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results", type=Path, default=ROOT / "benchmark_results.csv")
    parser.add_argument("--out", type=Path, default=ROOT / "aggregate_results.json")
    args = parser.parse_args()
    rows = load(args.results)
    groups = defaultdict(list)
    categories = defaultdict(list)
    for row in rows:
        groups[row["configuration"]].append(row)
        categories[(row["configuration"], row["category"])].append(row)
    expected = {"dio_offline", "dio_online", "opengloss_standard", "opengloss_simplified", "hybrid"}
    if set(groups) != expected or any(len(group) != 500 for group in groups.values()):
        raise ValueError("Expected 500 rows for each of the five configurations")
    comparisons = []
    for name in sorted(expected - {"dio_offline"}):
        result = paired_bootstrap(rows, name, "dio_offline")
        if result:
            comparisons.append(result)
    output = {
        "sampling": "fixed purposive quota sample; intervals describe item variability, not population representativeness",
        "configurations": {name: describe(group) for name, group in sorted(groups.items())},
        "by_category": {f"{name}/{category}": describe(group)
                        for (name, category), group in sorted(categories.items())},
        "paired_quality_differences": comparisons,
        "readability_note": "Flesch-Kincaid values use heuristic syllables and are unstable for short glosses.",
    }
    args.out.write_text(json.dumps(output, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"Wrote {args.out}")


if __name__ == "__main__":
    main()
