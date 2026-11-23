#!/usr/bin/env python3
"""The metadata walk of FerroUI's object model, shared by the generators.

Reads the Rust sources of FerroUI.Base and FerroUI.Controls (and of the other
crates of controls a generator names) and collects, without building
anything:

    class and base       `ferro_class! { Name: Base, ... }`
    can be created       `ferro_class_info!(Name { new: ...`
    properties           `pub fn x_property() -> StyledProperty<T>` (and
                         `DirectProperty<Owner, T>`, `AttachedProperty<T>`)
                         in the `impl` of the owner
    routed events        `pub fn x_event() -> RoutedEvent<Args>`
    paths of types       the `rust_paths.rs` of the crates
    details              the registered name of each property and event, its
                         default value, its documentation, the read-only
                         direct properties, the documentation of the classes,
                         and the enumerations with their members

Two generators use it: `generate_view_builders.py` (the element builders of
`ferroui-view`) and `generate_api_model.py` (the language-neutral API model of
the bindings, `bindings/model/ferroui-api.json`). Standard library only.
"""

import os
import re

import rust_paths

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Public types whose simple name another public type of the crate has: the name the script's
# model gives each (its builders' re-export is named after it). A source that declares or imports
# the path names the type by this name; the generated signatures use the public path. The entry
# has no effect while the path does not exist (`navigation` lands with Frame,
# docs/porting/winui-controls.md).
SCOPED_PATHS = {
    "ferroui_controls::navigation::NavigationEventArgs": "FrameNavigationEventArgs",
}

# The crates whose classes get builders: (source directory, crate name in paths).
CRATES = [
    ("src/FerroUI.Base", "ferroui_base"),
    ("src/FerroUI.Controls", "ferroui_controls"),
]

# Types every value type may name without a path.
PRELUDE_TYPES = {
    "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize",
    "u8", "u16", "u32", "u64", "u128", "usize", "str", "String", "Option", "Vec", "Box",
}
STD_TYPES = {
    "Rc": "std::rc::Rc",
    "Arc": "std::sync::Arc",
    "Duration": "std::time::Duration",
}

# Types of the chosen crates of the workspace, and the Rust handles of markup types,
# that a property can hold and that no `rust_paths.rs` lists (they have no markup type
# of their own: `UriRef` is a handle of `System.Uri`), by the path FerroUI re-exports
# them by (C5 of #423, docs/idiomatic-rust-signatures.md).
EXTERNAL_TYPES = {
    "Locale": "ferroui_base::utilities::Locale",
    "UriRef": "ferroui_base::utilities::UriRef",
}

RUST_KEYWORDS = {
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
}


def snake(name):
    name = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    name = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name)
    return name.lower()


def ident(name):
    return f"r#{name}" if name in RUST_KEYWORDS else name


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def rust_files(directory):
    for root, dirs, files in os.walk(directory):
        dirs[:] = sorted(d for d in dirs if d not in ("tests", "target") and not d.endswith("_tests"))
        for name in sorted(files):
            if name.endswith(".rs") and not name.endswith("_tests.rs") and name != "tests.rs":
                yield os.path.join(root, name)


def strip_comments(text):
    """Blanks out line comments (keeping offsets), so that patterns do not
    match inside documentation."""
    return re.sub(r"//[^\n]*", lambda m: " " * len(m.group(0)), text)


def matching(text, start, open_char, close_char):
    """The index of the character that closes the one at `start`."""
    depth = 0
    for index in range(start, len(text)):
        char = text[index]
        if char == open_char:
            depth += 1
        elif char == close_char:
            depth -= 1
            if depth == 0:
                return index
    return len(text)


def split_top_level(text, separator=","):
    parts, depth, current = [], 0, ""
    for char in text:
        if char in "<([{":
            depth += 1
        elif char in ">)]}":
            depth -= 1
        if char == separator and depth == 0:
            parts.append(current.strip())
            current = ""
        else:
            current += char
    if current.strip():
        parts.append(current.strip())
    return parts


def generic_argument(text, start):
    """The text between the `<` at `start` and its `>`."""
    end = matching(text, start, "<", ">")
    return text[start + 1:end]


