"""Measure SQLite storage and lookup costs for DIO dictionary candidates."""

import argparse
import csv
import ctypes
import json
import math
import os
import random
import sqlite3
import subprocess
import sys
import time
from pathlib import Path


def working_set_bytes():
    if sys.platform != "win32":
        return None

    from ctypes import wintypes

    class ProcessMemoryCounters(ctypes.Structure):
        _fields_ = [
            ("cb", wintypes.DWORD),
            ("PageFaultCount", wintypes.DWORD),
            ("PeakWorkingSetSize", ctypes.c_size_t),
            ("WorkingSetSize", ctypes.c_size_t),
            ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
            ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
            ("PagefileUsage", ctypes.c_size_t),
            ("PeakPagefileUsage", ctypes.c_size_t),
        ]

    counters = ProcessMemoryCounters()
    counters.cb = ctypes.sizeof(counters)
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    psapi = ctypes.WinDLL("psapi", use_last_error=True)
    kernel32.GetCurrentProcess.restype = ctypes.c_void_p
    psapi.GetProcessMemoryInfo.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ProcessMemoryCounters),
        wintypes.DWORD,
    ]
    if not psapi.GetProcessMemoryInfo(
        kernel32.GetCurrentProcess(), ctypes.byref(counters), counters.cb
    ):
        raise OSError(ctypes.get_last_error(), "GetProcessMemoryInfo failed")
    return counters.WorkingSetSize


def open_database(path):
    return sqlite3.connect(f"file:{path.as_posix()}?mode=ro", uri=True)


def query(connection, kind, term, level):
    if kind == "dio":
        return connection.execute(
            "SELECT pos, definition, example, synset_id FROM definitions WHERE word=?",
            (term.casefold(),),
        ).fetchall()
    return connection.execute(
        "SELECT pos, definition, example, sense_id FROM definitions "
        "WHERE word=? AND reading_level=? ORDER BY sense_index",
        (term.casefold(), level),
    ).fetchall()


def summary(values, unit="ms"):
    if not values:
        return None
    values = sorted(values)
    percentile = lambda p: values[max(0, math.ceil(p * len(values)) - 1)]
    return {
        "n": len(values),
        f"p50_{unit}": percentile(0.50),
        f"p95_{unit}": percentile(0.95),
        f"p99_{unit}": percentile(0.99),
        f"min_{unit}": values[0],
        f"max_{unit}": values[-1],
    }


def storage_stats(connection, path):
    page_size = connection.execute("PRAGMA page_size").fetchone()[0]
    page_count = connection.execute("PRAGMA page_count").fetchone()[0]
    stats = {
        "database_bytes": path.stat().st_size,
        "page_size": page_size,
        "page_count": page_count,
        "allocated_bytes": page_size * page_count,
        "objects": {},
    }
    try:
        stats["objects"] = dict(
            connection.execute("SELECT name, SUM(pgsize) FROM dbstat GROUP BY name")
        )
        stats["index_bytes"] = sum(
            size for name, size in stats["objects"].items()
            if connection.execute(
                "SELECT type FROM sqlite_master WHERE name=?", (name,)
            ).fetchone() == ("index",)
        )
    except sqlite3.OperationalError:
        stats["dbstat_unavailable"] = True
    return stats


def probe(args):
    before = working_set_bytes()
    start = time.perf_counter_ns()
    connection = open_database(args.database)
    opened = time.perf_counter_ns()
    after_open = working_set_bytes()
    rows = query(connection, args.kind, args.term, args.level)
    queried = time.perf_counter_ns()
    after_query = working_set_bytes()
    connection.close()
    print(json.dumps({
        "open_ms": (opened - start) / 1e6,
        "lookup_ms": (queried - opened) / 1e6,
        "row_count": len(rows),
        "working_set_before": before,
        "working_set_open": after_open,
        "working_set_lookup": after_query,
    }))


def benchmark(args):
    with args.corpus.open(newline="", encoding="utf-8") as file:
        terms = [row["term"] for row in csv.DictReader(file)]
    if not terms:
        raise ValueError("Corpus has no terms")
    connection = open_database(args.database)
    before = working_set_bytes()
    storage = storage_stats(connection, args.database)
    after_open = working_set_bytes()
    randomizer = random.Random(args.seed)
    warm_times = []
    warm_found = 0
    for _ in range(args.warm_rounds):
        order = terms.copy()
        randomizer.shuffle(order)
        for term in order:
            start = time.perf_counter_ns()
            rows = query(connection, args.kind, term, args.level)
            warm_times.append((time.perf_counter_ns() - start) / 1e6)
            warm_found += bool(rows)
    after_warm = working_set_bytes()
    connection.close()

    cold = []
    order = terms.copy()
    randomizer.shuffle(order)
    for term in order[:args.cold_count]:
        command = [
            sys.executable, str(Path(__file__).resolve()), "probe",
            "--database", str(args.database), "--kind", args.kind,
            "--term", term, "--level", args.level,
        ]
        result = subprocess.run(command, capture_output=True, text=True, check=True)
        cold.append(json.loads(result.stdout))

    results = {
        "database": str(args.database.resolve()),
        "kind": args.kind,
        "reading_level": args.level if args.kind == "opengloss" else None,
        "corpus_size": len(terms),
        "warm_rounds": args.warm_rounds,
        "cold_count": len(cold),
        "seed": args.seed,
        "cold_definition": "first query in a new process and SQLite connection; OS file cache not cleared",
        "storage": storage,
        "warm_lookup": summary(warm_times),
        "warm_found_queries": warm_found,
        "fresh_process_connection_open": summary([x["open_ms"] for x in cold]),
        "fresh_process_lookup": summary([x["lookup_ms"] for x in cold]),
        "fresh_process_found_queries": sum(x["row_count"] > 0 for x in cold),
        "working_set_before_open_bytes": before,
        "working_set_after_open_bytes": after_open,
        "working_set_after_warm_bytes": after_warm,
        "fresh_process_working_set_delta_bytes": summary([
            x["working_set_lookup"] - x["working_set_before"]
            for x in cold if x["working_set_before"] is not None
        ], "bytes"),
        "python_version": sys.version,
        "sqlite_version": sqlite3.sqlite_version,
        "platform": sys.platform,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "output": str(args.output),
        "database_bytes": storage["database_bytes"],
        "warm_p95_ms": results["warm_lookup"]["p95_ms"],
        "fresh_p95_ms": results["fresh_process_lookup"]["p95_ms"],
    }))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    measure = subcommands.add_parser("measure")
    measure.add_argument("--database", type=Path, required=True)
    measure.add_argument("--kind", choices=("dio", "opengloss"), required=True)
    measure.add_argument("--corpus", type=Path, required=True)
    measure.add_argument("--output", type=Path, required=True)
    measure.add_argument("--level", default="neutral")
    measure.add_argument("--warm-rounds", type=int, default=20)
    measure.add_argument("--cold-count", type=int, default=500)
    measure.add_argument("--seed", type=int, default=20261125)
    single = subcommands.add_parser("probe")
    single.add_argument("--database", type=Path, required=True)
    single.add_argument("--kind", choices=("dio", "opengloss"), required=True)
    single.add_argument("--term", required=True)
    single.add_argument("--level", default="neutral")
    args = parser.parse_args()
    if args.command == "probe":
        probe(args)
    else:
        benchmark(args)


if __name__ == "__main__":
    main()
