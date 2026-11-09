#!/usr/bin/env python3
"""Evaluate all four predeclared fixed-size goals; retain every miss."""

import argparse
import csv
import json
import math
from pathlib import Path
import statistics


def evaluate(rows):
    cases = {}
    for row in rows:
        size, operation = int(row["bytes"]), row["operation"]
        if row["pattern"] != "random" or size not in (32, 64):
            continue
        if operation not in ("encode", "decode"):
            raise ValueError(f"Unknown operation: {operation}")
        value = float(row["ns_per_op"])
        if not math.isfinite(value) or value <= 0 or int(row["iterations"]) <= 0:
            raise ValueError("All measurements must be finite and positive")
        bucket = cases.setdefault((size, operation), {}).setdefault(row["algorithm"], {})
        sample = int(row["sample"])
        if sample in bucket:
            raise ValueError("Duplicate timing sample")
        bucket[sample] = value
    result = []
    for size in (32, 64):
        for operation in ("encode", "decode"):
            data = cases.get((size, operation), {})
            mandatory = {"z58-fixed", "five8", "braid58"}
            if not mandatory.issubset(data):
                raise ValueError(f"Missing strong baseline or implementation for {size}/{operation}")
            candidates = [name for name in ("five8", "braid58", "fd-bs58") if name in data]
            for name in ["z58-fixed", *candidates]:
                if len(data[name]) < 9:
                    raise ValueError(f"Strict acceptance needs at least 9 samples for {size}/{operation}/{name}")
            times = {name: statistics.median(data[name].values()) for name in ["z58-fixed", *candidates]}
            fastest = min(candidates, key=times.__getitem__)
            ratio = times[fastest] / times["z58-fixed"]
            result.append({
                "bytes": size,
                "operation": operation,
                "pattern": "random",
                "z58_ns": times["z58-fixed"],
                "strongest_baseline": fastest,
                "baseline_ns": times[fastest],
                "speedup": ratio,
                "target": 2.0,
                "passed": ratio >= 2.0,
                "all_medians_ns": times,
            })
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    with args.csv.open(newline="") as handle:
        results = evaluate(csv.DictReader(handle))
    report = {"target_met": all(row["passed"] for row in results), "cases": results}
    text = json.dumps(report, indent=2, allow_nan=False)
    print(text)
    if args.output:
        args.output.write_text(text + "\n")
    if not report["target_met"]:
        raise SystemExit("The 2x target was not met in every predeclared case; raw results remain available.")


if __name__ == "__main__":
    main()