class Model:
    def __init__(self):
        self.paths = {}            # type name -> public path ("ferroui_controls::Button")
        self.ambiguous = set()
        self.bases = {}            # class -> base class
        self.crate_of = {}         # class -> crate
        self.creatable = set()
        self.properties = {}       # owner -> [(kind, snake, type, target, accessor)]
        self.events = {}           # owner -> [(snake, args)]
        self.left_out = []
        # The details the API model reads (generate_api_model.py); the view builders do not.
        self.property_details = {}  # (owner, snake) -> {"name", "default", "doc"}
        self.read_only = {}         # owner -> [(kind, snake, type, holder)]: direct, no setter
        self.event_details = {}     # (owner, snake) -> {"name", "doc"}
        self.class_docs = {}        # class -> documentation of its struct
        self.enums = {}             # name -> {"crate", "members": [(name, value, doc)], "doc"}

    def add_path(self, name, path):
        if name in self.paths and self.paths[name] != path:
            self.ambiguous.add(name)
        self.paths[name] = path


def read_paths(model, directory, crate):
    text = read(os.path.join(ROOT, directory, "rust_paths.rs"))
    for match in re.finditer(r"crate::([A-Za-z0-9_:]+)", text):
        path = match.group(1)
        if path.startswith("ferro_rust_paths"):
            continue
        name = path.split("::")[-1]
        public_path = f"{crate}::{path}"
        model.add_path(SCOPED_PATHS.get(public_path, name), public_path)


def qualify_scoped_names(text, crate, relative_path):
    """Keep same-name contracts separate using their declaration/import scope."""
    module = relative_path.replace(os.sep, "/").removesuffix(".rs").split("/")
    if module[-1] in ("lib", "mod"):
        module.pop()
    aliases = {}
    for public_path, alias in SCOPED_PATHS.items():
        segments = public_path.split("::")
        if segments[:-1] == [crate] + module:
            aliases[segments[-1]] = alias

    for statement in rust_paths.top_level_statements(rust_paths.strip_comments(text)):
        _, statement = rust_paths.split_attributes(statement)
        imported = re.match(r"(?:pub(?:\([^)]*\))?\s+)?use\s+(.+);$", statement, re.S)
        if not imported:
            continue
        for segments, local in rust_paths.parse_use_tree(imported.group(1)):
            if not segments:
                continue
            if segments[0] == "crate":
                segments = [crate] + segments[1:]
            elif segments[0] in ("self", "super"):
                current = [crate] + module
                while segments and segments[0] in ("self", "super"):
                    if segments.pop(0) == "super":
                        current.pop()
                segments = current + segments
            imported_path = "::".join(segments)
            if local == "*":
                for public_path, alias in SCOPED_PATHS.items():
                    if public_path.rsplit("::", 1)[0] == imported_path:
                        aliases[public_path.rsplit("::", 1)[1]] = alias
            elif imported_path in SCOPED_PATHS:
                aliases[local] = SCOPED_PATHS[imported_path]

    for public_path, alias in SCOPED_PATHS.items():
        paths = [public_path]
        if public_path.startswith(crate + "::"):
            paths.append("crate::" + public_path[len(crate) + 2:])
        for path in paths:
            text = re.sub(r"\b" + re.escape(path) + r"\b", alias, text)
    return re.sub(r"\b[A-Z]\w*\b", lambda match: aliases.get(match.group(0), match.group(0)), text)


def read_classes(model, directory, crate):
    for path in rust_files(os.path.join(ROOT, directory)):
        raw = read(path)
        read_docs_and_enums(model, raw, crate)
        text = strip_comments(raw)
        relative_path = os.path.relpath(path, os.path.join(ROOT, directory))
        text = qualify_scoped_names(text, crate, relative_path)
        for match in re.finditer(r"ferro_class!\s*[({]\s*([A-Z]\w*)\s*:\s*([A-Z]\w*)", text):
            model.bases[match.group(1)] = match.group(2)
            model.crate_of[match.group(1)] = crate
        for match in re.finditer(r"ferro_class_info!\s*\(\s*([A-Z]\w*)\s*\{\s*new\s*:", text):
            model.creatable.add(match.group(1))
        read_members(model, text, crate, path, docs_of(raw))


