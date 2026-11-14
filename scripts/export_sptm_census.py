#!/usr/bin/env python3
"""Export a per-register summary of the static SPTM register census.

Input: the census CSV and the fork-coverage JSON from the research workspace (passed as paths).
Output: a sanitized JSON file with one record per implementation-defined register. No local path,
no disassembly text and no device data is written. Reads and writes only the named files.

Usage: export_sptm_census.py CENSUS.csv COVERAGE.json OUT.json
"""
import csv
import json
import sys
from collections import Counter, defaultdict


def main(csv_path: str, coverage_path: str, out_path: str) -> int:
    rows = list(csv.DictReader(open(csv_path, encoding="utf-8")))
    coverage = json.load(open(coverage_path, encoding="utf-8"))
    defined = set(coverage["defined_in_fork"])

    by_reg = defaultdict(list)
    for r in rows:
        by_reg[r["register_encoding"]].append(r)

    registers = []
    for name in sorted(by_reg):
        rs = by_reg[name]
        access = Counter(r["access_classification"] for r in rs)
        graph = Counter(r["call_graph_class"] for r in rs)
        registers.append({
            "encoding": name,
            "defined_in_fork": name in defined,
            "accesses": len(rs),
            "mrs": sum(1 for r in rs if r["instruction_type"] == "MRS"),
            "msr": sum(1 for r in rs if r["instruction_type"] == "MSR"),
            "read_modify_write_candidates": access.get("read-modify-write candidate", 0),
            "reachable_from_early_exception_vectors": sum(
                1 for r in rs if r["reachable_from_early_exception_vectors"] == "True"),
            "call_graph_classes": sorted(graph),
            "lifetime_classes": sorted({r["lifetime_classification"] for r in rs}),
        })

    undefined = [r for r in registers if not r["defined_in_fork"]]
    out = {
        "schema_version": 1,
        "source_description": "Static census of the SPTM image (no execution). Access classes are candidates; lifetime classes are low confidence where runtime dispatch is unresolved.",
        "access_rows": len(rows),
        "register_count": len(registers),
        "defined_in_fork_count": len(registers) - len(undefined),
        "undefined_in_fork_count": len(undefined),
        "runtime_reachable_register_count": sum(
            1 for r in registers if "runtime-root-reachable" in r["call_graph_classes"]),
        "registers": registers,
    }
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2)
        f.write("\n")
    print(f"registers={len(registers)} undefined_in_fork={len(undefined)} rows={len(rows)}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    sys.exit(main(*sys.argv[1:]))
