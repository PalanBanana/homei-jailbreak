#!/usr/bin/env python3
"""Summarise the UI contrast and accessibility audit records exported from an .xcresult bundle.

Input: the directory written by `xcresulttool export attachments` (it contains manifest.json and the
attachment files). The UI tests attach one `audit-<screen>.json` per audited screen.

Output: counts per category (PASS, FAIL, OCCLUDED, CLIPPED, INCONCLUSIVE) per screen and in total, the
screen and page of every non-PASS element, and Apple's own audit findings. Exit status is 1 when any
element FAILS or is CLIPPED, or when no audit records were found at all. OCCLUDED and INCONCLUSIVE
results are printed and never counted as passes, but they do not by themselves fail the step: they are
limits to be read, not defects.

Reads local files only.
"""
from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path

CATEGORIES = ["PASS", "FAIL", "OCCLUDED", "CLIPPED", "INCONCLUSIVE"]


def load_records(root: Path) -> list[dict]:
    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    records = []
    for test in manifest:
        for att in test.get("attachments", []):
            name = att.get("suggestedHumanReadableName", "")
            if name.startswith("audit-") and ".json" in name:
                body = json.loads((root / att["exportedFileName"]).read_text(encoding="utf-8"))
                body["test"] = test.get("testIdentifier", "")
                records.append(body)
    return records


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: summarize_ui_audit.py <exported-attachments-dir>")
        return 2
    try:
        records = load_records(Path(argv[1]))
    except (OSError, json.JSONDecodeError, KeyError) as e:
        print(f"ERROR: cannot read audit records: {e}")
        return 1
    if not records:
        print("ERROR: no audit-*.json records were found, so nothing was verified")
        return 1

    total = Counter()
    system = Counter()
    raw_occlusions = 0
    problems = []
    limits = []
    overruled = []
    lines = [f"{'screen':<22}{'pages':>6}" + "".join(f"{c:>14}" for c in CATEGORIES)]
    for rec in sorted(records, key=lambda r: r["screen"]):
        app_obs = [o for o in rec["final"] if not o["label"].startswith("[system bar]")]
        system.update(o["classification"] for o in rec["final"] if o["label"].startswith("[system bar]"))
        counts = Counter(o["classification"] for o in app_obs)
        total.update(counts)
        raw_occlusions += sum(1 for o in rec["raw"] if o["classification"] == "OCCLUDED")
        lines.append(f"{rec['screen']:<22}{rec['pages']:>6}" + "".join(f"{counts[c]:>14}" for c in CATEGORIES))
        for o in app_obs:
            where = f"{rec['screen']} page {o['page']}: [{o['label']}] {o['detail']}"
            if o["classification"] in ("FAIL", "CLIPPED"):
                problems.append(f"{o['classification']} {where}")
            elif o["classification"] in ("OCCLUDED", "INCONCLUSIVE"):
                limits.append(f"{o['classification']} {where}")
        for line in rec.get("appleOverruled", []):
            overruled.append(f"APPLE-OVERRULED {rec['screen']}: {line}")
        for key, label in (("appleReported", "APPLE-AUDIT"), ("appleUnattributed", "APPLE-UNATTRIBUTED"),
                           ("appleInconclusive", "APPLE-INCONCLUSIVE")):
            for line in rec.get(key, []):
                (problems if label == "APPLE-AUDIT" else limits).append(f"{label} {rec['screen']}: {line}")
        if not rec.get("reachedEnd", False):
            problems.append(f"{rec['screen']}: did not reach the end of the screen")
        if rec.get("stoppedEarly", False):
            problems.append(f"{rec['screen']}: scrolling stopped while text still lies under the tab bar (screen not fully audited)")
    lines.append(f"{'TOTAL':<22}{'':>6}" + "".join(f"{total[c]:>14}" for c in CATEGORIES))

    out = ["UI contrast and accessibility audit (the application's own text)", *lines, "",
           "System-drawn bar text (tab bar and navigation bar), reported apart because it is drawn by the system over a"
           " translucent glass gradient and a pixel reading there is not reliable: "
           + ", ".join(f"{c} {system[c]}" for c in CATEGORIES) + ".",
           f"Raw occlusion events across pages (elements seen inside a bar or scroll-edge zone): {raw_occlusions}",
           f"Problems (fail the step): {len(problems)}", *[f"  {p}" for p in problems],
           f"Limits (not passes, not failures): {len(limits)}", *[f"  {p}" for p in limits],
           f"Apple findings overruled by pixel evidence (recorded, not counted as defects): {len(overruled)}",
           *[f"  {p}" for p in overruled]]
    text = "\n".join(out)
    print(text)
    summary = Path(__import__("os").environ.get("GITHUB_STEP_SUMMARY", ""))
    if summary.name:
        with summary.open("a", encoding="utf-8") as fh:
            fh.write("```\n" + text + "\n```\n")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
