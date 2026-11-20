#!/usr/bin/env python3
"""Converts the XAML documents of a theme into idiomatic Rust (#988, a prototype).

The converter reads the documents of a theme crate (`--crate src/FerroUI.Themes.WinUI`) and
the type models the build writes for the crates the theme is built on (the `.xamlmeta` files
of their build scripts, found below `--target-dir`), and writes one Rust module per document
into `<crate>/rust_theme/`, over the API of `ferroui_markup_xaml::rust_theme`
(docs/xaml-themes.md, section 10). The output is deterministic: the same documents and models
give the same files, byte for byte (`--check` compares instead of writing, for CI).

    python3 scripts/themes/xaml_to_rust.py --crate src/FerroUI.Themes.WinUI \
        --target-dir "$CARGO_TARGET_DIR" [--check]

What the Rust looks like: a document is a module; its resources are calls of `Resources`
(`resources.add("Key", 8.0)`, `resources.alias(..)`, `resources.control_theme(..)`), a control
theme is a function of its setters and nested styles, a template is a function that builds its
elements with builder chains, and the comments of the document are kept where they were.
"""

from __future__ import annotations

import argparse
import difflib
import glob
import json
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field

NS_X = "http://schemas.microsoft.com/winfx/2006/xaml"
NS_FERRO = "https://github.com/ferroui"

# The crates whose type models the theme documents name.
MODEL_CRATES = [
    "ferroui_base",
    "ferroui_controls",
    "ferroui_markup_xaml",
    "ferroui_dialogs",
    "ferroui_media_video_controls",
    "ferroui_media_camera_controls",
    "ferroui_themes_winui",
]

# The theme variants of the keys of theme dictionaries.
VARIANTS = {
    "Default": "ThemeVariant::default()",
    "Light": "ThemeVariant::light()",
    "Dark": "ThemeVariant::dark()",
    "HighContrast": "crate::WinUITheme::high_contrast()",
}

RUST_KEYWORDS = {
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
}


class Unsupported(Exception):
    """A construct the converter does not convert (yet)."""


class ElementRefLiteral(Exception):
    """A reference to a named element, in place of a literal."""


def snake(name: str) -> str:
    name = name.replace(".", "_").replace("-", "_")
    name = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    name = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name)
    name = re.sub(r"_+", "_", name).strip("_").lower()
    if not name or name[0].isdigit():
        name = "_" + name
    if name in RUST_KEYWORDS:
        name += "_"
    return name


def rust_str(text: str) -> str:
    return json.dumps(text, ensure_ascii=False)


def f64_lit(text: str) -> str:
    value = float(text)
    if value == int(value) and abs(value) < 1e15:
        return f"{int(value)}.0"
    return repr(value)


# --- the type models ---------------------------------------------------------------------------


@dataclass
class Prop:
    """A property of a type, as the converter resolves it."""

    name: str
    kind: str  # styled, attached, direct, plain
    value_type: str
    accessor: str | None = None  # `Owner::name_property()` for a registered property
    getter: str | None = None  # the path of the getter of a plain property
    setter: str | None = None  # the path of the setter of a plain property
    owner: dict | None = None
    read_only: bool = False


class Model:
    def __init__(self, files: list[str]):
        self.by_full: dict[str, dict] = {}
        self.by_path: dict[str, dict] = {}
        self.xmlns: dict[str, list[str]] = {}
        for path in files:
            with open(path, encoding="utf-8") as handle:
                data = json.load(handle)
            for uri, namespace in data.get("xmlns_definitions", []):
                self.xmlns.setdefault(uri, [])
                if namespace not in self.xmlns[uri]:
                    self.xmlns[uri].append(namespace)
            for entry in data.get("types", []):
                full = f"{entry['namespace']}.{entry['name']}"
                previous = self.by_full.get(full)
                if previous is None or (entry.get("object_model") and not previous.get("object_model")):
                    self.by_full[full] = entry
                self.by_path.setdefault(entry["rust_path"], entry)
                for handle in entry.get("handles", []):
                    self.by_path.setdefault(handle, entry)

    def resolve(self, uri: str, name: str) -> dict:
        if uri.startswith("using:"):
            namespaces = [uri[len("using:"):]]
        elif uri.startswith("clr-namespace:"):
            namespaces = [uri[len("clr-namespace:"):].split(";")[0]]
        else:
            namespaces = self.xmlns.get(uri, [])
        for namespace in namespaces:
            found = self.by_full.get(f"{namespace}.{name}")
            if found is not None:
                return found
        raise Unsupported(f"the type {name} of {uri}")

    def base(self, entry: dict) -> dict | None:
        base = entry.get("base")
        if not base:
            return None
        return self.by_path.get(base)

    def chain(self, entry: dict):
        seen = set()
        while entry is not None and id(entry) not in seen:
            seen.add(id(entry))
            yield entry
            entry = self.base(entry)

    def is_a(self, entry: dict, name: str) -> bool:
        return any(c["name"] == name for c in self.chain(entry))

    def find(self, entry: dict, name: str) -> Prop | None:
        for c in self.chain(entry):
            for r in c.get("registered", []):
                if r["name"] == name:
                    return Prop(
                        name=name,
                        kind=r["kind"],
                        value_type=r["value_type"],
                        accessor=f"{path_of(c)}::{r['accessor']}()",
                        owner=c,
                        read_only=r.get("read_only", False),
                    )
            for p in c.get("properties", []):
                if p["name"] == name:
                    getter = p.get("getter", {}).get("call")
                    setter = p.get("setter", {}).get("call") if p.get("setter") else None
                    return Prop(
                        name=name,
                        kind="plain",
                        value_type=p["value_type"],
                        getter=getter["path"] if isinstance(getter, dict) else None,
                        setter=setter["path"] if isinstance(setter, dict) else None,
                        owner=c,
                    )
        return None

    def content_property(self, entry: dict) -> str | None:
        for c in self.chain(entry):
            if c.get("content_property"):
                return c["content_property"]
        return None

    def enum_of(self, value_type: str) -> dict | None:
        entry = self.by_path.get(value_type)
        if entry is not None and entry.get("kind") == "enum":
            return entry
        return None


def path_of(entry: dict) -> str:
    return entry.get("public_path") or entry["rust_path"]


def strip_option(value_type: str) -> tuple[str, bool]:
    if value_type.startswith("Option<") and value_type.endswith(">"):
        return value_type[len("Option<"):-1], True
    return value_type, False


# --- markup extensions -------------------------------------------------------------------------


@dataclass
class Ext:
    name: str
    positional: list
    named: dict


def parse_extension(text: str):
    """`{Name a, b, Key=Value, Key={Nested}}` as an `Ext`; any other text as itself."""
    text = text.strip()
    if not (text.startswith("{") and text.endswith("}")) or text.startswith("{}"):
        return text
    body = text[1:-1].strip()
    match = re.match(r"([\w:.]+)\s*(.*)$", body, re.S)
    if not match:
        return text
    name, rest = match.group(1), match.group(2)
    parts, depth, current, quote = [], 0, "", None
    for char in rest:
        if quote:
            current += char
            if char == quote:
                quote = None
            continue
        if char in "'\"":
            quote = char
            current += char
        elif char == "{":
            depth += 1
            current += char
        elif char == "}":
            depth -= 1
            current += char
        elif char == "," and depth == 0:
            parts.append(current.strip())
            current = ""
        else:
            current += char
    if current.strip():
        parts.append(current.strip())
    positional, named = [], {}
    for part in parts:
        m = re.match(r"^([\w.:()]+)\s*=\s*(.*)$", part, re.S)
        if m and not part.startswith("{"):
            value = m.group(2).strip()
            if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
                value = value[1:-1]
            named[m.group(1)] = parse_extension(value)
        else:
            positional.append(parse_extension(part))
    return Ext(name, positional, named)


# --- the documents -----------------------------------------------------------------------------


class Document:
    def __init__(self, crate: str, relative: str):
        self.relative = relative
        path = os.path.join(crate, relative)
        with open(path, encoding="utf-8") as handle:
            text = handle.read()
        root_tag = re.search(r"<(?!\?|!)[^>]*>", re.sub(r"<!--.*?-->", "", text, flags=re.S), re.S).group(0)
        self.prefixes = {m.group(1) or "": m.group(2) for m in re.finditer(r'xmlns(?::(\w+))?="([^"]*)"', root_tag)}
        builder = ET.TreeBuilder(insert_comments=True)
        parser = ET.XMLParser(target=builder)
        parser.feed(text)
        self.root = parser.close()
        # The comment before the root element: the header of the document.
        header = re.match(r"\s*<!--(.*?)-->", text, re.S)
        self.header = header.group(1) if header else ""

    def uri_of(self, prefix: str) -> str:
        return self.prefixes.get(prefix, "")


def local(tag) -> tuple[str, str]:
    if not isinstance(tag, str):
        return "", ""
    if tag.startswith("{"):
        uri, name = tag[1:].split("}", 1)
        return uri, name
    return "", tag


def is_comment(node) -> bool:
    return node.tag is ET.Comment


def comment_lines(node) -> list[str]:
    text = node.text or ""
    lines = [line.rstrip() for line in text.strip("\n").split("\n")]
    # Removes the common indentation.
    indents = [len(line) - len(line.lstrip()) for line in lines if line.strip()]
    cut = min(indents) if indents else 0
    first = lines[0].strip() if lines else ""
    out = [first] + [line[cut:] if len(line) >= cut else line.strip() for line in lines[1:]]
    while out and not out[-1].strip():
        out.pop()
    return [f"// {line}".rstrip() for line in out if True] or ["//"]


def key_of(node) -> str | None:
    return node.attrib.get(f"{{{NS_X}}}Key")


# --- the output --------------------------------------------------------------------------------


class Code:
    """The lines of a function body, indented by rustfmt afterwards."""

    def __init__(self):
        self.lines: list[str] = []

    def add(self, line: str):
        self.lines.append(line)

    def extend(self, lines):
        self.lines.extend(lines)


@dataclass
class Function:
    name: str
    doc: list[str]
    signature: str
    body: list[str]


@dataclass
class Context:
    """Where a value is made: in a template (the class of its templated parent) or not."""

    target: dict | None  # the class a setter's property is looked up on
    template_parent: dict | None = None  # the class of the templated parent, in a template
    in_template: bool = False
    maker: str = "t"  # what starts an element: the template, or a scope outside templates
    names: dict | None = None  # the classes of the named elements of the template
    scope: str = "t.scope()"  # the scope of static resources, as an expression
    data_type: dict | None = None  # the class of the data context (`x:DataType`)
    property_owner: dict | None = None  # the class a property named by a value is of (transitions)


