#!/usr/bin/env python3
"""Check WCAG 2.x text contrast for the HOMEI colour tokens.

Reads the tokens from App/Sources/Theme.swift so there is one source of truth, then checks the
foreground and background pairs the views actually draw, including tinted badge and banner
backgrounds (a colour at some opacity over a surface). Normal text needs 4.5:1.

Limits: this checks colour pairs only. It does not measure rendered pixels, and it does not test
Increase Contrast or other accessibility settings.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MIN_RATIO = 4.5


def tokens() -> dict[str, tuple[float, float, float]]:
    text = (ROOT / "App/Sources/Theme.swift").read_text(encoding="utf-8")
    found = {}
    for name, r, g, b in re.findall(
        r"static let (\w+) = Color\(red: ([\d.]+), green: ([\d.]+), blue: ([\d.]+)\)", text
    ):
        found[name] = (float(r), float(g), float(b))
    return found


def blend(fg, bg, alpha):
    return tuple(alpha * f + (1 - alpha) * b for f, b in zip(fg, bg))


def luminance(c):
    def lin(v):
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = (lin(v) for v in c)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = sorted((luminance(a), luminance(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def main() -> int:
    t = tokens()
    need = ["background", "surface", "surfaceRaised", "accent", "textPrimary", "textSecondary",
            "positive", "caution", "negative", "neutral"]
    missing = [n for n in need if n not in t]
    if missing:
        print("FAIL missing tokens:", ", ".join(missing))
        return 1
    bg, surface, raised = t["background"], t["surface"], t["surfaceRaised"]
    pairs = []
    for fg in ("textPrimary", "textSecondary", "accent", "positive", "caution", "negative", "neutral"):
        for name, back in (("background", bg), ("surface", surface), ("surfaceRaised", raised)):
            pairs.append((f"{fg} on {name}", t[fg], back))
    # Status badges: the status colour at 16% over the card surface.
    for status in ("positive", "caution", "negative", "neutral"):
        pairs.append((f"{status} badge text on its 16% tint over surface", t[status], blend(t[status], surface, 0.16)))
    # Provenance and data-kind banners: accent at 12% over the screen background and over cards.
    for name, back in (("background", bg), ("surface", surface)):
        pairs.append((f"accent banner text on its 12% tint over {name}", t["accent"], blend(t["accent"], back, 0.12)))
    failed = 0
    for label, fg, back in pairs:
        r = ratio(fg, back)
        ok = r >= MIN_RATIO
        failed += not ok
        print(f"{'PASS' if ok else 'FAIL'}  {r:5.2f}:1  {label}")
    print(f"\n{len(pairs) - failed}/{len(pairs)} pairs meet {MIN_RATIO}:1")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
