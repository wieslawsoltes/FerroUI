"""The public Rust paths of the registered types of a crate, for the emitter of
Rust source of the markup compiler (docs/porting/xaml.md, section 9).

Generated Rust names a type of a framework crate from another crate, so it
needs a PUBLIC path of the type (`ferroui_controls::Border`), which is not the
path of the module that declares it (`border` is a private module; the type is
re-exported by `pub use border::Border;` in `lib.rs`). This module reads the
module tree of a crate (`mod` / `pub mod` declarations, `pub use` of single
names, lists and globs, items declared `pub`), resolves the namespaces of all
modules by a fixpoint over the `use` declarations (an explicit import shadows a
glob, as in Rust), and gives every item its shortest public path (ties broken
alphabetically by segments).

`generate_markup_types.py` calls `rust_paths_file(crate)` to write the
generated `rust_paths.rs` of each framework crate. The file lists, for every
class of the crate's type table (`register_types.rs`) and every markup type of
its type lists (`const TYPES: &[&MarkupType]`, `markup_types![..]`), the
crate-relative public path; the macro `ferro_rust_paths!` turns each path into
the registered item and its path text from the same tokens, so rustc checks
that the path names the registered type. That the paths are public is checked
from outside the crates by a checked-in file of the XAML test crate
(`emitter/rust_paths_check.rs`).

The parser is a reader of the crate's own declaration style, not of Rust in
general: modules under `#[cfg(test)]` are skipped, an inline `mod x { .. }` is
read, items defined by the macros of `ITEM_MACROS` are known, generic types and
types of other crates are not listed. Python 3, standard library only.
"""

import os
import re

# Macros of the framework that declare a public type: regex over the invocation
# text -> the group that is the type name.
ITEM_MACROS = [
    re.compile(r"^(?:\w+::)*ferro_transition_class!\s*\(\s*(\w+)\s*:"),
    re.compile(r"^(?:\w+::)*media_collection_type!\s*\(\s*(\w+)\s*,"),
]

VIS = r"(?P<vis>pub(?:\s*\((?:crate|super|self|in [\w:]+)\))?\s+)?"


def strip_comments(text):
    """The text without comments, with string and character literals emptied."""
    out = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            if j < 0:
                break
            i = j
            continue
        if text.startswith("/*", i):
            depth = 1
            i += 2
            while i < n and depth:
                if text.startswith("/*", i):
                    depth += 1
                    i += 2
                elif text.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
            continue
        if c == "r" and re.match(r'r#*"', text[i : i + 8]) and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            m = re.match(r'r(#*)"', text[i:])
            hashes = m.group(1)
            end = text.find('"' + hashes, i + len(m.group(0)))
            out.append('""')
            i = end + 1 + len(hashes)
            continue
        if c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                if text[j] == "\\":
                    j += 1
                j += 1
            out.append('""')
            i = j + 1
            continue
        if c == "'":
            m = re.match(r"'(\\u\{[0-9a-fA-F]+\}|\\.|[^\\'])'", text[i:])
            if m:
                out.append("' '")
                i += len(m.group(0))
                continue
        out.append(c)
        i += 1
    return "".join(out)


def top_level_statements(body):
    """The top-level items of a module body (comments stripped), in order."""
    i = 0
    n = len(body)
    start = 0
    depth = 0
    while i < n:
        c = body[i]
        if c in "{[(":
            depth += 1
        elif c in "}])":
            depth -= 1
            if depth == 0 and c == "}":
                j = i + 1
                while j < n and body[j] in " \t\r\n":
                    j += 1
                if j < n and body[j] == ";":
                    i = j
                yield body[start : i + 1].strip()
                start = i + 1
        elif c == ";" and depth == 0:
            yield body[start : i + 1].strip()
            start = i + 1
        i += 1


def match_bracket(text, open_index):
    depth = 0
    for k in range(open_index, len(text)):
        if text[k] in "[({":
            depth += 1
        elif text[k] in "])}":
            depth -= 1
            if depth == 0:
                return k
    return len(text) - 1