class Converter:
    def __init__(self, model: Model, crate: str, unit_names: dict[str, int]):
        self.model = model
        self.crate = crate
        self.unit_names = unit_names
        self.unsupported: dict[str, int] = {}
        self.uses: set[str] = set()
        self.functions: list[Function] = []
        self.function_names: set[str] = set()
        self.doc: Document | None = None

    # --- names and types ---

    def fn_name(self, base: str) -> str:
        name = snake(base)
        candidate, index = name, 2
        while candidate in self.function_names:
            candidate = f"{name}_{index}"
            index += 1
        self.function_names.add(candidate)
        return candidate

    def use(self, path: str) -> str:
        """The short name of the public path `path`, imported by the module."""
        if path.startswith("crate::"):
            return path
        if not path.startswith("::"):
            return path
        if path.rsplit("::", 1)[1] in RESERVED_NAMES:
            if path.startswith("::ferroui_themes_winui::"):
                return "crate::" + path[len("::ferroui_themes_winui::"):]
            return path
        self.uses.add(path)
        # A placeholder the rendering replaces by the short name, or by the path when two
        # imported types have the same name.
        return f"\x00{path}\x01"

    def type_of(self, node) -> dict:
        uri, name = local(node.tag)
        return self.model.resolve(uri, name)

    def type_name(self, text: str) -> dict:
        """A type named in an attribute (`TargetType="Button"`, `{x:Type local:Foo}`)."""
        text = text.strip()
        if text.startswith("{"):
            ext = parse_extension(text)
            if isinstance(ext, Ext) and ext.name == "x:Type":
                text = ext.positional[0] if ext.positional else ext.named.get("TypeName")
        prefix, _, name = text.rpartition(":")
        return self.model.resolve(self.doc.uri_of(prefix), name)

    def type_path(self, entry: dict) -> str:
        return self.use(path_of(entry))

    def note(self, what: str):
        self.unsupported[what] = self.unsupported.get(what, 0) + 1

    # --- values ---

    def literal(self, text: str, value_type: str, context: Context | None = None) -> str:
        """`text` as a Rust expression of the type `value_type`."""
        inner, optional = strip_option(value_type)
        expr = self.literal_inner(text, inner, context)
        return f"Some({expr})" if optional else expr

    def literal_inner(self, text: str, vt: str, context) -> str:
        t = text.strip()
        if vt == "f64" or vt == "f32":
            if t in ("NaN", "Infinity", "-Infinity"):
                return {"NaN": f"{vt}::NAN", "Infinity": f"{vt}::INFINITY", "-Infinity": f"{vt}::NEG_INFINITY"}[t]
            return f64_lit(t) if vt == "f64" else f"{f64_lit(t)}_f32"
        if vt in ("i32", "i64", "u32", "usize", "u8", "u16", "i16"):
            return str(int(t))
        if vt == "bool":
            return "true" if t.lower() == "true" else "false"
        if vt == "String":
            return f"String::from({rust_str(text)})"
        if vt == "&'static str":
            return rust_str(text)
        if vt == "::ferroui_base::ferro_property::BoxedValue":
            return f"value({rust_str(text)}.to_string()).expect(\"a value\")"
        entry = self.model.by_path.get(vt)
        if entry is None:
            unwrapped = re.sub(r"^(?:::ferroui_base::type_system::Ref|::std::rc::Rc)<(.*)>$", r"\1", vt)
            entry = self.model.by_path.get(unwrapped)
        name = entry["name"] if entry else vt.rsplit("::", 1)[-1]
        if vt == "&'static ::ferroui_base::ferro_property::FerroProperty":
            owner = context.property_owner if context is not None else None
            prop = self.property(owner, t)
            return f"{self.accessor(prop)}.property()"
        if vt == "char":
            return f"'{t}'"
        if vt.startswith("::ferroui_base::element_ref::ElementRef<"):
            raise ElementRefLiteral(t)
        if entry is not None and entry.get("kind") == "enum":
            return self.enum_value(t, entry)
        if name == "Thickness":
            nums = [f64_lit(x) for x in re.split(r"[,\s]+", t) if x]
            if len(nums) == 1:
                return f"{self.use('::ferroui_base::Thickness')}::uniform({nums[0]})"
            if len(nums) == 2:
                return f"{self.use('::ferroui_base::Thickness')}::symmetric({nums[0]}, {nums[1]})"
            return f"{self.use('::ferroui_base::Thickness')}::new({', '.join(nums)})"
        if name == "CornerRadius":
            nums = [f64_lit(x) for x in re.split(r"[,\s]+", t) if x]
            if len(nums) == 1:
                return f"{self.use('::ferroui_base::CornerRadius')}::uniform({nums[0]})"
            if len(nums) == 2:
                return f"{self.use('::ferroui_base::CornerRadius')}::new({nums[0]}, {nums[1]}, {nums[1]}, {nums[0]})"
            return f"{self.use('::ferroui_base::CornerRadius')}::new({', '.join(nums)})"
        if name == "Color":
            return self.color(t)
        if name == "FontWeight":
            return f"{self.use('::ferroui_base::media::FontWeight')}::{t}" if not t.isdigit() else f"{self.use('::ferroui_base::media::FontWeight')}({t})"
        if name == "ITransform":
            return f"literal({self.use('::ferroui_base::media::transformation::TransformOperations')}::parse({rust_str(t)}))?"
        if name in ("IBrush",):
            return f"literal({self.use('::ferroui_base::media::Brush')}::parse({rust_str(t)}))?"
        if name == "TimeSpan" or vt.endswith("SignedDuration"):
            ms = timespan_ms(t)
            if ms is None:
                return f"literal({self.use('::ferroui_base::animation::SignedDuration')}::parse({rust_str(t)}))?"
            return f"{self.use('::ferroui_base::animation::SignedDuration')}::from_milliseconds({f64_lit(str(ms))})"
        if vt == "::std::time::Duration":
            ms = timespan_ms(t)
            if ms is None or ms < 0:
                raise Unsupported(f"the duration {t}")
            if ms == int(ms):
                return f"std::time::Duration::from_millis({int(ms)})"
            return f"std::time::Duration::from_secs_f64({f64_lit(str(ms / 1000))})"
        if name == "Type" or vt.startswith("&'static ::ferroui_base::type_system::TypeInfo"):
            return f"{self.type_path(self.type_name(t))}::TYPE"
        if entry is not None:
            for m in entry.get("static_properties", []) + entry.get("fields", []):
                if m.get("name", "").lower() == t.lower():
                    expr, _ = self.static_member(f"{entry['namespace']}.{entry['name']}.{m['name']}".replace(entry['namespace'] + ".", "", 1)) if False else (None, None)
                    holder = m.get("getter") if isinstance(m.get("getter"), dict) else m
                    callable_path = (holder.get("callable") or {}).get("path")
                    function = callable_path.rsplit("::", 1)[1] if callable_path else snake(m["name"])
                    return f"{self.qualified(entry)}::{function}()"
            parse = entry.get("parse_call")
            if isinstance(parse, dict) and parse.get("path"):
                path = parse["path"]
                return f"literal({self.use_call(path)}({rust_str(t)}))?"
            parse = entry.get("parse")
            if isinstance(parse, dict) and "::" in parse.get("path", "") and parse["path"].split("::")[-2][:1].isupper():
                function = parse["path"].rsplit("::", 1)[1]
                return f"literal({self.qualified(entry)}::{function}({rust_str(t)}))?"
            if isinstance(parse, dict) and parse.get("path"):
                # A parser of a module that is not public: the parse function of the markup metadata.
                return f"literal({self.qualified(entry)}::__markup_parse(String::from({rust_str(t)})))?"
        raise Unsupported(f"a literal of {vt}")

    def use_call(self, path: str) -> str:
        """`::a::b::Type::function` with the type imported: `Type::function`."""
        owner, function = path.rsplit("::", 1)
        return f"{self.use(owner)}::{function}"

    def color(self, text: str) -> str:
        color = self.use("::ferroui_base::media::Color")
        if text.startswith("#"):
            digits = text[1:]
            if len(digits) == 6:
                digits = "FF" + digits
            elif len(digits) == 3:
                digits = "FF" + "".join(c * 2 for c in digits)
            elif len(digits) == 4:
                digits = "".join(c * 2 for c in digits)
            return f"{color}::from_uint32(0x{digits.upper()})"
        return f"{self.use('::ferroui_base::media::Colors')}::{snake(text).upper()}"

    def enum_value(self, text: str, entry: dict) -> str:
        members = {
            m["name"].lower(): m["rust_variant"] if "rust_variant" in m else m["rust_value"].split("::")[-1]
            for m in entry.get("enum_members", [])
        }
        parts = [p.strip() for p in re.split(r"[,|]", text) if p.strip()]
        path = self.type_path(entry)
        if len(parts) == 1:
            variant = members.get(parts[0].lower())
            if variant is None:
                raise Unsupported(f"the member {text} of {entry['name']}")
            return f"{path}::{variant}"
        values = []
        for part in parts:
            variant = members.get(part.lower())
            if variant is None:
                raise Unsupported(f"the member {part} of {entry['name']}")
            values.append(f"{path}::{variant}")
        return " | ".join(values)

    def static_member(self, text: str) -> tuple[str, str | None]:
        """`{x:Static Type.Member}`: the expression and, when known, its type."""
        prefix_name, _, member = text.rpartition(".")
        prefix, _, name = prefix_name.rpartition(":")
        entry = self.model.resolve(self.doc.uri_of(prefix), name)
        if entry.get("kind") == "enum":
            return self.enum_value(member, entry), entry["rust_path"]
        for m in entry.get("fields", []) + entry.get("static_properties", []):
            if m.get("name") == member:
                holder = m.get("getter") if isinstance(m.get("getter"), dict) else m
                call = holder.get("call")
                value_type = m.get("value_type") or m.get("return_type")
                if isinstance(call, dict) and call.get("path"):
                    return f"{self.use_call(call['path'])}()", value_type
                callable_path = (holder.get("callable") or {}).get("path")
                if callable_path:
                    return f"{self.qualified(entry)}::{callable_path.rsplit('::', 1)[1]}()", value_type
                return f"{self.qualified(entry)}::{snake(member)}()", value_type
        for m in entry.get("properties", []):
            if m.get("name") == member and m.get("is_static"):
                call = m.get("getter", {}).get("call")
                if isinstance(call, dict) and call.get("path"):
                    return f"{self.use_call(call['path'])}()", m.get("value_type")
        return f"{self.qualified(entry)}::{snake(member)}()", None

    def qualified(self, entry: dict) -> str:
        """The path of a type to call an associated function of: `<dyn T>` for a trait."""
        path = self.type_path(entry)
        return f"<dyn {path}>" if entry["rust_path"].startswith("dyn ") else path

    def shared_handle(self, entry: dict) -> bool:
        """Whether the class is held as `Rc<Self>` (a converter, a transition of markup)."""
        if entry.get("this", "").startswith("::std::rc::Rc<"):
            return True
        return f"::std::rc::Rc<{entry['rust_path']}>" in entry.get("handles", [])

    def constructor(self, entry: dict) -> str:
        """The call of the default constructor of a class outside the object model."""
        for constructor in entry.get("constructors", []):
            if constructor.get("parameters"):
                continue
            call = constructor.get("call")
            if isinstance(call, dict) and call.get("path"):
                return f"{self.use_call(call['path'])}()"
            callable_path = constructor.get("callable", {}).get("path")
            if callable_path:
                function = callable_path.rsplit("::", 1)[1]
                made = f"{self.qualified(entry)}::{function}()"
                if self.shared_handle(entry) and function == "new":
                    return f"{self.use('::std::rc::Rc')}::new({made})"
                return made
        made = f"{self.qualified(entry)}::new()"
        if self.shared_handle(entry):
            return f"{self.use('::std::rc::Rc')}::new({made})"
        return made

    def constructor_value(self, node) -> str:
        """An object made by its constructor alone, as a value (not a handle)."""
        entry = self.type_of(node)
        return f"{self.qualified(entry)}::new()"

    # --- properties ---

    def property(self, target: dict | None, name: str) -> Prop:
        name = name.strip()
        if name.startswith("(") and name.endswith(")"):
            name = name[1:-1]
        if "." in name:
            owner_name, prop_name = name.split(".", 1)
            prefix, _, owner_name = owner_name.rpartition(":")
            owner = self.model.resolve(self.doc.uri_of(prefix), owner_name)
            found = self.model.find(owner, prop_name)
            if found is None:
                raise Unsupported(f"the property {name}")
            return found
        name = name.strip("()")
        if target is not None:
            found = self.model.find(target, name)
            if found is not None:
                return found
        control = self.model.by_full.get("FerroUI.Controls.Control")
        found = self.model.find(control, name)
        if found is not None:
            return found
        raise Unsupported(f"the property {name} of {target['name'] if target else '?'}")

    def accessor(self, prop: Prop) -> str:
        owner, accessor = prop.accessor.rsplit("::", 1)
        return f"{self.use(owner)}::{accessor}"

    def untyped(self, prop: Prop) -> str:
        return f"{self.accessor(prop)}.untyped()"

    # --- selectors ---

    def selector(self, text: str, parent_target: dict | None) -> tuple[str, dict | None]:
        """The selector `text` as a builder chain, and the class its last compound names."""
        alternatives = split_top(text, ",")
        if len(alternatives) > 1:
            built = [self.selector(a, parent_target) for a in alternatives]
            targets = {id(t): t for _, t in built if t is not None}
            target = built[0][1] if len(targets) == 1 else parent_target
            items = ", ".join(b for b, _ in built)
            return f"{self.use('::ferroui_base::styling::Selectors')}::or([{items}])", target
        return self.selector_chain(text.strip(), parent_target)

    def selector_chain(self, text: str, parent_target: dict | None) -> tuple[str, dict | None]:
        tokens = tokenize_selector(text)
        expr = None
        target = None
        nesting_target = parent_target

        def chain(call: str, start: str):
            nonlocal expr
            expr = start if expr is None else f"{expr}.{call}"

        for kind, value in tokens:
            if kind == "nesting":
                chain("nesting()", "nesting()")
                target = nesting_target
            elif kind == "type":
                entry = self.type_name(value.replace("|", ":"))
                p = self.type_path(entry)
                chain(f"of::<{p}>()", f"of::<{p}>()")
                target = entry
            elif kind == "is":
                entry = self.type_name(value.replace("|", ":"))
                p = self.type_path(entry)
                chain(f"is::<{p}>()", f"{self.use('::ferroui_markup_xaml::rust_theme::is')}::<{p}>()")
                target = entry
            elif kind == "class":
                chain(f"class({rust_str(value)})", f"{self.use('::ferroui_markup_xaml::rust_theme::class')}({rust_str(value)})")
            elif kind == "name":
                chain(f"name({rust_str(value)})", f"{self.use('::ferroui_markup_xaml::rust_theme::named')}({rust_str(value)})")
            elif kind == "child":
                chain("child()", "?")
                target = None
            elif kind == "descendant":
                chain("descendant()", "?")
                target = None
            elif kind == "template":
                chain("template()", "?")
                target = None
            elif kind == "not":
                inner, _ = self.selector(value, target or parent_target)
                inner = inner
                chain(f"not({inner})", "?")
            elif kind == "nth-child" or kind == "nth-last-child":
                step, offset = nth(value)
                method = "nth_child" if kind == "nth-child" else "nth_last_child"
                chain(f"{method}({step}, {offset})", "?")
            elif kind == "property":
                pname, pvalue = value
                on = target or parent_target
                prop = self.property(on, pname)
                if prop.kind == "plain":
                    raise Unsupported("a selector on a plain property")
                literal = self.literal(pvalue, prop.value_type)
                chain(f"property_is({self.accessor(prop)}, {literal})", "?")
            else:
                raise Unsupported(f"the selector token {kind}")
        if expr is None or "?" in expr:
            raise Unsupported(f"the selector {text}")
        return expr, target

    # --- documents ---

    def convert(self, doc: Document) -> str:
        self.doc = doc
        self.uses = set()
        self.functions = []
        self.function_names = {"merge", "build", "populate"}
        root = doc.root
        uri, name = local(root.tag)
        body = Code()
        if name == "ResourceDictionary":
            self.dictionary_entries(root, body, "resources", Context(None))
            entry_fn = Function(
                "merge",
                ["/// Adds the resources of the document to `resources`."],
                "pub(crate) fn merge(resources: &Resources) -> Result",
                body.lines + ["Ok(())"],
            )
            dictionary = self.use("::ferroui_base::controls::ResourceDictionary")
            self.functions.insert(0, Function(
                "build",
                ["/// The resource dictionary of the document, declared in `scope`."],
                f"pub(crate) fn build(scope: &Scope) -> Result<Ref<{dictionary}>>",
                [f"let dictionary = {dictionary}::new();", "merge(&Resources::new(&dictionary, scope))?;", "Ok(dictionary)"],
            ))
        elif name == "Styles":
            self.styles_root(root, body)
            styles_type = self.use("::ferroui_base::styling::Styles")
            entry_fn = Function(
                "populate",
                ["/// Populates `styles` (and its resources) from the document, declared in `scope`."],
                f"pub(crate) fn populate(styles: &Ref<{styles_type}>, scope: &Scope) -> Result",
                ["let scope = &scope.with_styles(styles);"] + body.lines + ["Ok(())"],
            )
            self.functions.insert(0, Function(
                "build",
                ["/// The styles of the document, declared in `scope`."],
                f"pub(crate) fn build(scope: &Scope) -> Result<Ref<{styles_type}>>",
                [f"let styles = {styles_type}::new();", "populate(&styles, scope)?;", "Ok(styles)"],
            ))
        else:
            raise Unsupported(f"a document whose root is {name}")
        return self.render(doc, [entry_fn] + self.functions)

    def render(self, doc: Document, functions: list[Function]) -> str:
        out = [f"//! `{doc.relative}`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988)."]
        out.append("//!")
        out.append("//! Generated: change the document or the converter and run the converter again")
        out.append("//! (docs/xaml-themes.md, section 10). The header of the document:")
        out.append("//!")
        for line in header_lines(doc.header):
            out.append(f"//! {line}".rstrip())
        out.append("")
        out.append("#![allow(unused_imports, clippy::all)]")
        out.append("")
        out.append("use crate::rust_theme::prelude::*;")
        names: dict[str, list[str]] = {}
        for path in sorted(self.uses):
            names.setdefault(path.rsplit("::", 1)[1], []).append(path)
        imports = []
        replacements = {}
        for short, paths in sorted(names.items()):
            if len(paths) == 1:
                imports.append(paths[0].lstrip(":"))
                replacements[paths[0]] = short
            else:
                for path in paths:
                    replacements[path] = ("crate::" + path[len("::ferroui_themes_winui::"):]) if path.startswith("::ferroui_themes_winui::") else path
        for path in sorted(imports):
            if path.split("::")[-1] in PRELUDE_NAMES:
                continue
            if path.startswith("ferroui_themes_winui::"):
                path = "crate::" + path[len("ferroui_themes_winui::"):]
            out.append(f"use {path};")
        for function in functions:
            out.append("")
            out.extend(function.doc)
            out.append(function.signature + " {")
            out.extend(function.body)
            out.append("}")
        text = "\n".join(out) + "\n"
        return re.sub("\x00([^\x01]*)\x01", lambda m: replacements[m.group(1)], text)

    # --- resource dictionaries ---

    def dictionary_entries(self, node, body: Code, target: str, context: Context):
        """The entries of the dictionary element `node`, added through the `Resources` named
        `target`."""
        for child in list(node):
            if is_comment(child):
                body.extend(comment_lines(child))
                continue
            uri, name = local(child.tag)
            if name == "ResourceDictionary.ThemeDictionaries":
                for theme in list(child):
                    if is_comment(theme):
                        body.extend(comment_lines(theme))
                        continue
                    variant = key_of(theme)
                    ext = parse_extension(variant)
                    if isinstance(ext, Ext) and ext.name == "x:Static":
                        variant_expr, _ = self.static_member(ext.positional[0])
                        variant = ext.positional[0].rsplit(".", 1)[-1]
                    else:
                        variant_expr = VARIANTS[variant]
                    function = self.fn_name(f"{variant}_theme")
                    inner = Code()
                    self.dictionary_entries(theme, inner, "resources", context)
                    self.functions.append(
                        Function(
                            function,
                            [f"/// The `{variant}` theme dictionary."],
                            f"fn {function}(resources: &Resources) -> Result",
                            inner.lines + ["Ok(())"],
                        )
                    )
                    body.add(f"{target}.theme({variant_expr}, {function})?;")
                continue
            if name == "ResourceDictionary.MergedDictionaries":
                for merged in list(child):
                    if is_comment(merged):
                        body.extend(comment_lines(merged))
                        continue
                    self.merged(merged, body, target)
                continue
            if name.startswith("Design."):
                continue
            if "." in name:
                raise Unsupported(f"the property element {name} of a dictionary")
            try:
                self.resource(child, body, target, context)
            except (Unsupported, IndexError, KeyError, TypeError, ValueError) as error:
                if not isinstance(error, Unsupported):
                    import traceback
                    traceback.print_exc()
                self.note(str(error))
                body.add(f"// UNSUPPORTED: {error}")

    def merged(self, node, body: Code, target: str):
        uri, name = local(node.tag)
        if name == "MergeResourceInclude":
            source = node.attrib["Source"]
            document = source.split("FerroUI.Themes.WinUI", 1)[-1].lstrip("/")
            index = self.unit_names.get(document)
            if index is None:
                raise Unsupported(f"the include {source}")
            if index == 0:
                # A document of the ambient unit: always merged.
                body.add(f"crate::rust_theme::{module_of(document)}::merge(&{target})?;")
            else:
                body.add(f"{target}.include({index})?; // {document}")
            return
        entry = self.type_of(node)
        body.add(f"{target}.merge({self.type_path(entry)}::new());")

    def resource(self, node, body: Code, target: str, context: Context):
        uri, name = local(node.tag)
        key = key_of(node)
        text = (node.text or "").strip()
        if uri == NS_X:
            if name == "Double":
                body.add(f"{target}.add({rust_str(key)}, {f64_lit(text)});")
                return
            if name == "String":
                body.add(f"{target}.text({rust_str(key)}, {rust_str(node.text or '')});")
                return
            if name == "Boolean":
                body.add(f"{target}.add({rust_str(key)}, {'true' if text.lower() == 'true' else 'false'});")
                return
            if name == "Int32":
                body.add(f"{target}.add({rust_str(key)}, {int(text)}_i32);")
                return
            raise Unsupported(f"the resource x:{name}")
        if name == "StaticResource":
            body.add(f"{target}.alias({rust_str(key)}, {rust_str(node.attrib['ResourceKey'])});")
            return
        if name == "ControlTheme":
            self.control_theme(node, body, target)
            return
        if name == "ResourceInclude":
            source = node.attrib["Source"].lstrip("/")
            module = module_of(source)
            body.add(f"{target}.deferred({rust_str(key)}, |scope| Ok(value(crate::rust_theme::{module}::build(scope)?)));")
            return
        entry = self.type_of(node)
        if key is None:
            raise Unsupported(f"a resource without a key ({name})")
        if name == "SolidColorBrush" and set(node.attrib) - {f"{{{NS_X}}}Key"} == {"Color"} and len(node) == 0:
            color = node.attrib["Color"]
            ext = parse_extension(color)
            if isinstance(ext, Ext) and ext.name == "StaticResource":
                body.add(f"{target}.static_color_brush({rust_str(key)}, {rust_str(ext.positional[0])});")
            elif isinstance(ext, Ext) and ext.name == "DynamicResource":
                body.add(f"{target}.dynamic_color_brush({rust_str(key)}, {rust_str(ext.positional[0])});")
            else:
                digits = self.color(color)
                if "from_uint32(" in digits:
                    body.add(f"{target}.color_brush({rust_str(key)}, {digits.split('from_uint32(')[1][:-1]});")
                else:
                    body.add(f"{target}.deferred({rust_str(key)}, |_| Ok(value({self.use('::ferroui_base::media::SolidColorBrush')}::with_color({digits}))));")
            return
        if not node.attrib.keys() - {f"{{{NS_X}}}Key"} and len(node) == 0 and text and entry.get("kind") not in ("struct", "enum") and self.parses(entry):
            body.add(f"{target}.deferred({rust_str(key)}, |_| Ok(value({self.parsed(entry, text)})));")
            return
        if not node.attrib.keys() - {f"{{{NS_X}}}Key"} and len(node) == 0 and text and entry.get("kind") in ("struct", "enum"):
            literal = self.literal(text, entry["rust_path"])
            if "?" in literal:
                body.add(f"{target}.deferred({rust_str(key)}, |_| Ok(value({literal})));")
            else:
                body.add(f"{target}.add({rust_str(key)}, {literal});")
            return
        # Any other object: made when it is first looked up.
        if name == "ControlTemplate":
            scoped = Context(None, maker="scope", scope="scope")
            template = self.template_value(node, scoped, None, key)
            body.add(f"{target}.deferred({rust_str(key)}, |scope| Ok(value({template})));")
            return
        if name in ("DataTemplate", "ItemsPanelTemplate"):
            raise Unsupported(f"a {name} resource")
        expr = self.object(node, Context(None, maker="scope", scope="scope"), "scope")
        body.add(f"{target}.deferred({rust_str(key)}, |scope| Ok(value({expr})));")

    # --- control themes and styles ---

    def control_theme(self, node, body: Code, target: str):
        key = key_of(node)
        target_type = self.type_name(node.attrib["TargetType"]) if "TargetType" in node.attrib else None
        if key is None or key.startswith("{"):
            type_entry = self.type_name(key) if key else target_type
            key_expr = f"type_key::<{self.type_path(type_entry)}>()"
            self.use("::ferroui_markup_xaml::rust_theme::type_key")
            base_name = f"{type_entry['name']}_theme"
            title = f"The default control theme of `{type_entry['name']}`."
        else:
            key_expr = rust_str(key)
            base_name = key
            title = f"`{key}`: a control theme of `{target_type['name'] if target_type else '?'}`."
        function = self.fn_name(base_name)
        inner = Code()
        if target_type is not None:
            inner.add(f"theme.target::<{self.type_path(target_type)}>();")
        if "BasedOn" in node.attrib:
            ext = parse_extension(node.attrib["BasedOn"])
            based = ext.positional[0]
            if isinstance(based, Ext) and based.name == "x:Type":
                inner.add(f"theme.based_on(type_key::<{self.type_path(self.type_name(based.positional[0]))}>())?;")
                self.use("::ferroui_markup_xaml::rust_theme::type_key")
            else:
                inner.add(f"theme.based_on({rust_str(based)})?;")
        context = Context(target_type, template_parent=target_type)
        self.style_members(node, inner, "theme", context, function)
        self.functions.append(
            Function(function, [f"/// {title}"], f"fn {function}(theme: &StyleBuilder) -> Result", inner.lines + ["Ok(())"])
        )
        body.add(f"{target}.control_theme({key_expr}, {function});")

    def style_members(self, node, body: Code, style: str, context: Context, owner_fn: str):
        for child in list(node):
            if is_comment(child):
                body.extend(comment_lines(child))
                continue
            uri, name = local(child.tag)
            try:
                if name == "Setter":
                    self.setter(child, body, style, context, owner_fn)
                elif name == "Style":
                    self.nested_style(child, body, style, context, owner_fn)
                elif name in ("Style.Animations", "ControlTheme.Animations"):
                    for animation in list(child):
                        if is_comment(animation):
                            body.extend(comment_lines(animation))
                            continue
                        expr = self.object(animation, Context(context.target, template_parent=context.template_parent, property_owner=context.target), f"{style}.scope()")
                        body.add(f"{style}.animation({expr});")
                elif name in ("ControlTheme.Resources", "Style.Resources"):
                    raise Unsupported("the resources of a style")
                else:
                    raise Unsupported(f"the member {name} of a style")
            except Unsupported as error:
                self.note(str(error))
                body.add(f"// UNSUPPORTED: {error}")

    def nested_style(self, node, body: Code, style: str, context: Context, owner_fn: str):
        selector_text = node.attrib.get("Selector")
        selector, target = self.selector(selector_text, context.target)
        inner = Code()
        nested_context = Context(target or context.target, template_parent=context.template_parent)
        self.style_members(node, inner, "style", nested_context, owner_fn)
        body.add(f"{style}.style({selector}, |style| {{")
        body.extend(inner.lines)
        body.add("Ok(())")
        body.add("})?;")

    def setter(self, node, body: Code, style: str, context: Context, owner_fn: str):
        prop = self.property(context.target, node.attrib["Property"])
        if prop.kind == "plain":
            raise Unsupported(f"a setter of the plain property {prop.name}")
        data_type = self.data_type_of(node) or context.data_type
        context = Context(context.target, template_parent=context.template_parent, in_template=context.in_template,
                          maker=context.maker, names=context.names, scope=context.scope, data_type=data_type,
                          property_owner=context.property_owner)
        value_text = node.attrib.get("Value")
        children = [c for c in list(node) if not is_comment(c)]
        if value_text is None:
            if len(children) == 1 and local(children[0].tag)[1] == "Setter.Value":
                children = [c for c in list(children[0]) if not is_comment(c)]
            if len(children) != 1:
                raise Unsupported("a setter without a value")
            value_node = children[0]
            vuri, vname = local(value_node.tag)
            if vname in ("Binding", "MultiBinding", "CompiledBinding", "ReflectionBinding", "TemplateBinding", "DynamicResource"):
                scoped = Context(context.target, template_parent=context.template_parent, scope=f"{style}.scope()", maker=f"{style}.scope()",
                                 data_type=context.data_type, names=context.names)
                body.add(f"{style}.bind({self.accessor(prop)}, {self.plain_object(value_node, scoped)});")
                return
            if vname == "ControlTemplate":
                function = self.template(value_node, context, owner_fn)
                body.add(f"{style}.template({function});")
                return
            scoped = Context(context.target, template_parent=context.template_parent, scope=f"{style}.scope()", maker=f"{style}.scope()",
                             data_type=context.data_type, names=context.names)
            if vname == "Template" and "ITemplate" in prop.value_type:
                function = self.adorner_template(value_node, context, owner_fn)
                maker = "focus_adorner" if "ITemplateOf<Option<" in prop.value_type else "template_of"
                body.add(f"{style}.set({self.accessor(prop)}, {self.optional(self.use('::ferroui_markup_xaml::rust_theme::' + maker) + '(' + style + '.scope(), ' + function + ')', prop)});")
                return
            if vname == "WindowDrawnDecorationsTemplate":
                content = [c for c in list(value_node) if not is_comment(c)][0]
                parent = context.template_parent or self.model.by_full["FerroUI.Controls.Control"]
                function = self.fn_name(f"{owner_fn}_decorations")
                names = {}
                collect_names(self, content, names)
                inner = Code()
                decorations_context = Context(parent, template_parent=parent, in_template=True, maker="t", names=names)
                expr = self.element(content, decorations_context, "t")
                inner.add(f"Ok({expr}.end())")
                decorations = self.use("::ferroui_controls::chrome::WindowDrawnDecorationsContent")
                self.functions.append(Function(function, [f"/// The decorations of `{owner_fn}`."],
                    f"fn {function}(t: &Template<{self.type_path(parent)}>) -> Result<Ref<{decorations}>>", inner.lines))
                body.add(f"{style}.set({self.accessor(prop)}, {self.optional(self.use('::ferroui_markup_xaml::rust_theme::decorations_template') + '(' + style + '.scope(), ' + function + ')', prop)});")
                return
            if vname == "Template":
                function = self.value_template(value_node, prop, owner_fn, context.data_type)
                body.add(f"{style}.set_template({self.accessor(prop)}, {function});")
                return
            template = self.template_value(value_node, scoped, prop, owner_fn)
            if template is not None:
                body.add(f"{style}.set({self.accessor(prop)}, {self.optional(template, prop)});")
                return
            scoped.property_owner = context.target
            expr = self.object(value_node, scoped, f"{style}.scope()")
            body.add(f"{style}.set_value({self.accessor(prop)}, value({expr}));")
            return
        ext = parse_extension(value_text)
        if isinstance(ext, Ext):
            if ext.name == "DynamicResource":
                body.add(f"{style}.dynamic({self.accessor(prop)}, {rust_str(ext.positional[0])});")
            elif ext.name == "StaticResource":
                body.add(f"{style}.static_resource({self.accessor(prop)}, {rust_str(ext.positional[0])})?;")
            elif ext.name == "x:Null":
                body.add(f"{style}.set_null({self.accessor(prop)});")
            elif ext.name == "x:Static":
                expr, static_type = self.static_member(ext.positional[0])
                body.add(f"{style}.set({self.accessor(prop)}, {self.optional(expr, prop, static_type)});")
            elif ext.name == "x:Type":
                entry = self.type_name(value_text)
                body.add(f"{style}.set({self.accessor(prop)}, Some({self.type_path(entry)}::TYPE));")
            elif ext.name in ("Binding", "CompiledBinding", "ReflectionBinding"):
                scoped = Context(context.target, template_parent=context.template_parent, scope=f"{style}.scope()", maker=f"{style}.scope()",
                                 data_type=context.data_type, names=context.names)
                binding = self.binding(ext, scoped, None)
                body.add(f"{style}.bind({self.accessor(prop)}, {binding});")
            elif ext.name == "TemplateBinding":
                source_name = ext.positional[0] if ext.positional else ext.named.get("Property")
                if source_name is None:
                    raise Unsupported("a template binding of the templated parent in a setter")
                source = self.property(context.template_parent, source_name)
                extra = ext.named.keys() - {"Property"}
                if not extra:
                    body.add(f"{style}.template_bind({self.accessor(prop)}, {self.accessor(source)});")
                else:
                    binding = f"{self.use('::ferroui_base::data::TemplateBinding')}::new({self.accessor(source)}.property())"
                    scoped = Context(context.target, scope=f"{style}.scope()")
                    for key in sorted(extra):
                        value_ext = ext.named[key]
                        if key == "Converter":
                            binding += f".with_converter_value({self.converter(value_ext, scoped)})"
                        elif key == "Mode":
                            binding += f".with_mode({self.use('::ferroui_base::data::BindingMode')}::{value_ext})"
                        elif key == "ConverterParameter":
                            binding += f".with_converter_parameter(value({rust_str(value_ext)}.to_string()))"
                        else:
                            raise Unsupported(f"the option {key} of a template binding")
                    body.add(f"{style}.bind({self.accessor(prop)}, {binding});")
            else:
                raise Unsupported(f"the markup extension {ext.name} in a setter")
            return
        literal = self.literal(value_text, prop.value_type, context)
        body.add(f"{style}.set({self.accessor(prop)}, {literal});")

    def optional(self, expr: str, prop: Prop, value_type: str | None = None) -> str:
        _, optional = strip_option(prop.value_type)
        if value_type is not None and value_type.startswith("Option<"):
            return expr
        return f"Some({expr})" if optional else expr

    def styles_root(self, root, body: Code):
        for child in list(root):
            if is_comment(child):
                body.extend(comment_lines(child))
                continue
            uri, name = local(child.tag)
            if name == "Styles.Resources":
                dictionaries = [c for c in list(child) if not is_comment(c)]
                for c in list(child):
                    if is_comment(c):
                        body.extend(comment_lines(c))
                body.add("let resources = Resources::new(&styles.resources(), scope);")
                for dictionary in dictionaries:
                    self.dictionary_entries(dictionary, body, "resources", Context(None))
            elif name == "StyleInclude":
                source = child.attrib["Source"].split("FerroUI.Themes.WinUI", 1)[-1].lstrip("/")
                module = module_of(source)
                body.add(f"styles.add(crate::rust_theme::{module}::build(scope)?);")
            elif name == "Style":
                selector, target = self.selector(child.attrib["Selector"], None)
                inner = Code()
                context = Context(target, scope="style.scope()", maker="style.scope()")
                self.style_members(child, inner, "style", context, "style")
                body.add(f"styles.add({self.use('::ferroui_markup_xaml::rust_theme::style')}(scope, {selector}, |style| {{")
                body.extend(inner.lines)
                body.add("Ok(())")
                body.add("})?);")
            else:
                raise Unsupported(f"the member {name} of a styles document")

    # --- templates ---

    def template_function(self, node, target: dict, owner_fn: str, suffix: str, doc: str, returns: str, end: str,
                          data_type: dict | None = None) -> str:
        function = self.fn_name(f"{owner_fn}_{suffix}")
        content = [c for c in list(node) if not is_comment(c)]
        body = Code()
        for comment in [c for c in list(node) if is_comment(c)]:
            body.extend(comment_lines(comment))
        if len(content) != 1:
            raise Unsupported("a template without one root")
        names = {}
        collect_names(self, content[0], names)
        data_type = self.data_type_of(node) or data_type
        template_context = Context(target, template_parent=target, in_template=True, maker="t", names=names, data_type=data_type)
        expr = self.element(content[0], template_context, "t")
        body.add(f"Ok({expr}{end})")
        self.functions.append(
            Function(function, [doc], f"fn {function}(t: &Template<{self.type_path(target)}>) -> Result<Ref<{returns}>>", body.lines)
        )
        return function

    def template(self, node, context: Context, owner_fn: str) -> str:
        target = self.type_name(node.attrib["TargetType"]) if "TargetType" in node.attrib else context.template_parent
        if target is None:
            target = self.model.by_full["FerroUI.Controls.Primitives.TemplatedControl"]
        control = self.use("::ferroui_controls::Control")
        return self.template_function(
            node, target, owner_fn, "template", f"/// The template of `{target['name']}` of `{owner_fn}`.", control, ".control()",
            context.data_type,
        )

    def adorner_template(self, node, context: Context, owner_fn: str) -> str:
        control_entry = self.model.by_full["FerroUI.Controls.Control"]
        control = self.use("::ferroui_controls::Control")
        return self.template_function(
            node, control_entry, owner_fn, "focus_adorner", f"/// The focus adorner of `{owner_fn}`.", control, ".control()"
        )

    def panel_template(self, node, context: Context, owner_fn: str) -> str:
        control_entry = self.model.by_full["FerroUI.Controls.Control"]
        panel = self.use("::ferroui_controls::Panel")
        return self.template_function(
            node, control_entry, owner_fn, "items_panel", f"/// The items panel of `{owner_fn}`.", panel, ".end().upcast()"
        )

    def template_value(self, node, context: Context, prop: Prop, owner_fn: str) -> str | None:
        """A template as the value of a property: `(expression, is_markup_value)`."""
        _, vname = local(node.tag)
        if vname == "ControlTemplate":
            target = self.type_name(node.attrib["TargetType"]) if "TargetType" in node.attrib else context.template_parent
            if target is None:
                target = self.model.by_full["FerroUI.Controls.Primitives.TemplatedControl"]
            function = self.template(node, context, owner_fn)
            return f"{self.use('::ferroui_markup_xaml::rust_theme::control_template')}({context.scope}, {function})"
        if vname == "FocusAdornerTemplate":
            function = self.adorner_template(node, context, owner_fn)
            return f"{self.use('::ferroui_markup_xaml::rust_theme::focus_adorner')}({context.scope}, {function})"
        if vname == "ItemsPanelTemplate":
            function = self.panel_template(node, context, owner_fn)
            return f"{self.use('::ferroui_markup_xaml::rust_theme::items_panel')}({context.scope}, {function})"
        if vname == "DataTemplate":
            control_entry = self.model.by_full["FerroUI.Controls.Control"]
            control = self.use("::ferroui_controls::Control")
            if node.attrib.get("DataType") == "x:String":
                function = self.template_function(
                    node, control_entry, owner_fn, "data_template", f"/// A data template of `{owner_fn}` for text.", control, ".control()",
                    None,
                )
                return f"{self.use('::ferroui_markup_xaml::rust_theme::data_template_for')}::<String>({context.scope}, {function})"
            typed = self.type_name(node.attrib["DataType"]) if "DataType" in node.attrib else None
            function = self.template_function(
                node, control_entry, owner_fn, "data_template", f"/// A data template of `{owner_fn}`.", control, ".control()",
                typed or context.data_type,
            )
            if typed is not None:
                return f"{self.use('::ferroui_markup_xaml::rust_theme::data_template_of')}::<{self.markup_type(typed)}>({context.scope}, {function})"
            return f"{self.use('::ferroui_markup_xaml::rust_theme::data_template')}({context.scope}, {function})"
        return None

    def value_template(self, node, prop: Prop, owner_fn: str, data_type) -> str:
        """A `<Template>` of the value of a setter: made for each control the setter applies to."""
        control_entry = self.model.by_full["FerroUI.Controls.Control"]
        inner, optional = strip_option(prop.value_type)
        if inner.endswith("BoxedValue"):
            end, returns = ".end())", "Option<ferroui_base::BoxedValue>"
            prefix = "value("
        else:
            end, returns = ".end().upcast())" if optional else ".end().upcast()", self.value_type(prop.value_type)
            prefix = "Some(" if optional else ""
        function = self.fn_name(f"{owner_fn}_{snake(prop.name)}_template")
        content = [c for c in list(node) if not is_comment(c)]
        body = Code()
        for comment in [c for c in list(node) if is_comment(c)]:
            body.extend(comment_lines(comment))
        names = {}
        collect_names(self, content[0], names)
        context = Context(control_entry, template_parent=control_entry, in_template=True, maker="t", names=names,
                          data_type=self.data_type_of(node) or data_type)
        expr = self.element(content[0], context, "t")
        body.add(f"Ok({prefix}{expr}{end})")
        self.functions.append(Function(
            function, [f"/// The value of `{prop.name}` of `{owner_fn}`, made for each control."],
            f"fn {function}(t: &Template<{self.use('::ferroui_controls::Control')}>) -> Result<{returns}>", body.lines))
        return function

    def data_type_of(self, node) -> dict | None:
        text = node.attrib.get(f"{{{NS_X}}}DataType")
        if not text:
            return None
        try:
            return self.type_name(text)
        except Unsupported:
            return None

    # --- elements and objects ---

    def object(self, node, context: Context, scope: str) -> str:
        """An object made outside a template (a resource or the value of a setter), finished."""
        entry = self.type_of(node)
        local_context = Context(entry, template_parent=context.template_parent, in_template=False, maker=scope, names={},
                                scope=scope, property_owner=context.property_owner, data_type=context.data_type)
        text = (node.text or "").strip()
        attributes = [k for k in node.attrib if local(k)[0] != NS_X]
        if text and not attributes and not [c for c in list(node) if not is_comment(c)] and self.parses(entry):
            return self.parsed(entry, text)
        if entry.get("object_model"):
            built = self.element_with(node, local_context, f"{scope}.make({self.type_path(entry)}::new())", entry)
            return f"{built}.end()"
        return self.plain_object(node, local_context)

    def element(self, node, context: Context, maker: str) -> str:
        """An element (of an object-model class), started by `maker`, not finished."""
        entry = self.type_of(node)
        if not entry.get("object_model"):
            raise Unsupported(f"the plain object {entry['name']} as an element")
        return self.element_with(node, context, f"{maker}.make({self.type_path(entry)}::new())", entry)

    def element_with(self, node, context: Context, start: str, entry: dict) -> str:
        calls = [start]
        data_type = context.data_type
        if f"{{{NS_X}}}DataType" in node.attrib:
            data_type = self.data_type_of(node)
        own = Context(entry, template_parent=context.template_parent, in_template=context.in_template,
                      maker=context.maker, names=context.names, scope=context.scope,
                      property_owner=context.property_owner, data_type=data_type)
        for attribute, text in node.attrib.items():
            auri, aname = local(attribute)
            try:
                if auri == NS_X:
                    if aname == "Name":
                        calls.append(f".name({rust_str(text)})?")
                    elif aname in ("Key", "DataType"):
                        pass
                    else:
                        raise Unsupported(f"the directive x:{aname}")
                    continue
                if aname == "Name":
                    calls.append(f".name({rust_str(text)})?")
                    continue
                if aname == "Classes":
                    for name in text.split():
                        calls.append(f".class({rust_str(name)})")
                    continue
                calls.append(self.attribute(aname, text, own))
            except Unsupported as error:
                self.note(str(error))
                calls.append(f"\n// UNSUPPORTED: {error}\n")
        children = []
        for child in list(node):
            if is_comment(child):
                calls.append("\n" + "\n".join(comment_lines(child)) + "\n")
                continue
            _, cname = local(child.tag)
            if "." in cname:
                try:
                    calls.append(self.property_element(cname, child, own))
                except Unsupported as error:
                    self.note(str(error))
                    calls.append(f"\n// UNSUPPORTED: {error}\n")
                continue
            children.append(child)
        text = (node.text or "").strip()
        if children or text:
            content = self.model.content_property(entry)
            try:
                if content is None:
                    raise Unsupported(f"the content of {entry['name']}")
                if text and not children:
                    calls.append(self.attribute(content, node.text or "", own))
                else:
                    calls.append(self.content(self.property(entry, content), children, own))
            except Unsupported as error:
                self.note(str(error))
                calls.append(f"\n// UNSUPPORTED: {error}\n")
        return "".join(calls)

    def attribute(self, name: str, text: str, own: Context) -> str:
        prop = self.property(own.target, name)
        ext = parse_extension(text)
        if prop.kind == "plain":
            if isinstance(ext, Ext):
                raise Unsupported(f"a markup extension on the plain property {name}")
            if prop.setter is None:
                raise Unsupported(f"the plain property {name} without a setter")
            literal = self.literal(text, prop.value_type, own)
            return f".with(|e| {{ {self.use_call(prop.setter)}(e, {literal}); Ok(()) }})?"
        accessor = self.accessor(prop)
        if isinstance(ext, Ext):
            if ext.name == "TemplateBinding":
                return self.template_binding(ext, prop, own)
            if ext.name == "DynamicResource":
                return f".dynamic({accessor}, {rust_str(ext.positional[0])})"
            if ext.name == "StaticResource":
                return f".static_resource({accessor}, {rust_str(ext.positional[0])})?"
            if ext.name == "x:Null":
                return f".set_value({accessor}, None)?"
            if ext.name == "x:Static":
                expr, static_type = self.static_member(ext.positional[0])
                return f".set({accessor}, {self.optional(expr, prop, static_type)})"
            if ext.name == "x:Type":
                entry = self.type_name(text)
                return f".set({accessor}, Some({self.type_path(entry)}::TYPE))"
            if ext.name in ("Binding", "CompiledBinding", "ReflectionBinding"):
                return f".bind({accessor}, {self.binding(ext, own, prop)})"
            raise Unsupported(f"the markup extension {ext.name}")
        try:
            literal = self.literal(text, prop.value_type, own)
        except ElementRefLiteral:
            return f".set({accessor}, t.element_ref({rust_str(text.strip())})?)"
        return f".set({accessor}, {literal})"

    def template_binding(self, ext: Ext, prop: Prop, own: Context) -> str:
        source_name = ext.positional[0] if ext.positional else ext.named.get("Property")
        source = self.property(own.template_parent, source_name) if source_name else None
        extra = ext.named.keys() - {"Property"}
        if source is not None and not extra:
            return f".template_bind({self.accessor(prop)}, {self.accessor(source)})"
        if extra - {"Converter", "Mode", "ConverterParameter"}:
            raise Unsupported(f"a template binding with {sorted(extra)}")
        source_expr = f"Some({self.accessor(source)}.property())" if source is not None else "None"
        converter = self.converter(ext.named["Converter"], own) if "Converter" in ext.named else "None"
        mode = f"Some({self.use('::ferroui_base::data::BindingMode')}::{ext.named['Mode']})" if "Mode" in ext.named else "None"
        parameter = f"value({rust_str(ext.named['ConverterParameter'])}.to_string())" if "ConverterParameter" in ext.named else "None"
        return f".template_bind_with({self.accessor(prop)}, {source_expr}, {converter}, {mode}, {parameter})"

    def property_element(self, cname: str, node, own: Context) -> str:
        owner_name, pname = cname.split(".", 1)
        if pname == "Styles":
            return self.element_styles(node, own)
        if owner_name != own.target["name"] and not self.model.is_a(own.target, owner_name):
            prop = self.property(None, cname)  # an attached property element (`ToolTip.Tip`)
        else:
            prop = self.property(own.target, pname)
        children = [c for c in list(node) if not is_comment(c)]
        return self.content(prop, children, own)

    def element_styles(self, node, own: Context) -> str:
        calls = []
        for child in list(node):
            if is_comment(child):
                calls.append("\n" + "\n".join(comment_lines(child)) + "\n")
                continue
            _, name = local(child.tag)
            if name != "Style":
                raise Unsupported(f"{name} in the styles of an element")
            selector, target = self.selector(child.attrib["Selector"], own.target)
            inner = Code()
            context = Context(target or own.target, template_parent=own.template_parent, in_template=own.in_template,
                              maker="style.scope()", names=own.names, scope="style.scope()")
            self.style_members(child, inner, "style", context, "element_style")
            calls.append(f".style({selector}, |style| {{\n" + "\n".join(inner.lines) + "\nOk(())\n})?")
        return "".join(calls)

    def content(self, prop: Prop, children: list, own: Context) -> str:
        """The children of an element as the value (or the items) of the property `prop`."""
        inner, optional = strip_option(prop.value_type)
        accessor = None if prop.kind == "plain" else self.accessor(prop)
        if prop.name == "Children" and self.model.is_a(own.target, "Panel"):
            items = ", ".join(f"{self.element(c, own, own.maker)}.control()" for c in children)
            return f".children([{items}])"
        if len(children) == 1 and prop.kind != "plain" and self.is_collection(inner):
            child_entry = self.type_of(children[0]) if local(children[0].tag)[1] not in ("Binding", "MultiBinding") else None
            if child_entry is not None and child_entry["rust_path"] == inner:
                holder = Context(own.target, template_parent=own.template_parent, in_template=own.in_template, maker=own.maker,
                                 names=own.names, scope=own.scope, property_owner=self.owner_of(own), data_type=own.data_type)
                value = self.plain_object(children[0], holder)
                return f".set({accessor}, {self.optional(value, prop)})"
        if prop.kind == "plain" and not self.is_collection(inner) and len(children) == 1 and self.plain_setter(own.target, prop.name):
            setter = self.plain_setter(own.target, prop.name)
            if setter is None:
                raise Unsupported(f"the plain property {prop.name} without a setter")
            child = children[0]
            entry = self.type_of(child)
            if entry.get("object_model"):
                value = f"{self.element(child, own, own.maker)}.end()"
            else:
                value = self.plain_object(child, own)
            return f".with(|e| {{ e.{setter}({value}); Ok(()) }})?"
        if prop.kind == "plain" or self.is_collection(inner):
            # The items of a collection the element has.
            getter = prop.getter
            if prop.kind == "plain" and getter is None:
                raise Unsupported(f"the collection {prop.name} without a getter")
            holder = Context(own.target, template_parent=own.template_parent, in_template=own.in_template, maker=own.maker,
                             names=own.names, scope=own.scope, property_owner=self.owner_of(own), data_type=own.data_type)
            untyped = "BoxedValue" in inner or "ItemCollection" in inner or "BoxedValue" in (prop.value_type or "")
            items = [self.item(c, holder, untyped) for c in children]
            if "GestureRecognizer" in inner:
                items = [item[: -len(".into()")] if item.endswith(".into()") else item for item in items]
            if prop.kind != "plain":
                collection = self.model.by_path.get(inner)
                ctor = f"{self.type_path(collection)}::new()" if collection else None
                if ctor is None:
                    raise Unsupported(f"the collection {inner}")
                adds = "".join(f"items.add({item});" for item in items)
                value = f"{{ let items = {ctor}; {adds} {'Some(items)' if optional else 'items'} }}"
                return f".set({accessor}, {value})"
            adds = "".join(f"items.add({item});" for item in items)
            return f".with(|e| {{ let items = {self.use_call(getter)}(e); {adds} Ok(()) }})?"
        if len(children) != 1:
            raise Unsupported(f"{len(children)} values of {prop.name}")
        child = children[0]
        if local(child.tag)[1] in ("Binding", "CompiledBinding", "ReflectionBinding", "TemplateBinding", "DynamicResource", "MultiBinding"):
            return f".bind({accessor}, {self.plain_object(child, own)})"
        template = self.template_value(child, own, prop, "template")
        if template is not None:
            return f".set({accessor}, {self.optional(template, prop)})"
        entry = self.type_of(child)
        if entry.get("object_model"):
            element = self.element(child, own, own.maker)
            if "Ref<" in inner and inner.startswith("::ferroui_base::type_system::Ref<"):
                return f".child({accessor}, {element})"
            if inner.endswith("BoxedValue"):
                return f".content({accessor}, {element})?"
            return f".set({accessor}, {self.optional(element + '.end().into()', prop)})"
        value = self.plain_object(child, own)
        if local(child.tag)[1] in ("MultiBinding", "Binding", "CompiledBinding", "ReflectionBinding", "TemplateBinding", "DynamicResource"):
            return f".bind({accessor}, {value})"
        if inner.endswith("BoxedValue"):
            return f".set_value({accessor}, value({value}))?"
        return f".set({accessor}, {self.optional(value, prop)})"

    def owner_of(self, own: Context) -> dict | None:
        """The class the properties named by the items of a collection of `own` are of: the
        element itself (its transitions), but the animated class for the frames of an animation."""
        if own.target is not None and own.target["name"] in ("Animation", "KeyFrame", "KeyFrames"):
            return own.property_owner
        return own.target

    def plain_setter(self, entry: dict, name: str) -> str | None:
        """The name of the setter method of the plain property `name`."""
        for c in self.model.chain(entry):
            for p in c.get("properties", []):
                if p["name"] == name and p.get("setter"):
                    setter = p["setter"]
                    callable_path = (setter.get("callable") or {}).get("path")
                    if callable_path:
                        return callable_path.rsplit("::", 1)[1]
                    if isinstance(setter.get("call"), dict):
                        return setter["call"]["path"].rsplit("::", 1)[1]
                    return f"set_{snake(name)}"
        return None

    def is_collection(self, value_type: str) -> bool:
        entry = self.model.by_path.get(value_type)
        if entry is None:
            return value_type.startswith("::ferroui_base::collections::ferro_list::FerroList<")
        base = entry.get("base", "")
        return entry["name"] in ("Transitions", "ColumnDefinitions", "RowDefinitions", "KeyFrames", "GradientStops", "Classes") or "FerroList<" in base

    def item(self, child, own: Context, untyped: bool = False) -> str:
        """An item of a collection: an element or a plain object."""
        if local(child.tag)[1] in ("Binding", "CompiledBinding", "ReflectionBinding", "TemplateBinding", "DynamicResource"):
            return self.plain_object(child, own)
        template = self.template_value(child, own, None, "item")
        if template is not None:
            return template
        entry = self.type_of(child)
        if entry.get("object_model"):
            if untyped:
                return f"value({self.element(child, own, own.maker)}.end())"
            if own.target and own.target["name"] in ("GestureRecognizerCollection",) or "GestureRecognizers" in str(own.property_owner and ""):
                return f"{self.element(child, own, own.maker)}.end()"
            return f"{self.element(child, own, own.maker)}.end().into()"
        return self.plain_object(child, own)

    def plain_object(self, node, context: Context, shared: bool = True) -> str:
        """An object of a class outside the object model (a converter, a collection, a value
        parsed from text): its constructor, then its properties and items."""
        _, name = local(node.tag)
        text = (node.text or "").strip()
        attributes = {local(k)[1]: v for k, v in node.attrib.items() if local(k)[0] != NS_X}
        children = [c for c in list(node) if not is_comment(c)]
        if name == "DynamicResource":
            return f"{self.use('::ferroui_markup_xaml::rust_theme::dynamic_resource_binding')}({rust_str(attributes['ResourceKey'])})"
        if name in ("Binding", "CompiledBinding", "ReflectionBinding"):
            ext = Ext(name, [], {k: parse_extension(v) for k, v in attributes.items()})
            return self.binding(ext, context, None)
        entry = self.type_of(node)
        if name == "TemplateBinding":
            source = self.property(context.template_parent, attributes["Property"])
            return f"template_binding({self.accessor(source)})"
        if name in ("Binding", "CompiledBinding", "ReflectionBinding"):
            ext = Ext(name, [], dict(attributes))
            return self.binding(ext, context, None)
        if name == "Setter":
            prop = self.property(context.property_owner, attributes["Property"])
            value_text = attributes.get("Value")
            if value_text is None:
                raise Unsupported("an animation setter without a value")
            ext = parse_extension(value_text)
            if isinstance(ext, Ext) and ext.name in ("Binding", "CompiledBinding"):
                binding = self.binding(ext, context, prop)
                return f"{self.use('::ferroui_markup_xaml::rust_theme::bound_setter')}({self.accessor(prop)}, {binding})"
            if isinstance(ext, Ext):
                raise Unsupported(f"{ext.name} in an animation setter")
            literal = self.literal(value_text, prop.value_type, context)
            return f"{self.use('::ferroui_markup_xaml::rust_theme::setter')}({self.accessor(prop)}, {literal})"
        if name == "MultiBinding":
            for child in list(children):
                if local(child.tag)[1] == "MultiBinding.Converter":
                    converter_node = [c for c in list(child) if not is_comment(c)][0]
                    attributes["Converter"] = converter_node
                    children.remove(child)
            expr = f"{self.use('::ferroui_base::data::MultiBinding')}::new()"
            for aname, value_text in sorted(attributes.items(), key=lambda item: item[0]):
                ext = parse_extension(value_text) if isinstance(value_text, str) else None
                if aname == "Converter" and not isinstance(value_text, str):
                    expr += f".with_converter_value(Some({self.use('::std::rc::Rc')}::new({self.constructor_value(value_text)})))"
                elif aname == "Converter":
                    converter = self.converter(ext, context).replace(".converter(", ".multi_converter(")
                    expr += f".with_converter_value({converter})"
                elif aname == "ConverterParameter":
                    expr += f".with_converter_parameter(value({rust_str(value_text)}.to_string()))"
                elif aname == "Mode":
                    expr += f".with_mode({self.use('::ferroui_base::data::BindingMode')}::{value_text})"
                else:
                    raise Unsupported(f"the option {aname} of a multi binding")
            adds = "".join(f"bindings.add({self.plain_object(c, context)});" for c in children)
            return f"{{ let binding = {expr}; let bindings = binding.bindings(); {adds} binding }}"
        if text and not attributes and not children:
            if self.parses(entry):
                return self.parsed(entry, text)
            raise Unsupported(f"the text content of {entry['name']}")
        ctor = self.constructor(entry) if shared else f"{self.qualified(entry)}::new()"
        property_elements = [c for c in children if "." in local(c.tag)[1]]
        children = [c for c in children if "." not in local(c.tag)[1]]
        if not attributes and not children and not property_elements:
            return ctor
        statements = [f"let object = {ctor};"]
        for aname, value_text in attributes.items():
            prop = self.model.find(entry, aname)
            setter = None
            if prop is not None:
                for c in self.model.chain(entry):
                    for p in c.get("properties", []):
                        if p["name"] == aname and isinstance(p.get("setter"), dict):
                            callable_path = (p["setter"].get("callable") or {}).get("path")
                            if callable_path:
                                setter = callable_path.rsplit("::", 1)[1]
                if setter is None and prop.setter:
                    setter = prop.setter.rsplit("::", 1)[1]
                if setter is None:
                    for c in self.model.chain(entry):
                        for p in c.get("properties", []):
                            if p["name"] == aname and p.get("setter"):
                                setter = f"set_{snake(aname)}"
            if prop is None or setter is None:
                raise Unsupported(f"the property {aname} of {entry['name']}")
            ext = parse_extension(value_text)
            if isinstance(ext, Ext):
                if ext.name == "StaticResource":
                    vt, _ = strip_option(prop.value_type)
                    value = f"{context.scope}.resource::<{self.value_type(prop.value_type)}>({rust_str(ext.positional[0])})?"
                elif ext.name == "x:Static":
                    value, _ = self.static_member(ext.positional[0])
                    value = self.optional(value, prop)
                else:
                    raise Unsupported(f"{ext.name} on the plain property {aname}")
            else:
                value = self.literal(value_text, prop.value_type, context)
            statements.append(f"object.{setter}({value});")
        for element in property_elements:
            pname = local(element.tag)[1].split(".", 1)[1]
            setter = self.plain_setter(entry, pname)
            values = [c for c in list(element) if not is_comment(c)]
            if setter is None or len(values) != 1:
                raise Unsupported(f"the property element {pname} of {entry['name']}")
            value_entry = self.type_of(values[0])
            if value_entry.get("object_model"):
                value = f"{self.element(values[0], context, context.maker)}.end()"
            else:
                value = self.plain_object(values[0], context, shared=False)
            statements.append(f"object.{setter}({value});")
        if children:
            content = self.model.content_property(entry)
            method = None
            for m in entry.get("methods", []):
                if m["name"] == "Add" and isinstance(m.get("call"), dict):
                    method = m["call"]["path"]
            for child in children:
                item = self.item(child, context)
                if method is not None:
                    statements.append(f"{self.use_call(method)}(&object, {item});")
                else:
                    statements.append(f"object.add({item});")
        statements.append("object")
        return "{ " + " ".join(statements) + " }"

    def parses(self, entry: dict) -> bool:
        parse = entry.get("parse_call")
        return (isinstance(parse, dict) and bool(parse.get("path"))) or bool((entry.get("parse") or {}).get("path"))

    def parsed(self, entry: dict, text: str) -> str:
        parse = entry.get("parse_call")
        if isinstance(parse, dict) and parse.get("path"):
            return f"literal({self.use_call(parse['path'])}({rust_str(text)}))?"
        function = entry["parse"]["path"].rsplit("::", 1)[-1]
        return f"literal({self.qualified(entry)}::{function}({rust_str(text)}))?"

    def value_type(self, value_type: str) -> str:
        """A Rust type of the models, with the public paths of its types imported."""
        def replace(match):
            path = match.group(0)
            if path == "::ferroui_base::type_system::Ref":
                return "Ref"
            if path == "::std::rc::Rc":
                return "std::rc::Rc"
            entry = self.model.by_path.get(path)
            if entry is not None:
                return self.type_path(entry)
            # A type of a private module: its public path is the module above it.
            parent, _, name = path.rpartition("::")
            return self.use(parent.rpartition("::")[0] + "::" + name)
        return re.sub(r"(::[a-z_0-9]+)+::[A-Z]\w*", replace, value_type)

    # --- bindings ---

    def converter(self, ext, own: Context) -> str:
        """A value converter: `{x:Static Type.Member}` or `{StaticResource Key}`."""
        if isinstance(ext, Ext) and ext.name == "x:Static":
            expr, _ = self.static_member(ext.positional[0])
            return f"Some({expr})"
        if isinstance(ext, Ext) and ext.name == "StaticResource":
            return f"Some({own.scope}.converter({rust_str(ext.positional[0])})?)"
        raise Unsupported(f"the converter {ext}")

    def binding(self, ext: Ext, own: Context, prop: Prop | None) -> str:
        named = dict(ext.named)
        if ext.name == "ReflectionBinding":
            path_text = ext.positional[0] if ext.positional else named.pop("Path", "")
            expr = f"{self.use('::ferroui_base::data::ReflectionBinding')}::new({rust_str(path_text)})"
            relative = named.pop("RelativeSource", None)
            if relative is not None:
                mode = relative.positional[0] if relative.positional else relative.named.get("Mode")
                expr += (f".with_relative_source(Some({self.use('::ferroui_base::data::RelativeSource')}::new("
                         f"{self.use('::ferroui_base::data::RelativeSourceMode')}::{mode})))")
            if named:
                raise Unsupported(f"the options {sorted(named)} of a reflection binding")
            return expr
        path_text = ext.positional[0] if ext.positional else named.pop("Path", "")
        named.pop("Path", None)
        if isinstance(path_text, Ext):
            raise Unsupported("a path that is a markup extension")
        steps = []
        current = None
        relative = named.pop("RelativeSource", None)
        element_name = named.pop("ElementName", None)
        text = path_text.strip()
        negate = 0
        while text.startswith("!"):
            negate += 1
            text = text[1:]
        if relative is not None:
            if not isinstance(relative, Ext) or relative.name != "RelativeSource":
                raise Unsupported("a relative source")
            mode = relative.positional[0] if relative.positional else relative.named.get("Mode")
            if mode == "TemplatedParent":
                steps.append(".templated_parent()")
                current = own.template_parent
            elif mode == "Self":
                steps.append(".self_()")
                current = own.target
            elif mode == "FindAncestor":
                ancestor = self.type_name(relative.named["AncestorType"])
                level = int(relative.named.get("AncestorLevel", "1")) - 1
                steps.append(f".ancestor_of::<{self.type_path(ancestor)}>({level})")
                current = ancestor
            else:
                raise Unsupported(f"the relative source {mode}")
        elif element_name is not None:
            steps.append(f".named(t.names(), {rust_str(element_name)})")
            current = (own.names or {}).get(element_name)
        segments = split_path(text)
        if segments and segments[0].startswith("$parent"):
            m = re.match(r"\$parent\[([\w:|]+)(?:\s*,\s*(\d+))?\]", segments[0])
            ancestor = self.type_name(m.group(1).replace("|", ":"))
            steps.append(f".ancestor_of::<{self.type_path(ancestor)}>({int(m.group(2) or 0)})")
            current = ancestor
            segments = segments[1:]
        elif segments and segments[0].startswith("$self"):
            steps.append(".self_()")
            current = own.target
            segments = segments[1:]
        elif segments and segments[0].startswith("#"):
            name = segments[0][1:]
            steps.append(f".named(t.names(), {rust_str(name)})")
            current = (own.names or {}).get(name)
            if current is None:
                raise Unsupported(f"the element {name} of a binding path")
            segments = segments[1:]
        elif relative is None and element_name is None and segments:
            # The data context: typed by `x:DataType`, unknown otherwise.
            current = own.data_type
            if current is None and not segments[0].startswith("("):
                raise Unsupported(f"a binding to an untyped data context ({path_text})")
        for segment in segments:
            accepts_null = segment.endswith("?")
            segment = segment.rstrip("?")
            if current is None and not segment.startswith("("):
                raise Unsupported(f"the path {path_text} on an unknown type")
            if segment.startswith("(") and segment.endswith(")"):
                p = self.property(None, segment[1:-1])
            else:
                p = self.model.find(current, segment)
                if p is None:
                    method = self.method_of(current, segment)
                    if method is not None:
                        steps.append(method)
                        current = None
                        continue
                    raise Unsupported(f"the member {segment} of {current['name']}")
            if p.kind == "plain" and p.owner.get("object_model"):
                steps.append(f".class_member::<{self.type_path(p.owner)}>({rust_str(p.name)})")
            elif p.kind == "plain":
                steps.append(f".member::<{self.markup_type(p.owner)}>({rust_str(p.name)})")
            elif accepts_null:
                steps.append(f".prop_or_null({self.accessor(p)})")
            else:
                steps.append(f".prop({self.accessor(p)})")
            inner, _ = strip_option(p.value_type)
            inner = re.sub(r"^::ferroui_base::type_system::Ref<(.*)>$", r"\1", inner)
            inner = re.sub(r"^::std::rc::Rc<(.*)>$", r"\1", inner)
            current = self.model.by_path.get(inner)
        for _ in range(negate):
            steps.append(".not()")
        expr = "path()" + "".join(steps) + ".binding()"
        for key, value in sorted(named.items()):
            if key == "Mode":
                expr += f".with_mode({self.use('::ferroui_base::data::BindingMode')}::{value})"
            elif key == "Converter":
                expr += f".with_converter_value({self.converter(value, own)})"
            elif key == "ConverterParameter":
                expr += f".with_converter_parameter({self.option_value(value)})"
            elif key == "StringFormat":
                expr += f".with_string_format(Some(String::from({rust_str(value)})))"
            elif key == "FallbackValue":
                expr += f".with_fallback_value({self.option_value(value)})"
            elif key == "TargetNullValue":
                expr += f".with_target_null_value({self.option_value(value)})"
            elif key == "x:DataType" or key == "DataType":
                continue
            else:
                raise Unsupported(f"the binding option {key}")
        return expr

    def option_value(self, value) -> str:
        """The value of an option of a binding (a parameter, a fallback value)."""
        if isinstance(value, Ext):
            if value.name == "x:Static":
                expr, _ = self.static_member(value.positional[0])
                return f"value({expr})"
            if value.name == "x:Null":
                return "None"
            raise Unsupported(f"the option value {value.name}")
        return f"value({rust_str(value)}.to_string())"

    def markup_type(self, entry: dict) -> str:
        path = self.type_path(entry)
        return f"dyn {path}" if entry.get("kind") == "interface" else path

    def method_of(self, entry: dict, name: str) -> str | None:
        for c in self.model.chain(entry):
            for m in c.get("methods", []):
                if m["name"] == name and not m.get("is_static"):
                    this = self.type_path(c)
                    if isinstance(m.get("call"), dict):
                        call = m["call"]["path"].rsplit("::", 1)[1]
                    elif (m.get("callable") or {}).get("path"):
                        call = m["callable"]["path"].rsplit("::", 1)[1]
                    else:
                        call = snake(name)
                    handle = f"Ref<{this}>" if c.get("object_model") else this
                    return f".method::<{handle}>({rust_str(name)}, |this| {{ this.{call}(); }})"
        return None