IMPL = re.compile(r"\bimpl\s+([A-Z]\w*)\s*(?:,\s*also\s*\[[^\]]*\]\s*)?\{")
PROPERTY = re.compile(r"pub fn (\w+)_property\(\)\s*->\s*(StyledProperty|DirectProperty|AttachedProperty)\s*<")
EVENT = re.compile(r"pub fn (\w+)_event\(\)\s*->\s*RoutedEvent\s*<")


def owner_at(text, position):
    """The class a declaration at `position` belongs to and the type whose
    `impl` holds its accessor: the same class, but for
    `ferro_property!(for X; ...)` in the `impl` of another type, which
    registers the property on `X`."""
    impls = [m for m in IMPL.finditer(text, 0, position)]
    holder = impls[-1].group(1) if impls else None
    start = text.rfind("ferro_property!(", 0, position)
    if start != -1:
        declared = re.match(r"ferro_property!\(\s*for\s+([A-Z]\w*)\s*;", text[start:position])
        if declared:
            return declared.group(1), holder
    return holder, holder


def read_members(model, text, crate, path, docs=None):
    docs = docs or {}
    seen = {}
    for match in PROPERTY.finditer(text):
        occurrence = seen.get(("property", match.group(1)), 0)
        seen[("property", match.group(1))] = occurrence + 1
        owner, holder = owner_at(text, match.start())
        if owner is None or holder is None:
            continue
        kind = match.group(2)
        arguments = generic_argument(text, match.end() - 1)
        body_start = text.find("{", match.end())
        body = text[body_start:matching(text, body_start, "{", "}") + 1]
        model.property_details[(owner, match.group(1))] = {
            "name": registered_name(body),
            "default": registered_default(body),
            "doc": docs.get(("property", match.group(1), occurrence), ""),
        }
        value_type = arguments
        if kind == "DirectProperty":
            parts = split_top_level(arguments)
            if len(parts) != 2:
                continue
            value_type = parts[1]
            direct = re.search(r"register_direct(?:_with)?::<[^(]*>\s*\(", body)
            if direct:
                call = body[direct.end() - 1:matching(body, direct.end() - 1, "(", ")") + 1]
                args = split_top_level(call[1:-1])
                if len(args) >= 3 and args[2].strip() == "None":
                    model.left_out.append(f"{owner}.{match.group(1)}: a direct property without a setter")
                    model.read_only.setdefault(owner, []).append((kind, match.group(1), value_type, holder))
                    continue
        target = None
        if kind == "AttachedProperty":
            attached = re.search(r"register_attached(?:_with)?::<\s*(\w+)\s*,\s*(\w+)", body)
            target = attached.group(2) if attached else "Control"
        model.properties.setdefault(owner, []).append((kind, match.group(1), value_type, target, holder))
    for match in EVENT.finditer(text):
        occurrence = seen.get(("event", match.group(1)), 0)
        seen[("event", match.group(1))] = occurrence + 1
        owner, holder = owner_at(text, match.start())
        if owner is None:
            continue
        args = generic_argument(text, match.end() - 1)
        model.events.setdefault(owner, []).append((match.group(1), args))
        body_start = text.find("{", match.end())
        body = text[body_start:matching(text, body_start, "{", "}") + 1]
        model.event_details[(owner, match.group(1))] = {
            "name": registered_name(body),
            "doc": docs.get(("event", match.group(1), occurrence), ""),
        }


def resolve(model, type_text):
    """The type with public paths, or None when a name has none."""
    out, position = "", 0
    for match in re.finditer(r"\b[A-Za-z_]\w*\b", type_text):
        name = match.group(0)
        out += type_text[position:match.start()]
        position = match.end()
        if name in PRELUDE_TYPES or name == "dyn":
            out += name
        elif name in STD_TYPES:
            out += STD_TYPES[name]
        elif name in EXTERNAL_TYPES:
            out += EXTERNAL_TYPES[name]
        elif name in model.paths and name not in model.ambiguous:
            out += model.paths[name]
        else:
            return None
    return out + type_text[position:]


def ancestors(model, name):
    chain = []
    current = model.bases.get(name)
    while current is not None:
        chain.append(current)
        current = model.bases.get(current)
    return chain


def class_path(model, name):
    if name in model.paths and name not in model.ambiguous:
        return model.paths[name]
    return None


