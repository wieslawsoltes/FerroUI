#!/usr/bin/env python3
"""Validates the inventory of an upstream sync and writes its generated tables.

    python3 -I scripts/port-status/upstream_sync.py           # rewrite the generated sections
    python3 -I scripts/port-status/upstream_sync.py --check   # exit 1 when they are out of date

The inventory is `docs/porting/data/upstream-sync-12.2.0.json`: one entry per upstream commit.
The prose of `docs/porting/upstream-sync-12.2.0.md` is edited by hand; the sections between
`<!-- generated:NAME -->` and `<!-- /generated:NAME -->` come from this script.
"""
import collections
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
DATA = ROOT / "docs/porting/data/upstream-sync-12.2.0.json"
DOC = ROOT / "docs/porting/upstream-sync-12.2.0.md"
EXPECTED_COMMITS = 182
CLASSES = ("port", "already", "skip-performance", "skip-better", "not-applicable", "deferred")
STATES = ("planned", "done")
# Built from parts so that this file passes the check it implements.
FORBIDDEN = re.compile("|".join(("ava" + "lonia", r"\ba" + "vn", r"(?<![\w`])@[A-Za-z0-9-]+")), re.I)


def validate(data):
    errors = []
    commits = data["commits"]
    if len(commits) != EXPECTED_COMMITS:
        errors.append(f"expected {EXPECTED_COMMITS} commits, found {len(commits)}")
    seen = set()
    for entry in commits:
        name = entry.get("hash", "?")[:10]
        if len(entry.get("hash", "")) != 40:
            errors.append(f"{name}: not a full hash")
        if entry["hash"] in seen:
            errors.append(f"{name}: listed twice")
        seen.add(entry["hash"])
        if entry.get("class") not in CLASSES:
            errors.append(f"{name}: unknown class {entry.get('class')!r}")
        if not entry.get("note", "").strip():
            errors.append(f"{name}: no note")
        if not entry.get("area"):
            errors.append(f"{name}: no area")
        if entry.get("class") == "port":
            if entry.get("state") not in STATES:
                errors.append(f"{name}: a port needs a state of {STATES}")
            if not entry.get("part"):
                errors.append(f"{name}: a port needs a part")
        for field in ("note", "area", "part"):
            if FORBIDDEN.search(str(entry.get(field, ""))):
                errors.append(f"{name}: forbidden name or handle in {field}")
    return errors


def summary(commits):
    counts = collections.Counter(e["class"] for e in commits)
    done = sum(1 for e in commits if e.get("state") == "done")
    lines = ["| Class | Commits |", "|---|---:|"]
    for name in CLASSES:
        if name == "port":
            lines.append(f"| `port` ({done} done, {counts[name] - done} planned) | {counts[name]} |")
        else:
            lines.append(f"| `{name}` | {counts[name]} |")
    lines.append(f"| **Total** | **{len(commits)}** |")
    return "\n".join(lines)


def by_area(commits):
    areas = collections.defaultdict(collections.Counter)
    for entry in commits:
        areas[entry["area"]][entry["class"]] += 1
    lines = ["| Area | " + " | ".join(f"`{c}`" for c in CLASSES) + " | Total |",
             "|---|" + "---:|" * (len(CLASSES) + 1)]
    for area in sorted(areas):
        cells = " | ".join(str(areas[area][c]) if areas[area][c] else "" for c in CLASSES)
        lines.append(f"| {area} | {cells} | {sum(areas[area].values())} |")
    return "\n".join(lines)


def table(entries, with_class=True):
    head = "| Commit | PR | Date | Area | " + ("Class | " if with_class else "") + "Part | Note |"
    rule = "|---|---:|---|---|" + ("---|" if with_class else "") + "---|---|"
    lines = [head, rule]
    for e in entries:
        pr = f"#{e['pr']}" if e.get("pr") else ""
        cls = f"`{e['class']}`{' (done)' if e.get('state') == 'done' else ''} | " if with_class else ""
        note = e["note"].replace("|", "\\|")
        lines.append(f"| `{e['hash'][:10]}` | {pr} | {e['date']} | {e['area']} | {cls}{e.get('part', '')} | {note} |")
    return "\n".join(lines)


def render(data):
    commits = data["commits"]
    return {
        "summary": summary(commits),
        "areas": by_area(commits),
        "deferred": table([e for e in commits if e["class"] == "deferred"], with_class=False),
        "inventory": table(commits),
    }


def apply(text, sections):
    for name, body in sections.items():
        pattern = re.compile(rf"(<!-- generated:{name} -->\n).*?(<!-- /generated:{name} -->)", re.S)
        if not pattern.search(text):
            raise SystemExit(f"error: section {name} is missing from {DOC.name}")
        text = pattern.sub(lambda m: m.group(1) + body + "\n" + m.group(2), text)
    return text


def main():
    data = json.loads(DATA.read_text(encoding="utf-8"))
    errors = validate(data)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    current = DOC.read_text(encoding="utf-8")
    updated = apply(current, render(data))
    if "--check" in sys.argv[1:]:
        if updated != current:
            print(f"{DOC.name} is out of date: run scripts/port-status/upstream_sync.py", file=sys.stderr)
            return 1
        return 0
    DOC.write_text(updated, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