def collect_names(converter, node, names: dict):
    """The classes of the named elements of a template, for the paths of bindings (`#PART_X`)."""
    if is_comment(node):
        return
    name = node.attrib.get(f"{{{NS_X}}}Name") or node.attrib.get("Name")
    _, tag = local(node.tag)
    if name and "." not in tag:
        try:
            names[name] = converter.type_of(node)
        except Unsupported:
            pass
    for child in list(node):
        collect_names(converter, child, names)


def timespan_ms(text: str) -> float | None:
    """The milliseconds of a time span in the text form of XAML (`[-][d.]h:m:s[.f]`)."""
    m = re.match(r"^(-)?(?:(\d+)\.)?(\d+):(\d+):(\d+(?:\.\d+)?)$", text.strip())
    if not m:
        return None
    sign = -1 if m.group(1) else 1
    days = int(m.group(2) or 0)
    total = ((days * 24 + int(m.group(3))) * 60 + int(m.group(4))) * 60 + float(m.group(5))
    return sign * round(total * 1000, 6)


def split_path(text: str) -> list[str]:
    parts, depth, current = [], 0, ""
    for char in text:
        if char in "([":
            depth += 1
        elif char in ")]":
            depth -= 1
        if char == "." and depth == 0:
            parts.append(current)
            current = ""
        else:
            current += char
    if current:
        parts.append(current)
    return [p.strip() for p in parts if p.strip()]