def split_attributes(statement):
    attributes = []
    while True:
        statement = statement.lstrip()
        if statement.startswith("#!["):
            statement = statement[match_bracket(statement, 2) + 1 :]
        elif statement.startswith("#["):
            end = match_bracket(statement, 1)
            attributes.append(statement[2:end])
            statement = statement[end + 1 :]
        else:
            return attributes, statement


def is_test_only(attributes):
    for attribute in attributes:
        compact = attribute.replace(" ", "")
        if compact == "cfg(test)" or compact.startswith(("cfg(all(test", "cfg(any(test")):
            return True
    return False


def parse_use_tree(text):
    """`a::b::{C, D as E, f::*}` -> [(segments, alias or '*')]."""
    result = []

    def walk(prefix, tree):
        tree = tree.strip()
        if not tree:
            return
        if tree.endswith("}") and "{" in tree:
            k = tree.index("{")
            head = tree[:k].strip().rstrip(":").strip()
            head_segments = [p for p in head.split("::") if p] if head else []
            depth, current = 0, ""
            for ch in tree[k + 1 : -1]:
                if ch == "{":
                    depth += 1
                if ch == "}":
                    depth -= 1
                if ch == "," and depth == 0:
                    walk(prefix + head_segments, current)
                    current = ""
                else:
                    current += ch
            walk(prefix + head_segments, current)
            return
        m = re.match(r"^(.*?)\s+as\s+(\w+)$", tree, re.S)
        if m:
            segments = [p.strip() for p in m.group(1).split("::") if p.strip()]
            if m.group(2) != "_":
                result.append((prefix + segments, m.group(2)))
            return
        segments = [p.strip() for p in tree.split("::") if p.strip()]
        if segments[-1] == "*":
            result.append((prefix + segments[:-1], "*"))
        elif segments[-1] == "self":
            result.append((prefix + segments[:-1], (prefix + segments)[-2]))
        else:
            result.append((prefix + segments, segments[-1]))

    walk([], text)
    return result


class Module:
    def __init__(self, path, file):
        self.path = path  # segments below the crate root
        self.file = file
        self.children = {}  # name -> (visibility, Module)
        self.items = {}  # name -> (visibility, kind)
        self.uses = []  # (visibility, segments, alias or '*')


def is_pub(visibility):
    return visibility is not None and visibility.strip() == "pub"


def parse_module(path, file, body):
    module = Module(path, file)
    for statement in top_level_statements(body):
        attributes, s = split_attributes(statement)
        if is_test_only(attributes):
            continue
        m = re.match(VIS + r"mod\s+(\w+)\s*;", s)
        if m:
            name = m.group(2)
            directory = os.path.dirname(file) if os.path.basename(file) in ("lib.rs", "mod.rs") else file[:-3]
            for candidate in (os.path.join(directory, name + ".rs"), os.path.join(directory, name, "mod.rs")):
                if os.path.exists(candidate):
                    with open(candidate, encoding="utf-8") as f:
                        child = parse_module(path + [name], candidate, strip_comments(f.read()))
                    module.children[name] = (m.group("vis"), child)
                    break
            continue
        m = re.match(VIS + r"mod\s+(\w+)\s*\{", s)
        if m:
            child = parse_module(path + [m.group(2)], file, s[s.index("{") + 1 : s.rindex("}")])
            module.children[m.group(2)] = (m.group("vis"), child)
            continue
        m = re.match(VIS + r"use\s+(.*);$", s, re.S)
        if m:
            for segments, alias in parse_use_tree(m.group(2)):
                module.uses.append((m.group("vis"), segments, alias))
            continue
        m = re.match(VIS + r"(?:unsafe\s+)?(struct|enum|trait|type|union)\s+(\w+)", s)
        if m:
            module.items[m.group(3)] = (m.group("vis"), m.group(2))
            continue
        if re.match(r"(?:\w+::)*\w+!\s*[\({\[]", s) and not s.startswith("macro_rules!"):
            # A macro invocation that declares items (`bitflags!`, the property-bag
            # macros): the types written in it, and the types of `ITEM_MACROS`.
            for m2 in re.finditer(VIS + r"(struct|enum|trait)\s+(\w+)", s):
                if m2.group("vis") is not None:
                    module.items[m2.group(3)] = (m2.group("vis"), m2.group(2))
            for pattern in ITEM_MACROS:
                m = pattern.match(s)
                if m:
                    module.items[m.group(1)] = ("pub ", "struct")
    return module


