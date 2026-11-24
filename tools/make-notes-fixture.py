# SPDX-License-Identifier: MIT OR Apache-2.0
# Writes tests/fixtures/notes-folder/: 200 Markdown notes written for the test (an invented
# coastal town), with front matter, aliases and [[links]], three of which name no note.
# Deterministic: running it again gives the same files.

import os
import random

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tests", "fixtures", "notes-folder")
rng = random.Random(2026)
SYL = ["ar", "bel", "cor", "dun", "el", "fen", "gar", "hal", "is", "jor", "kel", "lin", "mor", "nes", "or", "pel", "quin", "ros", "sel", "tor", "ul", "ven", "wyn", "yor"]
WORDS = "the tide turned slowly while gulls circled over the harbor wall and lamps burned in narrow windows above the market".split()


def name():
    return "".join(rng.choice(SYL) for _ in range(rng.randint(2, 3))).capitalize()


def sentence():
    return " ".join(rng.choice(WORDS) for _ in range(rng.randint(6, 14))).capitalize() + "."


notes = []
used = set()
for folder, count, kind in [("Characters", 120, "character"), ("Places", 50, "place"), ("Lore", 30, "lore")]:
    for _ in range(count):
        n = name()
        while n in used:
            n = name()
        used.add(n)
        notes.append((folder, n, kind))
names = [n for _, n, _ in notes]
os.makedirs(ROOT, exist_ok=True)
for i, (folder, n, kind) in enumerate(notes):
    os.makedirs(os.path.join(ROOT, folder), exist_ok=True)
    lines = ["---"]
    if i % 3 == 0:
        lines.append(f"aliases: [{n[:3]}{i}]")
    if kind == "character":
        lines.append(f"age: {20 + i % 50}")
    lines.append("---")
    a, b = rng.choice(names), rng.choice(names)
    body = [
        f"{sentence()} {sentence()}",
        f"Often seen with [[{a}]], and wary of [[{b}|the {b.lower()} family]].",
        sentence(),
        "- " + sentence(),
    ]
    if i in (7, 77, 177):
        body.append(f"Once mentioned [[Nobody Called {i}]] in passing.")
    with open(os.path.join(ROOT, folder, f"{n}.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n\n" + "\n\n".join(body) + "\n")