# The names of the API of themes in Rust: a type of the same name is named by its path.
RESERVED_NAMES = {"Template", "Element", "Resources", "Scope", "StyleBuilder", "Result", "Selector", "ThemeVariant",
                  "ResourceKey", "Ref", "AnyProperty", "ThemeProperty", "PathExt", "SelectorExt", "Rc"}

PRELUDE_NAMES = {"Selector", "ThemeVariant", "ResourceKey", "ObjectType", "Ref", "StaticType", "Upcast"}


def header_lines(header: str) -> list[str]:
    lines = header.strip("\n").split("\n")
    indents = [len(line) - len(line.lstrip()) for line in lines if line.strip()]
    cut = min(indents) if indents else 0
    return [line[cut:].rstrip() for line in lines]


def split_top(text: str, separator: str) -> list[str]:
    parts, depth, current = [], 0, ""
    for char in text:
        if char in "([":
            depth += 1
        elif char in ")]":
            depth -= 1
        if char == separator and depth == 0:
            parts.append(current)
            current = ""
        else:
            current += char
    parts.append(current)
    return parts


def nth(value: str) -> tuple[int, int]:
    value = value.replace(" ", "")
    if value == "odd":
        return 2, 1
    if value == "even":
        return 2, 0
    match = re.match(r"^([+-]?\d*)n([+-]\d+)?$", value)
    if match:
        step = match.group(1)
        step = 1 if step in ("", "+") else -1 if step == "-" else int(step)
        return step, int(match.group(2) or 0)
    return 0, int(value)


