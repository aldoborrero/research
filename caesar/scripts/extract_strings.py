#!/usr/bin/env python3
"""
Caesar 3 binary string extractor and classifier.

Usage: python3 extract_strings.py <path_to_caesar3.exe>

Extracts ASCII strings from the binary and clusters them by prefix/category.
"""

import sys
import re
from collections import defaultdict


def extract_strings(filepath: str, min_length: int = 4) -> list[tuple[int, str]]:
    """Extract printable ASCII strings from a binary file."""
    with open(filepath, "rb") as f:
        data = f.read()

    strings = []
    current = []
    start = 0

    for i, byte in enumerate(data):
        if 0x20 <= byte <= 0x7E:  # printable ASCII
            if not current:
                start = i
            current.append(chr(byte))
        else:
            if len(current) >= min_length:
                strings.append((start, "".join(current)))
            current = []

    if len(current) >= min_length:
        strings.append((start, "".join(current)))

    return strings


def classify_string(s: str) -> str:
    """Classify a string into a category based on content patterns."""
    s_lower = s.lower()

    categories = {
        "file_path": [".555", ".sg2", ".sg3", ".map", ".sav", ".dat",
                      ".bmp", ".wav", ".smk", "\\"],
        "ui_text": ["click", "button", "menu", "dialog", "panel",
                     "scroll", "window"],
        "building": ["house", "farm", "temple", "senate", "forum",
                      "market", "granary", "warehouse", "theater",
                      "amphitheater", "colosseum", "hippodrome",
                      "bath", "barber", "school", "library", "academy",
                      "hospital", "fountain", "well", "aqueduct",
                      "reservoir", "garden", "plaza", "statue",
                      "prefecture", "fort", "tower", "wall", "gate"],
        "walker": ["walker", "figure", "citizen", "immigrant", "emigrant",
                   "soldier", "gladiator", "priest", "doctor", "teacher",
                   "engineer", "prefect", "trader", "merchant",
                   "cart", "donkey"],
        "resource": ["wheat", "vegetable", "fruit", "olive", "vine",
                     "meat", "wine", "oil", "iron", "timber",
                     "clay", "marble", "pottery", "furniture",
                     "weapon"],
        "god": ["ceres", "neptune", "mercury", "mars", "venus"],
        "rating": ["culture", "prosperity", "peace", "favor",
                   "population", "rating"],
        "msvc_runtime": ["runtime error", "assertion", "abort",
                         "microsoft", "visual c", "crt", "_except",
                         "__cdecl"],
        "directx": ["direct", "ddraw", "dsound", "dinput",
                    "directdraw", "directsound"],
        "debug": ["debug", "error", "warning", "assert", "log",
                  "trace", "dump"],
    }

    for category, keywords in categories.items():
        for kw in keywords:
            if kw in s_lower:
                return category

    if re.match(r'^[A-Z_]+$', s) and len(s) > 3:
        return "constant"

    return "other"


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <path_to_binary>")
        sys.exit(1)

    filepath = sys.argv[1]
    print(f"[*] Extracting strings from: {filepath}")

    strings = extract_strings(filepath)
    print(f"[*] Found {len(strings)} strings (min length 4)")

    # Classify
    categories = defaultdict(list)
    for offset, s in strings:
        cat = classify_string(s)
        categories[cat].append((offset, s))

    # Report
    print(f"\n{'='*60}")
    print(f"String Classification Report")
    print(f"{'='*60}\n")

    for cat in sorted(categories.keys()):
        items = categories[cat]
        print(f"\n## {cat.upper()} ({len(items)} strings)")
        print(f"{'-'*40}")
        for offset, s in items[:20]:  # show first 20
            print(f"  0x{offset:08X}: {s[:80]}")
        if len(items) > 20:
            print(f"  ... and {len(items)-20} more")


if __name__ == "__main__":
    main()
