#!/usr/bin/env python3
"""The manifest of an application, collected from its sources (docs/c-abi.md, section 17; #636).

    python3 scripts/capi/collect_manifest.py <file or directory>... [--out ferroui.json]
    python3 scripts/capi/collect_manifest.py <file or directory>... --check ferroui.json

Reads C and C++ (`.c .h .cc .cpp .cxx .hpp`), Go (`.go`), Dart (`.dart`) and Osprey (`.osp`) and writes the `ferroui.json` the
generators of the bindings and of the library take: the `classes`, `properties` and `events` the
sources name and the groups of calls (`abi`) they use. With `--check` it writes nothing and fails
when a manifest leaves out a class, a property, an event or a group the sources use.

**Conservative.** A name counts when a token of the source can be that name, in the spelling of
the language: a word that happens to match keeps a class that is not used, which costs size and
nothing else. A name the program builds at run time is not found, and goes in the manifest by
hand. Generated bindings (a `ferroui/` directory) and the outputs of the library are not read.

| Language | Classes | Properties | Events |
| --- | --- | --- | --- |
| C | `ferro_type_stack_panel()`, a literal `"StackPanel"` | a literal `"FontSize"` | a literal `"Click"` |
| C++ | `StackPanel` | `.font_size(`, `Grid::set_row(` | `.on_click(`, `.subscribe_click(` |
| Go | `NewStackPanel(`, `ferroui.StackPanel` | `.SetFontSize(`, `.FontSize(`, `GridSetRow(` | `.OnClick(` |
| Dart | `StackPanel(`, `StackPanel.of(` | `.fontSize`, `Grid.setRow(` | `.onClick(` |
| Osprey | `stackPanel(` | `fontSize(`, `getFontSize(` | `onClick(`, `raiseClick(` |

Osprey programs run on its runtime, which uses frames and signals, so a manifest collected from
Osprey keeps every group of calls (no `abi` key).
"""

import argparse
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.dirname(HERE))
from ferroui_selection import GROUPS, group_of  # noqa: E402
from generate_cpp_bindings import safe as cpp_safe, snake  # noqa: E402
from generate_osprey_bindings import load_model, scalar_kinds, weights  # noqa: E402

C_SUFFIXES = {".c", ".h"}
CPP_SUFFIXES = {".cc", ".cpp", ".cxx", ".hpp"}
GO_SUFFIXES = {".go"}
DART_SUFFIXES = {".dart"}
OSPREY_SUFFIXES = {".osp"}
SKIPPED_DIRECTORIES = {"ferroui", "ferroui-library"}

LITERAL = re.compile(r'"((?:[^"\\\n]|\\.)*)"')
ABI_CALL = re.compile(r"\bferro_[a-z0-9_]+")


def strip_comments(text, language):
    """The source without its comments, so that a class named in one is not a use. String
    literals are kept whole, a `//` or `/*` inside one is not a comment."""
    pattern = r'"(?:[^"\\\n]|\\.)*"|//[^\n]*'
    if language != "osprey":
        pattern += r"|/\*.*?\*/"
    return re.sub(pattern, lambda found: found.group(0) if found.group(0).startswith('"') else " ", text, flags=re.S)