def tokenize_selector(text: str):
    tokens = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == "^":
            tokens.append(("nesting", None))
            i += 1
        elif text.startswith("/template/", i):
            tokens.append(("template", None))
            i += len("/template/")
        elif c == ">":
            tokens.append(("child", None))
            i += 1
        elif c.isspace():
            j = i
            while j < n and text[j].isspace():
                j += 1
            following = text[j:j + 1]
            if following not in (">", "") and not text.startswith("/template/", j) and tokens and tokens[-1][0] not in ("child", "template"):
                tokens.append(("descendant", None))
            i = j
        elif c == ".":
            m = re.match(r"\.([\w-]+)", text[i:])
            tokens.append(("class", m.group(1)))
            i += len(m.group(0))
        elif c == "#":
            m = re.match(r"#([\w-]+)", text[i:])
            tokens.append(("name", m.group(1)))
            i += len(m.group(0))
        elif c == ":":
            m = re.match(r":([\w-]+)", text[i:])
            pseudo = m.group(1)
            i += len(m.group(0))
            if i < n and text[i] == "(":
                depth, j = 0, i
                while j < n:
                    if text[j] == "(":
                        depth += 1
                    elif text[j] == ")":
                        depth -= 1
                        if depth == 0:
                            break
                    j += 1
                argument = text[i + 1:j]
                i = j + 1
                if pseudo == "is":
                    tokens.append(("is", argument.strip()))
                elif pseudo == "not":
                    tokens.append(("not", argument.strip()))
                elif pseudo in ("nth-child", "nth-last-child"):
                    tokens.append((pseudo, argument.strip()))
                else:
                    raise Unsupported(f"the pseudo function :{pseudo}()")
            else:
                tokens.append(("class", ":" + pseudo))
        elif c == "[":
            j = text.index("]", i)
            body = text[i + 1:j]
            name, _, value = body.partition("=")
            tokens.append(("property", (name.strip(), value.strip())))
            i = j + 1
        else:
            m = re.match(r"[\w|]+", text[i:])
            if not m:
                raise Unsupported(f"the selector {text}")
            word = m.group(0)
            tokens.append(("type", word))
            i += len(word)
    return tokens