# ---- details for the API model ------------------------------------------------------------------

def registered_name(body):
    """The name a property or event registers (`register::<Button, _>("ClickMode", ...)`): the
    first string literal of its accessor; None for an accessor that adds an owner to the property
    of another class (`TextElement::font_size_property().add_owner::<X>()`)."""
    found = re.search(r'"([A-Za-z_][A-Za-z0-9_.]*)"', body)
    return found.group(1) if found else None


def registered_default(body):
    """The default value of a property as the Rust expression its registration gives
    (`ClickMode::Release`), when the registration names one: the argument after the name, or the
    argument of `StyledPropertyOptions::new(...)`. Informative: None when it is not written there."""
    found = re.search(r'"[A-Za-z_][A-Za-z0-9_.]*"\s*,', body)
    if not found:
        return None
    rest = body[found.end():]
    options = re.match(r"\s*StyledPropertyOptions::new\(", rest)
    if options:
        start = rest.index("(", options.start())
        value = rest[start + 1:matching(rest, start, "(", ")")]
        return " ".join(value.split()) or None
    depth, value = 0, ""
    for char in rest:
        if char in "([{<":
            depth += 1
        elif char in ")]}>":
            if depth == 0:
                break
            depth -= 1
        elif char == "," and depth == 0:
            break
        value += char
    value = " ".join(value.split())
    return value or None


DOC_LINE = re.compile(r"^\s*///\s?(.*)$")
ATTRIBUTE_LINE = re.compile(r"^\s*#\[")


def doc_before(lines, index):
    """The `///` documentation above line `index`, attributes skipped, as one paragraph."""
    collected = []
    index -= 1
    while index >= 0:
        line = lines[index]
        doc = DOC_LINE.match(line)
        if doc:
            collected.append(doc.group(1).strip())
        elif ATTRIBUTE_LINE.match(line):
            pass
        else:
            break
        index -= 1
    return " ".join(reversed([line for line in collected if line])).strip()


DECLARATION_LINE = re.compile(r"^\s*pub fn (\w+)_(property|event)\(\)")


def docs_of(raw):
    """The documentation of the property and event accessors of a file, by (kind, snake name,
    occurrence in the file)."""
    lines = raw.split("\n")
    docs, seen = {}, {}
    for index, line in enumerate(lines):
        found = DECLARATION_LINE.match(line)
        if not found:
            continue
        key = (found.group(2), found.group(1))
        occurrence = seen.get(key, 0)
        seen[key] = occurrence + 1
        docs[(found.group(2), found.group(1), occurrence)] = doc_before(lines, index)
    return docs


STRUCT_LINE = re.compile(r"^pub struct ([A-Z]\w*)\b")
ENUM_LINE = re.compile(r"^pub enum ([A-Z]\w*)\s*\{")


def read_docs_and_enums(model, raw, crate):
    """The documentation of the public structs (the classes) and the public enumerations whose
    members are all units, with their numeric values (explicit discriminants or the count)."""
    lines = raw.split("\n")
    for index, line in enumerate(lines):
        found = STRUCT_LINE.match(line)
        if found:
            model.class_docs.setdefault(found.group(1), doc_before(lines, index))
            continue
        found = ENUM_LINE.match(line)
        if not found:
            continue
        start = raw.index(line)
        brace = raw.index("{", start)
        block = raw[brace + 1:matching(raw, brace, "{", "}")]
        members, value, unit_only = [], 0, True
        block_lines = block.split("\n")
        for position, member_line in enumerate(block_lines):
            stripped = member_line.strip()
            if not stripped or stripped.startswith("//") or stripped.startswith("#"):
                continue
            member = re.match(r"^([A-Z]\w*)\s*(?:=\s*(-?(?:0x[0-9A-Fa-f_]+|[0-9_]+)))?\s*,?\s*(?://.*)?$", stripped)
            if not member:
                unit_only = False
                break
            if member.group(2):
                value = int(member.group(2).replace("_", ""), 0)
            members.append((member.group(1), value, doc_before(block_lines, position)))
            value += 1
        if unit_only and members and found.group(1) not in model.enums:
            model.enums[found.group(1)] = {"crate": crate, "members": members, "doc": doc_before(lines, index)}
