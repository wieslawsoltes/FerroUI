#!/usr/bin/env python3
"""Regenerates the settings reference in docs/configuration.md from KeyboardSettings.cs.

Usage: python3 build/scripts/generate_settings_docs.py
The tables between '## Reference' and '## Enumerations' are replaced.
"""
import re, pathlib

root = pathlib.Path(__file__).resolve().parents[2]
src = (root / "src/ScreenKeyboard.Core/Settings/KeyboardSettings.cs").read_text()
sections = re.split(r'// -{10,} (\w[\w &]*)\n', src)
out = []
strip = ["SwipeAction.", "KeyHintMode.", "UtilityKeyAction.", "SpaceBarMode.", "SmartbarLayout.", "ThemeMode.", "EmojiSkinTone."]
for i in range(1, len(sections), 2):
    name, body = sections[i].strip(), sections[i + 1]
    if name == "serialization":
        continue
    rows = []
    for m in re.finditer(r'private (\S+) _(\w+)( = ([^;]+))?;\s*/// <summary>(.*?)</summary>\s*public \S+ (\w+)', body, re.S):
        typ, _, _, default, summary, prop = m.groups()
        default = (default or ("false" if typ == "bool" else "Off" if typ == "OneHandedMode" else "0")).strip()
        for s in strip:
            default = default.replace(s, "")
        summary = re.sub(r'\s*///\s*', ' ', summary).strip()
        summary = re.sub(r'<c>(.*?)</c>', r'`\1`', summary)
        summary = re.sub(r'<see cref="(\w+)"/>', r'`\1`', summary)
        rows.append(f"| `{prop}` | `{typ}` | `{default}` | {summary} |")
    title = name[0].upper() + name[1:]
    out.append(f"### {title}\n\n| Setting | Type | Default | Description |\n|---|---|---|---|\n" + "\n".join(rows) + "\n")

doc_path = root / "docs/configuration.md"
doc = doc_path.read_text()
start = doc.index("## Reference") + len("## Reference")
end = doc.index("## Enumerations")
doc = doc[:start] + "\n\n" + "\n".join(out) + "\n" + doc[end:]
doc_path.write_text(doc)
print(f"{sum(o.count(chr(10) + '| `') for o in out)} settings documented")
