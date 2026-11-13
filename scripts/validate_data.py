#!/usr/bin/env python3
"""Validate HOMEI data, schemas, privacy rules and repository structure.

Reads local files only and makes no network requests. Requires the `jsonschema` package.
Exit status is non-zero if any check fails. Never weaken a check to make it pass.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

try:
    from jsonschema import Draft202012Validator
except ImportError:  # pragma: no cover
    sys.exit("ERROR: the jsonschema package is required. Install it with: pip install jsonschema")

ROOT = Path(__file__).resolve().parent.parent
SNAPSHOT_DATE = "2026-11-12"  # research snapshot the bundled data reflects

DATA_SCHEMAS = {
    "Data/evidence.json": "Data/schemas/evidence.schema.json",
    "Data/compatibility.json": "Data/schemas/compatibility.schema.json",
    "Data/research-index.json": "Data/schemas/research-index.schema.json",
    "Data/device-fixtures.json": "Data/schemas/device-fixtures.schema.json",
    "Catalog/Packages/catalog.json": "Catalog/Schemas/catalog.schema.json",
    "Catalog/Sources/sources.json": "Catalog/Schemas/sources.schema.json",
    "Bootstrap/Models/fixture-manifest.json": "Bootstrap/Models/manifest.schema.json",
}

REQUIRED_FILES = [
    "README.md", "README_KR.md", "CLAUDE.md", "CHANGELOG.md", "CONTRIBUTING.md", "SECURITY.md",
    "LICENSE", "LICENSE-DOCS", "THIRD_PARTY_NOTICES.md", ".gitignore", "Cargo.toml", "Cargo.lock",
    "Docs/ARCHITECTURE.md", "Docs/DEVICE_COMPATIBILITY.md", "Docs/SAFETY.md", "Docs/RECOVERY.md",
    "Docs/DEVELOPMENT.md", "Docs/ROADMAP.md", "Docs/REFERENCE_PROJECT_AUDIT.md",
    "Docs/PRODUCT_DIFFERENTIATION.md", "Docs/PRODUCT_REQUIREMENTS.md", "Docs/DESIGN_SPEC.md",
    "App/Sources/HomeiApp.swift", "App/project.yml", "App/Resources/HOMEI.entitlements",
    "App/Resources/Assets.xcassets/AppIcon.appiconset/Contents.json", "Tests/golden/parity-vectors.json",
    "Tests/SwiftTests/ParityTests.swift", "Tests/UITests/HomeiUITests.swift", "deny.toml", "scripts/check_contrast.py", "scripts/generate_app_icon.py",
    ".github/workflows/swift-build.yml", "App/Resources/en.lproj/Localizable.strings",
    "App/Resources/ko.lproj/Localizable.strings", "App/Resources/Branding/homei-icon.svg",
    ".github/workflows/validate.yml", ".github/workflows/secret-scan.yml",
]

# Patterns that must never appear in published text.
PRIVACY = {
    "UDID-shaped value": re.compile(r"\b[0-9A-Fa-f]{8}-[0-9A-Fa-f]{16}\b"),
    "40-hex identifier": re.compile(r"\b[0-9a-f]{40}\b"),
    "email address": re.compile(r"\b[\w.+-]+@[\w-]+\.[\w.-]+\b"),
    "personal path": re.compile(r"/home/[a-z0-9_-]+/|/Users/[A-Za-z0-9_-]+/"),
    "private key": re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
    "token": re.compile(r"\b(?:ghp|gho|github_pat|sk)_[A-Za-z0-9_]{16,}"),
    "device-state phrase": re.compile(
        r"owner[- ](?:reported|confirmed)|primary daily[- ]use|daily[- ]use phone|update[- ]blocking|OTA (?:update )?block",
        re.I),
}
# Files that legitimately contain such patterns because they define or test them.
# The test files below contain deliberately identifier-shaped synthetic strings that the code under
# test must reject. They are not real identifiers.
PRIVACY_EXEMPT = {"scripts/validate_data.py", "Tests/UnitTests/device_profile.rs", "Tests/FixtureTests/bundle.rs",
                  "Tests/CompatibilityTests/engine.rs", "Tests/SwiftTests/EngineTests.swift",
                  "Tests/golden/parity-vectors.json", "Cargo.lock"}
SKIP_DIRS = {".git", "target", "node_modules", ".build"}
TEXT_SUFFIXES = {".md", ".json", ".rs", ".toml", ".swift", ".strings", ".svg", ".yml", ".yaml", ".py", ".txt", ".lock", ""}

failures: list[str] = []


def check(name: str, problems: list[str]) -> None:
    print(("PASS  " if not problems else "FAIL  ") + name)
    for p in problems:
        print("      - " + p)
    failures.extend(f"{name}: {p}" for p in problems)


def load(rel: str):
    return json.loads((ROOT / rel).read_text(encoding="utf-8"))


def text_files():
    for p in sorted(ROOT.rglob("*")):
        if p.is_file() and not (set(p.relative_to(ROOT).parts) & SKIP_DIRS) and p.suffix in TEXT_SUFFIXES:
            yield p


def main() -> int:
    check("required files present", [f"missing {f}" for f in REQUIRED_FILES if not (ROOT / f).is_file()])

    data = {}
    problems = []
    for rel, schema_rel in DATA_SCHEMAS.items():
        try:
            data[rel] = load(rel)
            schema = load(schema_rel)
        except (OSError, json.JSONDecodeError) as e:
            problems.append(f"{rel}: {e}")
            continue
        for err in Draft202012Validator(schema).iter_errors(data[rel]):
            problems.append(f"{rel}: {'/'.join(map(str, err.absolute_path)) or '<root>'}: {err.message[:120]}")
    check("JSON Schema compliance", problems)
    if any(rel not in data for rel in DATA_SCHEMAS):
        return finish()

    evidence = {e["id"]: e for e in data["Data/evidence.json"]["evidence"]}
    decls = data["Data/compatibility.json"]["declarations"]
    findings = data["Data/research-index.json"]["findings"]
    fixtures = data["Data/device-fixtures.json"]["fixtures"]
    packages = data["Catalog/Packages/catalog.json"]["packages"]
    sources = {s["id"] for s in data["Catalog/Sources/sources.json"]["sources"]}

    ids = [e["id"] for e in data["Data/evidence.json"]["evidence"]] + [d["id"] for d in decls] + \
          [f["id"] for f in findings] + [f["id"] for f in fixtures]
    check("no duplicate IDs", sorted({i for i in ids if ids.count(i) > 1}))

    refs = [f"{r['id']} cites missing {e}" for r in decls + findings for e in r["evidence_ids"] if e not in evidence]
    check("evidence references resolve", refs)

    dates = [f"{e['id']}: {k} {e[k]} is after the snapshot date" for e in evidence.values()
             for k in ("publication_date", "access_date") if e[k] and e[k] > SNAPSHOT_DATE]
    dates += [f"{f['id']}: last_checked {f['last_checked']} is after the snapshot date" for f in findings
              if f["last_checked"] > SNAPSHOT_DATE]
    check("dates are not after the snapshot date", dates)

    positive = {"supported", "partially_supported"}
    pos = []
    for d in decls:
        if d["status"] in positive:
            classes = {evidence[e]["class"] for e in d["evidence_ids"] if e in evidence}
            if "target_compatible_result" not in classes:
                pos.append(f"{d['id']} is {d['status']} without exact target-result evidence")
    target = [d for d in decls if (d["product_type"], d["product_version"], d["build_version"]) == ("iPhone16,2", "27.0.1", "24A446")]
    if len(target) != 1 or target[0]["status"] != "not_established":
        pos.append("the research target (iPhone16,2 / 27.0.1 / 24A446) must have exactly one record, not_established")
    if any(e["class"] in ("target_compatible_result", "independent_reproduction") for e in evidence.values()):
        pos.append("no bundled evidence may claim a target result or reproduction until one is verified")
    check("compatibility rules (no unsupported positive state)", pos)

    fx = [f"{f['id']}: provenance must be test_fixture" for f in fixtures if f["profile"]["provenance"] != "test_fixture"]
    fx += [f"{f['id']}: device observation fields must stay null" for f in fixtures
           if f["profile"]["hardware_model"] or f["profile"]["device_class"]]
    check("fixtures are synthetic test records", fx)

    pk = [f"{p['id']}: unknown source {p['source_id']}" for p in packages if p["source_id"] not in sources]
    pk += [f"{p['id']}: not marked fixture" for p in packages if not p["fixture"]]
    pk += [f"{p['id']}: integrity must not be marked verified for a placeholder" for p in packages if p["integrity"]["verified"]]
    check("catalog is metadata-only test fixtures", pk)

    priv = []
    for p in text_files():
        rel = p.relative_to(ROOT).as_posix()
        if rel in PRIVACY_EXEMPT:
            continue
        body = p.read_text(encoding="utf-8", errors="replace")
        for name, rx in PRIVACY.items():
            for m in rx.finditer(body):
                if name == "email address" and m.group(0).endswith("@users.noreply.github.com"):
                    continue
                line = body.splitlines()[body.count(chr(10), 0, m.start())]
                # A workflow pins each action to a full commit SHA, which is a 40-hex value but not an identifier.
                if name == "40-hex identifier" and rel.startswith(".github/workflows/") and "uses:" in line:
                    continue
                priv.append(f"{rel}:{body.count(chr(10), 0, m.start()) + 1}: {name}")
    check("privacy and secret patterns absent", priv)

    icons = ROOT / "App/Resources/Assets.xcassets/AppIcon.appiconset"
    ic = []
    try:
        catalog = json.loads((icons / "Contents.json").read_text(encoding="utf-8"))
        for img in catalog["images"]:
            png = icons / img["filename"]
            if not png.is_file():
                ic.append(f"missing {img['filename']}")
                continue
            head = png.read_bytes()[:26]
            w, h = int.from_bytes(head[16:20], "big"), int.from_bytes(head[20:24], "big")
            want = round(float(img["size"].split("x")[0]) * int(img["scale"].rstrip("x")))
            if (w, h) != (want, want):
                ic.append(f"{img['filename']} is {w}x{h}, expected {want}x{want}")
            if head[25] in (4, 6):
                ic.append(f"{img['filename']} has an alpha channel (iOS icons must be opaque)")
    except (OSError, KeyError, ValueError) as e:
        ic.append(str(e))
    check("app icon set is complete, correctly sized and opaque", ic)

    en = [l for l in (ROOT / "README.md").read_text(encoding="utf-8").splitlines() if l.startswith("## ")]
    kr = [l for l in (ROOT / "README_KR.md").read_text(encoding="utf-8").splitlines() if l.startswith("## ")]
    check("README and README_KR have the same number of sections",
          [] if len(en) == len(kr) else [f"README has {len(en)} sections, README_KR has {len(kr)}"])

    def keys(path):
        return set(re.findall(r'^"([^"]+)"\s*=', (ROOT / path).read_text(encoding="utf-8"), re.M))
    en_k, ko_k = keys("App/Resources/en.lproj/Localizable.strings"), keys("App/Resources/ko.lproj/Localizable.strings")
    check("English and Korean string keys match",
          [f"missing in ko: {k}" for k in sorted(en_k - ko_k)] + [f"missing in en: {k}" for k in sorted(ko_k - en_k)])

    return finish()


def finish() -> int:
    total = "all checks passed" if not failures else f"{len(failures)} problem(s)"
    print(f"\n{total}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