class Crate:
    """The module tree of a crate and the namespaces of its modules."""

    def __init__(self, crate_root):
        lib = os.path.join(crate_root, "lib.rs")
        with open(lib, encoding="utf-8") as f:
            self.root = parse_module([], lib, strip_comments(f.read()))
        self.modules = {}
        self._index(self.root)
        self.namespaces = {}
        for key, module in self.modules.items():
            names = {}
            for name, (visibility, child) in module.children.items():
                names[name] = (visibility, ("module", key + (name,)))
            for name, (visibility, kind) in module.items.items():
                names[name] = (visibility, ("item", (key, name)))
            self.namespaces[key] = names
        self.crate_root = os.path.dirname(lib)
        # The names each module imports from other crates (`use std::rc::Rc;`):
        # name -> the absolute path.
        self.externals = {}
        for key, module in self.modules.items():
            for visibility, segments, alias in module.uses:
                if alias != "*" and segments[0] not in ("crate", "self", "super") and segments[0] not in module.children:
                    self.externals.setdefault(key, {})[alias] = segments
        globbed = set()
        changed = True
        while changed:
            changed = False
            for key, module in self.modules.items():
                names = self.namespaces[key]
                for visibility, segments, alias in module.uses:
                    if alias == "*":
                        target = self.resolve(key, segments)
                        if target is None or target[0] != "module":
                            continue
                        for name, (v2, t2) in list(self.namespaces[target[1]].items()):
                            if v2 is None or name in names:
                                continue
                            names[name] = (visibility, t2)
                            globbed.add((key, name))
                            changed = True
                    elif alias not in names or (key, alias) in globbed:
                        target = self.resolve(key, segments)
                        if target is not None and names.get(alias) != (visibility, target):
                            names[alias] = (visibility, target)
                            globbed.discard((key, alias))
                            changed = True

    def _index(self, module):
        self.modules[tuple(module.path)] = module
        for _, child in module.children.values():
            self._index(child)

    def resolve(self, key, segments):
        """A path as written in the module `key` -> ('module', key) | ('item', (key, name)) | None."""
        current = key
        rest = list(segments)
        if rest[0] == "crate":
            current, rest = (), rest[1:]
        elif rest[0] == "self":
            rest = rest[1:]
        while rest and rest[0] == "super":
            current, rest = current[:-1], rest[1:]
        if not rest:
            return ("module", current)
        for k, segment in enumerate(rest):
            entry = self.namespaces.get(current, {}).get(segment)
            if entry is None:
                return None
            target = entry[1]
            if k == len(rest) - 1:
                return target
            if target[0] != "module":
                return None
            current = target[1]
        return None

    def module_of_file(self, file):
        keys = [key for key, module in self.modules.items() if module.file == file]
        return min(keys, key=len) if keys else None

    def public_paths(self):
        """The item (module key, name) -> its shortest public path (segments)."""
        best = {}
        queue = [((), [])]
        seen = set()
        while queue:
            key, public = queue.pop(0)
            if key in seen:
                continue
            seen.add(key)
            for name, (visibility, target) in sorted(self.namespaces[key].items(), key=lambda entry: entry[0]):
                if not is_pub(visibility):
                    continue
                if target[0] == "module":
                    queue.append((target[1], public + [name]))
                else:
                    path = public + [name]
                    old = best.get(target[1])
                    if old is None or (len(path), path) < (len(old), old):
                        best[target[1]] = path
        return best