def module_of(document: str) -> str:
    """The Rust module of a document: `Controls/Button.xaml` is `controls::button`."""
    stem = document[: -len(".xaml")] if document.endswith(".xaml") else document
    return "::".join(snake(part) for part in stem.split("/"))


def find_models(target_dir: str) -> list[str]:
    found = []
    for crate in MODEL_CRATES:
        candidates = glob.glob(os.path.join(target_dir, "*", "build", "*", "out", f"{crate}.xamlmeta"))
        if not candidates:
            raise SystemExit(f"no type model of {crate} below {target_dir}: build the theme crate first")
        found.append(max(candidates, key=os.path.getmtime))
    return found


def documents_of(crate: str) -> list[str]:
    out = []
    for directory, _, files in os.walk(crate):
        relative_directory = os.path.relpath(directory, crate)
        if relative_directory.split(os.sep)[0] in ("tests", "target", "rust_theme"):
            continue
        for name in files:
            if name.endswith(".xaml"):
                out.append(os.path.normpath(os.path.join(relative_directory, name)).replace(os.sep, "/"))
    return sorted(out)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--crate", default="src/FerroUI.Themes.WinUI")
    parser.add_argument("--target-dir", default=os.environ.get("CARGO_TARGET_DIR", "target"))
    parser.add_argument("--check", action="store_true", help="compare with the files instead of writing them")
    parser.add_argument("--report", action="store_true", help="print what was not converted")
    parser.add_argument("--only", help="convert only this document")
    args = parser.parse_args()

    model = Model(find_models(args.target_dir))
    with open(os.path.join(args.crate, "compiled_xaml.xamlmeta"), encoding="utf-8") as handle:
        metadata = json.load(handle)
    units = metadata["theme_units"][0]["units"]
    unit_names = {}
    for unit in units:
        for document in unit["documents"]:
            unit_names[document] = unit["index"]
    converter = Converter(model, args.crate, unit_names)
    outputs: dict[str, str] = {}
    for document in documents_of(args.crate):
        if args.only and document != args.only:
            continue
        doc = Document(args.crate, document)
        try:
            text = converter.convert(doc)
        except (Unsupported, TypeError, KeyError, AttributeError, IndexError, ValueError) as error:
            converter.note(f"document: {error}")
            print(f"{document}: {error}", file=sys.stderr)
            continue
        outputs[os.path.join("rust_theme", module_of(document).replace("::", "/") + ".rs")] = text
    outputs.update(module_files(outputs, units))
    formatted = rustfmt(outputs)
    if args.report or converter.unsupported:
        for what, count in sorted(converter.unsupported.items(), key=lambda item: -item[1]):
            print(f"{count:6} {what}", file=sys.stderr)
    changed = []
    for relative, text in sorted(formatted.items()):
        path = os.path.join(args.crate, relative)
        current = open(path, encoding="utf-8").read() if os.path.exists(path) else None
        if current != text:
            changed.append(relative)
            if not args.check:
                os.makedirs(os.path.dirname(path), exist_ok=True)
                with open(path, "w", encoding="utf-8") as handle:
                    handle.write(text)
    if args.check:
        if changed:
            print("out of date: " + ", ".join(changed), file=sys.stderr)
            sys.exit(1)
        print(f"{len(formatted)} files up to date")
    else:
        print(f"{len(formatted)} files, {len(changed)} written")