class Spellings:
    """The names of the model in the spelling of each language."""

    def __init__(self, model, names):
        self.class_names = {cls["name"] for cls in model["classes"]}
        self.property_names = {prop["name"] for prop in model["properties"]}
        self.event_names = {event["name"] for event in model["events"]}
        self.entries = {cls["entry"]: cls["name"] for cls in model["classes"] if cls.get("entry")}
        self.cpp_properties, self.cpp_attached, self.cpp_events = {}, {}, {}
        self.osprey_classes, self.osprey_properties, self.osprey_events = {}, {}, {}
        self.go_properties, self.go_attached, self.go_events = {}, {}, {}
        self.dart_properties, self.dart_attached, self.dart_events = {}, {}, {}
        weight = weights(model)
        for cls in model["classes"]:
            if cls["creatable"]:
                self.osprey_classes[names.cls(cls)] = cls["name"]
        for prop in model["properties"]:
            for declaration in prop["declarations"]:
                if declaration["kind"] == "attached":
                    owner, short = prop["name"].rsplit(".", 1)
                    key = (owner, self._snake_of(short))
                    self.cpp_attached[key] = prop["name"]
                    self.go_attached[f"{owner}{short}"] = prop["name"]
                    self.go_attached[f"{owner}Set{short}"] = prop["name"]
                    self.dart_attached[f"{owner}.get{short}"] = prop["name"]
                    self.dart_attached[f"{owner}.set{short}"] = prop["name"]
                else:
                    camel = prop["words"][0] + "".join(word.capitalize() for word in prop["words"][1:])
                    for spelling in (camel, camel + "Value", camel + "Object"):
                        self.dart_properties.setdefault(spelling, set()).add(prop["name"])
                    pascal = "".join(word.capitalize() for word in prop["words"])
                    for spelling in (pascal, pascal + "Value", pascal + "Object", "Set" + pascal,
                                     "Set" + pascal + "Value", "Set" + pascal + "Object"):
                        self.go_properties.setdefault(spelling, set()).add(prop["name"])
                    base = cpp_safe(snake(prop["words"]))
                    self.cpp_properties.setdefault(base, set()).add(prop["name"])
                    self.cpp_properties.setdefault(base + "_object", set()).add(prop["name"])
            base = names.prop(prop)
            spellings = {base, base + "Of", base + "Object", "get" + base[:1].upper() + base[1:]}
            for read_only in (False, True):
                spellings.update(base + suffix for _, suffix in scalar_kinds(model, prop, weight, read_only=read_only))
            for spelling in spellings:
                self.osprey_properties.setdefault(spelling, set()).add(prop["name"])
        for event in model["events"]:
            words = snake(event["words"])
            self.cpp_events[f"on_{words}"] = event["name"]
            self.cpp_events[f"subscribe_{words}"] = event["name"]
            self.osprey_events[names.event(event)] = event["name"]
            self.go_events["On" + "".join(word.capitalize() for word in event["words"])] = event["name"]
            self.dart_events["on" + "".join(word.capitalize() for word in event["words"])] = event["name"]
            self.osprey_events["raise" + "".join(word.capitalize() for word in event["words"])] = event["name"]

    @staticmethod
    def _snake_of(name):
        return re.sub(r"(?<=[a-z0-9])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])", "_", name).lower()


class Usage:
    def __init__(self):
        self.classes, self.properties, self.events, self.groups = set(), set(), set(), set()
        self.osprey = False

    def manifest(self):
        result = {"classes": sorted(self.classes), "properties": sorted(self.properties),
                  "events": sorted(self.events)}
        if not self.osprey:
            result["abi"] = sorted(self.groups)
        return result


def read_c(text, spellings, usage):
    """What C and C++ share: the entries of the classes, the names in literals, and the calls."""
    for call in ABI_CALL.findall(text):
        if call in spellings.entries:
            usage.classes.add(spellings.entries[call])
        group = group_of(call)
        if group != "core":
            usage.groups.add(group)
    for literal in LITERAL.findall(text):
        if literal in spellings.class_names:
            usage.classes.add(literal)
        if literal in spellings.property_names:
            usage.properties.add(literal)
        if literal in spellings.event_names:
            usage.events.add(literal)


def read_cpp(text, spellings, usage):
    read_c(text, spellings, usage)
    # A literal is not code: the words of the text of a label name nothing.
    code = LITERAL.sub('""', text)
    for word in re.findall(r"\b[A-Z][A-Za-z0-9_]*\b", code):
        if word in spellings.class_names:
            usage.classes.add(word)
    for member in re.findall(r"(?:\.|->)\s*([a-z_][a-z0-9_]*)\s*\(", code):
        usage.properties.update(spellings.cpp_properties.get(member, ()))
        if member in spellings.cpp_events:
            usage.events.add(spellings.cpp_events[member])
    for owner, short in re.findall(r"\b([A-Z]\w*)::(?:set|get)_([a-z0-9_]+)\s*\(", code):
        name = spellings.cpp_attached.get((owner, short.rstrip("_")))
        if name:
            usage.properties.add(name)


def read_go(text, spellings, usage):
    # The runtime of the Go bindings links `ferro_raise`, a call of this group.
    usage.groups.add(group_of("ferro_raise"))
    code = LITERAL.sub('""', text)
    for word in re.findall(r"\b(?:New|ferroui\.)([A-Z]\w*)", code):
        if word in spellings.class_names:
            usage.classes.add(word)
    for member in re.findall(r"\.\s*([A-Z]\w*)\s*\(", code):
        usage.properties.update(spellings.go_properties.get(member, ()))
        if member in spellings.go_events:
            usage.events.add(spellings.go_events[member])
        if member in spellings.go_attached:
            usage.properties.add(spellings.go_attached[member])