PRELUDE = {
    "Option": "::std::option::Option",
    "String": "::std::string::String",
    "Vec": "::std::vec::Vec",
    "Box": "::std::boxed::Box",
}
PRIMITIVES = {
    "bool", "char", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "str",
}


def render_type(crate, public, key, text, crate_name):
    """The Rust type `text`, as written in the module `key`, with every path made
    absolute: (the text for the crate itself, with `crate::` for its items; the text
    for other crates, with `::crate_name::`). None if a path does not resolve to a
    public item."""
    tokens = re.findall(r"::|[<>,&()\[\];]|'static|\w+", text)
    inner, outer = [], []
    i = 0
    while i < len(tokens):
        token = tokens[i]
        if re.match(r"^\w+$", token) and token not in ("dyn", "mut") or token == "::":
            segments = []
            if token == "::":
                i += 1
            while i < len(tokens) and re.match(r"^\w+$", tokens[i]):
                segments.append(tokens[i])
                if i + 1 < len(tokens) and tokens[i + 1] == "::" and i + 2 < len(tokens) and re.match(r"^\w+$", tokens[i + 2]):
                    i += 2
                else:
                    i += 1
                    break
            rendered = render_path(crate, public, key, segments)
            if rendered is None:
                return None
            inner.append(rendered[0])
            outer.append(rendered[1].replace("crate::", "::%s::" % crate_name, 1) if rendered[1].startswith("crate::") else rendered[1])
            continue
        inner.append(token + (" " if token == "dyn" else ""))
        outer.append(token + (" " if token == "dyn" else ""))
        i += 1
    return "".join(inner), "".join(outer)


def render_path(crate, public, key, segments):
    if len(segments) == 1 and segments[0] in PRIMITIVES:
        return segments[0], segments[0]
    target = crate.resolve(key, segments)
    if target is not None:
        if target[0] != "item" or target[1] not in public:
            return None
        path = "crate::" + "::".join(public[target[1]])
        return path, path
    external = crate.externals.get(key, {}).get(segments[0])
    if external is not None:
        path = "::" + "::".join(external + segments[1:])
        return path, path
    if len(segments) == 1 and segments[0] in PRELUDE:
        return PRELUDE[segments[0]], PRELUDE[segments[0]]
    return None


def markup_entries(text):
    """The types `T` of the entries `<T as MarkupTyped>::MARKUP` of a type list, with
    balanced generic arguments."""
    entries = []
    for m in re.finditer(r"\s+as\s+MarkupTyped\s*>::MARKUP", text):
        depth = 0
        k = m.start() - 1
        while k >= 0:
            if text[k] == ">":
                depth += 1
            elif text[k] == "<":
                if depth == 0:
                    break
                depth -= 1
            k -= 1
        entries.append(text[k + 1 : m.start()].strip())
    return entries


def source_files(crate_root):
    for current, directories, files in os.walk(crate_root):
        directories.sort()
        for name in sorted(files):
            if name.endswith(".rs") and not name.endswith("_tests.rs"):
                yield os.path.join(current, name)