def module_files(outputs: dict[str, str], units: list[dict]) -> dict[str, str]:
    """The `mod.rs` files of the directories of the modules."""
    tree: dict[str, set[str]] = {}
    for relative in outputs:
        parts = relative.split("/")
        for depth in range(1, len(parts)):
            directory = "/".join(parts[:depth])
            name = parts[depth][:-3] if parts[depth].endswith(".rs") else parts[depth]
            tree.setdefault(directory, set()).add(name)
    files = {}
    for directory, names in tree.items():
        if directory == "rust_theme":
            continue
        lines = ["//! Generated by `scripts/themes/xaml_to_rust.py` (#988): one module per document.", ""]
        lines += [f"pub(crate) mod {name};" for name in sorted(names)]
        files[f"{directory}/mod.rs"] = "\n".join(lines) + "\n"
    root = ["// Generated by `scripts/themes/xaml_to_rust.py` (#988): the documents of the theme as Rust.", ""]
    root += [f"pub(crate) mod {name};" for name in sorted(tree.get("rust_theme", set()))]
    files["rust_theme/documents.rs"] = "\n".join(root) + "\n"
    files["rust_theme/units.rs"] = units_file(units)
    return files


def units_file(units: list[dict]) -> str:
    """The units of the theme (#568) over the Rust modules: the table `compiled_xaml` has for
    the compiled markup, with the same names, indices and paths, so that the applications that
    trim the theme name the same statics. A unit applies the document it is made of."""
    out = [
        "//! The units of the theme (#568) over the documents in Rust: generated by",
        "//! `scripts/themes/xaml_to_rust.py` (#988) from the unit graph of `compiled_xaml.xamlmeta`.",
        "",
        "use ferroui_markup_xaml::rust_theme::{populating, Resources};",
        "use ferroui_markup_xaml::xaml_il::runtime::compiled as rt;",
        "use ferroui_markup_xaml::xaml_il::runtime::XamlIlContext;",
        "use ferroui_markup_xaml::XamlLoadError;",
        "use ferroui_base::controls::ResourceDictionary;",
        "use ferroui_base::metadata::IServiceProvider;",
        "use ferroui_base::Ref;",
        "use std::rc::Rc;",
        "",
        "/// The full name of the class of the theme.",
        "pub const THEME_CLASS: &str = \"FerroUI.Themes.WinUI.WinUITheme\";",
        "",
        "static DOCUMENT: rt::DocumentInfo = rt::DocumentInfo { base_uri: Some(\"ferres://FerroUI.Themes.WinUI/WinUITheme.xaml\"), namespaces: &[] };",
        "",
    ]
    for unit in units:
        index = unit["index"]
        runs = []
        if index != 0:
            documents = unit["documents"]
            function = f"run_{index}"
            out.append(f"/// Applies `{documents[0]}`.")
            out.append(f"fn {function}(_: &Rc<XamlIlContext>, dictionary: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {{")
            out.append("    let resources = Resources::of(dictionary);")
            for document in documents:
                out.append(f"    crate::rust_theme::{module_of(document)}::merge(&resources)?;")
            out.append("    Ok(())")
            out.append("}")
            out.append("")
            runs.append(function)
        out.append(f"/// The unit `{unit['name']}` of the theme (#568).")
        out.append(f"pub static THEME_UNIT_{index}: rt::ThemeUnit = rt::ThemeUnit {{ name: {json.dumps(unit['name'])}, index: {index}, runs: &[{', '.join(runs)}] }};")
        out.append("")
    out.append("/// Every unit of the theme, in canonical order.")
    out.append("pub static THEME_UNITS: &[&rt::ThemeUnit] = &[" + ", ".join(f"&THEME_UNIT_{u['index']}" for u in units) + "];")
    out.append("")
    out.append("/// The ambient unit alone.")
    out.append("static AMBIENT_THEME_UNITS: &[&rt::ThemeUnit] = &[&THEME_UNIT_0];")
    out.append("")
    out.append("/// The type keys each unit defines, for the diagnostic of a trimmed control theme.")
    keys = ", ".join("&[" + ", ".join(json.dumps(k) for k in u["type_keys"]) + "]" for u in units)
    out.append(f"pub static THEME_UNIT_TYPE_KEYS: &[&[&str]] = &[{keys}];")
    out.append("")
    out.append("""/// The units the build applies: those of the last populate of the class.
pub fn active_theme_units() -> rt::ThemeParts {
    rt::active_theme_units(THEME_CLASS)
}

/// The units the application registered, else the ambient unit alone (every unit with
/// `ferroui-base/full-themes`).
#[cfg(not(ferroui_full_themes))]
pub fn registered_theme_units() -> rt::ThemeParts {
    rt::registered_theme_parts(THEME_CLASS).unwrap_or(AMBIENT_THEME_UNITS)
}

/// The units the application registered, else the ambient unit alone (every unit with
/// `ferroui-base/full-themes`).
#[cfg(ferroui_full_themes)]
pub fn registered_theme_units() -> rt::ThemeParts {
    THEME_UNITS
}

/// Populates `root` with every unit of the theme.
pub fn populate(
    service_provider: Option<Rc<dyn IServiceProvider>>,
    root: &Ref<crate::WinUITheme>,
) -> Result<(), XamlLoadError> {
    populate_with_parts(service_provider, root, THEME_UNITS)
}

/// Populates `root` from the document of the class in Rust with the units `parts`.
pub fn populate_with_parts(
    service_provider: Option<Rc<dyn IServiceProvider>>,
    root: &Ref<crate::WinUITheme>,
    parts: rt::ThemeParts,
) -> Result<(), XamlLoadError> {
    rt::set_active_theme_units(THEME_CLASS, parts);
    let context = rt::populate_context(service_provider, &DOCUMENT, rt::to_value(root.clone()));
    let styles = root.upcast_ref::<ferroui_base::styling::Styles>();
    let scope = ferroui_markup_xaml::rust_theme::Scope::empty().with_styles(styles);
    populating(scope.clone(), parts, context, || crate::rust_theme::win_ui_theme::populate(styles, &scope))
}

/// The loader table of the theme: the public document of the class, as the compiled markup
/// answers it.
pub fn try_load(
    service_provider: Option<&Rc<dyn IServiceProvider>>,
    uri: &str,
) -> Result<Option<ferroui_base::BoxedValue>, XamlLoadError> {
    if rt::uri_equals(uri, "ferres://ferroui.themes.winui/WinUITheme.xaml", "") {
        let service_provider = ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v3(service_provider.cloned());
        return Ok(ferroui_base::metadata::into_markup_value(crate::WinUITheme::with_registered_parts(Some(service_provider))));
    }
    Ok(None)
}""")
    return "\n".join(out) + "\n"


def rustfmt(outputs: dict[str, str]) -> dict[str, str]:
    result = {}
    for relative, text in outputs.items():
        process = subprocess.run(
            ["rustfmt", "--edition", "2021", "--emit", "stdout", "--config", "max_width=120,newline_style=Unix"],
            input=text,
            capture_output=True,
            text=True,
        )
        if process.returncode != 0:
            print(f"rustfmt failed on {relative}:\n{process.stderr[:2000]}", file=sys.stderr)
            result[relative] = text
        else:
            result[relative] = process.stdout
    return result


if __name__ == "__main__":
    main()