def read_dart(text, spellings, usage):
    code = re.sub(r"'(?:[^'\\\n]|\\.)*'", "''", LITERAL.sub('""', text))
    for word in re.findall(r"\b([A-Z]\w*)\s*(?:\(|\.of\s*\()", code):
        if word in spellings.class_names:
            usage.classes.add(word)
    for owner, member in re.findall(r"\b([A-Z]\w*)\.((?:get|set)[A-Z]\w*)\s*\(", code):
        if f"{owner}.{member}" in spellings.dart_attached:
            usage.properties.add(spellings.dart_attached[f"{owner}.{member}"])
    for member in re.findall(r"\.\s*([a-z]\w*)", code):
        usage.properties.update(spellings.dart_properties.get(member, ()))
        if member in spellings.dart_events:
            usage.events.add(spellings.dart_events[member])


def read_osprey(text, spellings, usage):
    usage.osprey = True
    code = LITERAL.sub('""', text)
    for word in re.findall(r"\b([a-zA-Z_]\w*)\s*\(", code):
        if word in spellings.osprey_classes:
            usage.classes.add(spellings.osprey_classes[word])
        usage.properties.update(spellings.osprey_properties.get(word, ()))
        if word in spellings.osprey_events:
            usage.events.add(spellings.osprey_events[word])


def sources(paths):
    for path in paths:
        if os.path.isfile(path):
            yield path
            continue
        if not os.path.isdir(path):
            raise SystemExit(f"{path} is neither a file nor a directory")
        for directory, subdirectories, files in os.walk(path):
            subdirectories[:] = sorted(name for name in subdirectories if name not in SKIPPED_DIRECTORIES)
            for name in sorted(files):
                yield os.path.join(directory, name)


def collect(paths, model=None, names=None):
    """The manifest of the sources under `paths`."""
    if model is None:
        model, names = load_model()
    spellings = Spellings(model, names)
    usage = Usage()
    found = 0
    for path in sources(paths):
        suffix = os.path.splitext(path)[1].lower()
        if suffix in C_SUFFIXES:
            language, reader = "c", read_c
        elif suffix in CPP_SUFFIXES:
            language, reader = "cpp", read_cpp
        elif suffix in GO_SUFFIXES:
            language, reader = "go", read_go
        elif suffix in DART_SUFFIXES:
            language, reader = "dart", read_dart
        elif suffix in OSPREY_SUFFIXES:
            language, reader = "osprey", read_osprey
        else:
            continue
        with open(path, encoding="utf-8", errors="replace") as file:
            reader(strip_comments(file.read(), language), spellings, usage)
        found += 1
    if not found:
        raise SystemExit("no C, C++, Go, Dart or Osprey source among the paths")
    return usage.manifest()


def missing(collected, manifest):
    """What the sources use and `manifest` leaves out: [(key, names)]. A manifest without a list
    of properties, events or groups has all of them."""
    gaps = []
    for key in ("classes", "properties", "events", "abi"):
        if key == "classes" or key in manifest:
            left_out = sorted(set(collected.get(key, [])) - set(manifest.get(key, [])))
            if left_out:
                gaps.append((key, left_out))
    if "abi" in manifest and "abi" not in collected and set(manifest["abi"]) != set(GROUPS):
        # Collected from Osprey: every group.
        gaps.append(("abi", sorted(set(GROUPS) - set(manifest["abi"]))))
    return gaps


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n", maxsplit=1)[0])
    parser.add_argument("paths", nargs="+", help="source files or directories")
    parser.add_argument("--out", help="write the manifest here (default: print it)")
    parser.add_argument("--check", help="fail if this manifest leaves out what the sources use")
    options = parser.parse_args()
    collected = collect(options.paths)
    if options.check:
        with open(options.check, encoding="utf-8") as file:
            gaps = missing(collected, json.load(file))
        for key, names in gaps:
            print(f"{options.check}: `{key}` leaves out {', '.join(names)}")
        sys.exit(1 if gaps else 0)
    text = json.dumps(collected, indent=2) + "\n"
    if options.out:
        with open(options.out, "w", encoding="utf-8", newline="\n") as file:
            file.write(text)
        print(f"{options.out}: {len(collected['classes'])} classes, {len(collected['properties'])} properties, "
              f"{len(collected['events'])} events")
    else:
        sys.stdout.write(text)


if __name__ == "__main__":
    main()