def registered_types(crate_root, crate):
    """[(kind, written text, file, item or None)] of the registered classes ('class') and the
    markup types ('type', 'contract') of the crate, in the order of their lists."""
    result = []
    with open(os.path.join(crate_root, "register_types.rs"), encoding="utf-8") as f:
        registration = strip_comments(f.read())
    start = registration.index("const TYPES")
    table = registration[start : registration.index("];", start)]
    for path in re.findall(r"crate((?:::\w+)+)", table):
        segments = ["crate"] + [p for p in path.split("::") if p]
        result.append(("class", "::".join(segments), "register_types.rs", crate.resolve((), segments)))
    for name in re.findall(r"<\s*([\w:]+)\s+as\s+StaticType\s*>::TYPE", table):
        result.append(("class", name, "register_types.rs", crate.resolve(("register_types",), name.split("::"))))
    for file in source_files(crate_root):
        key = crate.module_of_file(file)
        if key is None:
            continue
        with open(file, encoding="utf-8") as f:
            text = strip_comments(f.read())
        lists = re.findall(r"const \w*TYPES\s*:\s*&\[&MarkupType\]\s*=\s*&\[(.*?)\];", text, re.S)
        lists += re.findall(r"const \w*TYPES\s*:\s*&\[&MarkupType\]\s*=\s*markup_types!\[(.*?)\];", text, re.S)
        for entries in lists:
            for entry in markup_entries(entries):
                if "<" in entry:
                    result.append(("generic", entry, os.path.relpath(file, crate_root), key))
                else:
                    result.append(entry_of(crate, key, entry, os.path.relpath(file, crate_root)))
            if "MarkupTyped" not in entries:
                for entry in re.findall(r"(?:^|,)\s*((?:dyn\s+)?[\w:]+)\s*(?=,|$)", entries.strip()):
                    result.append(entry_of(crate, key, entry, os.path.relpath(file, crate_root)))
    return result


def entry_of(crate, key, entry, file):
    is_contract = entry.startswith("dyn")
    name = entry[3:].strip() if is_contract else entry
    return ("contract" if is_contract else "type", entry, file, crate.resolve(key, name.split("::")))


def rust_paths(crate_root):
    """(classes, types, contracts, report): the crate-relative public paths of the registered
    types, sorted, and what has none."""
    crate = Crate(crate_root)
    public = crate.public_paths()
    found = {"class": set(), "type": set(), "contract": set(), "generic": set()}
    report = []
    crate_name = crate_name_of(crate_root)
    for kind, text, file, target in registered_types(crate_root, crate):
        if kind == "generic":
            rendered = render_type(crate, public, target, text, crate_name)
            if rendered is None:
                report.append("%s: generic type %s does not name only public types" % (file, text))
            else:
                found["generic"].add(rendered)
            continue
        if target is None or target[0] != "item":
            report.append("%s: %s %s is not an item this script can resolve" % (file, kind, text))
            continue
        path = public.get(target[1])
        if path is None:
            report.append("%s: %s %s has no public path" % (file, kind, text))
            continue
        found[kind].add("crate::" + "::".join(path))
    return sorted(found["class"]), sorted(found["type"]), sorted(found["contract"]), sorted(found["generic"]), report


def crate_name_of(crate_root):
    """The name of the crate (`package.name` of its manifest, `-` as `_`)."""
    with open(os.path.join(crate_root, "Cargo.toml"), encoding="utf-8") as f:
        name = re.search(r'^name\s*=\s*"([^"]+)"', f.read(), re.M).group(1)
    return name.replace("-", "_")


def rust_paths_file(crate_root, header, macro):
    classes, types, contracts, generics, report = rust_paths(crate_root)

    def listing(paths):
        return "".join("        %s,\n" % path for path in paths)

    text = (
        header
        + "//! The public Rust paths of the registered types of this crate: every class of\n"
        "//! the type table and every non-generic markup type of the type lists, by the\n"
        "//! shortest path another crate names it by (`scripts/rust_paths.py`). The emitter\n"
        "//! of Rust source of the markup compiler reads them; nothing else does.\n\n"
        + macro + "! {\n"
        "    classes: [\n" + listing(classes) + "    ],\n"
        "    types: [\n" + listing(types) + "    ],\n"
        "    contracts: [\n" + listing(contracts) + "    ],\n"
        "    generics: [\n" + "".join("        (%s, %s),\n" % (inner, '"' + outer.lstrip(":") + '"') for inner, outer in generics if not inner.startswith("dyn ")) + "    ],\n"
        "    generic_contracts: [\n" + "".join("        (%s, %s),\n" % (inner[4:], '"' + outer[4:].lstrip(":") + '"') for inner, outer in generics if inner.startswith("dyn ")) + "    ],\n"
        "}\n"
    )
    return text, report
