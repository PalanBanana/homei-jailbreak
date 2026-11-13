#!/usr/bin/env python3
"""Generate AppIcon.appiconset from the original SVG, offline and reproducibly.

Source of truth: App/Resources/Branding/homei-icon.svg (original HOMEI artwork, kept unchanged).
Tools: rsvg-convert (librsvg) and ImageMagick. No network, no third-party artwork.

iOS applies its own rounded mask, so the icon must be a full-bleed opaque square. The source SVG has
rounded, transparent corners, so this script renders a square variant (rx=0) and flattens any alpha
onto the icon's own background colour. Output PNGs are stripped of metadata so reruns are byte-identical.
"""
from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SVG = ROOT / "App/Resources/Branding/homei-icon.svg"
OUT = ROOT / "App/Resources/Assets.xcassets/AppIcon.appiconset"
BACKGROUND = "#0b1730"  # first stop of the icon gradient, used only to flatten alpha

# (idiom, point size, scale). Pixel size = point size * scale.
SLOTS = [
    ("iphone", 20, 2), ("iphone", 20, 3), ("iphone", 29, 2), ("iphone", 29, 3),
    ("iphone", 40, 2), ("iphone", 40, 3), ("iphone", 60, 2), ("iphone", 60, 3),
    ("ipad", 20, 1), ("ipad", 20, 2), ("ipad", 29, 1), ("ipad", 29, 2),
    ("ipad", 40, 1), ("ipad", 40, 2), ("ipad", 76, 1), ("ipad", 76, 2), ("ipad", 83.5, 2),
    ("ios-marketing", 1024, 1),
]


def pixels(size: float, scale: int) -> int:
    px = size * scale
    assert px == int(px), (size, scale)
    return int(px)


def need(tool: str) -> str:
    path = shutil.which(tool) or (shutil.which("convert") if tool == "magick" else None)
    if not path:
        sys.exit(f"ERROR: {tool} is required")
    return path


def main() -> int:
    rsvg, magick = need("rsvg-convert"), need("magick")
    source = SVG.read_text(encoding="utf-8")
    square, n = re.subn(r'(<rect width="1024" height="1024") rx="\d+"', r'\1 rx="0"', source)
    if n != 1:
        sys.exit("ERROR: expected exactly one rounded background rect in the source SVG")
    OUT.mkdir(parents=True, exist_ok=True)
    for old in OUT.glob("*.png"):
        old.unlink()
    with tempfile.TemporaryDirectory() as tmp:
        square_svg = Path(tmp) / "square.svg"
        square_svg.write_text(square, encoding="utf-8")
        files: dict[int, str] = {}
        for _, size, scale in SLOTS:
            px = pixels(size, scale)
            if px in files:
                continue
            name = f"icon-{px}.png"
            raw = Path(tmp) / f"raw-{px}.png"
            subprocess.run([rsvg, "-w", str(px), "-h", str(px), str(square_svg), "-o", str(raw)], check=True)
            subprocess.run([magick, str(raw), "-background", BACKGROUND, "-alpha", "remove", "-alpha", "off",
                            "-strip", f"PNG24:{OUT / name}"], check=True)
            files[px] = name
    images = []
    for idiom, size, scale in SLOTS:
        label = f"{size:g}x{size:g}"
        images.append({"filename": files[pixels(size, scale)], "idiom": idiom, "scale": f"{scale}x", "size": label})
    (OUT / "Contents.json").write_text(
        json.dumps({"images": images, "info": {"author": "xcode", "version": 1}}, indent=2) + "\n", encoding="utf-8")
    (OUT.parent / "Contents.json").write_text(
        json.dumps({"info": {"author": "xcode", "version": 1}}, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {len(files)} PNG files and {len(images)} catalog entries to {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
