"""Write the case tables of the editor's tests from a record of the system Msftedit.

`native_cases.project_*` derives from a record the values each table holds; `check` compares them
with the table, and this module writes them into it. The writer edits a table in place: it parses
the constants with the position of every literal, replaces only the literals whose value differs
from the record, and keeps the layout, comments and helper calls of the table. Where the record
has more or fewer entries than the table, the entries are written from a neighbouring entry of the
table as the template, so a new row looks like the rows around it. A table that agrees with the
record is left byte for byte as it is, apart from its provenance (the comment block at its top,
and the provenance constants a table declares).
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from pathlib import Path as FilePath

import native_cases
import rust_literals
from rust_literals import Call, Path, Struct


@dataclass
class Node:
    kind: str  # "str", "int", "bool", "path", "call", "struct", "seq"
    start: int
    end: int
    value: object = None  # scalars: the value; path/call/struct: the name as written
    children: list = field(default_factory=list)  # seq and call: values; struct: (key, node)
    first: int = 0  # seq: the offset just after its opening bracket


class SpanParser:
    """`rust_literals.Parser` that keeps the source span of each value."""

    def __init__(self, source: str, tokens: list[tuple[str, str, int, int]], index: int):
        self.source = source
        self.tokens = tokens
        self.index = index

    def peek(self, offset: int = 0):
        index = self.index + offset
        return self.tokens[index] if index < len(self.tokens) else ("end", "", len(self.source), len(self.source))

    def take(self, value: str | None = None):
        token = self.peek()
        if value is not None and token[1] != value:
            raise ValueError(f"expected {value!r}, found {token[1]!r}")
        self.index += 1
        return token

    def sequence(self, closing: str) -> list:
        values = []
        while self.peek()[1] != closing:
            values.append(self.expression())
            if self.peek()[1] == ",":
                self.take(",")
        return values

    def expression(self) -> Node:
        kind, value, start, end = self.peek()
        if value == "&":
            self.take()
            node = self.expression()
            node.start = start
            return node
        if value == "-":
            self.take()
            node = self.expression()
            return Node("int", start, node.end, -node.value)
        if value in "[(":
            self.take()
            first = self.peek()[2]
            closing = "]" if value == "[" else ")"
            items = self.sequence(closing)
            close = self.take(closing)
            return Node("seq", start, close[3], value, items, first)
        if kind == "string":
            self.take()
            return Node("str", start, end, rust_literals.rust_string(value))
        if kind == "number":
            self.take()
            return Node("int", start, end, rust_literals.rust_integer(value))
        if kind == "ident":
            if value in ("true", "false"):
                self.take()
                return Node("bool", start, end, value == "true")
            self.take()
            last = end
            while self.peek()[0] == "path_separator":
                self.take()
                last = self.take()[3]
            name = self.source[start:last]
            if self.peek()[1] == "(":
                self.take()
                args = self.sequence(")")
                close = self.take(")")
                return Node("call", start, close[3], name, args)
            if self.peek()[1] == "{":
                self.take()
                fields = []
                while self.peek()[1] != "}":
                    key = self.take()[1]
                    self.take(":")
                    fields.append((key, self.expression()))
                    if self.peek()[1] == ",":
                        self.take(",")
                close = self.take("}")
                return Node("struct", start, close[3], name, fields)
            return Node("path", start, last, name)
        raise ValueError(f"unexpected token {value!r}")


def tokenize(source: str) -> list[tuple[str, str, int, int]]:
    tokens = []
    for match in rust_literals.TOKEN.finditer(source):
        kind = match.lastgroup
        if kind in ("space", "line_comment", "block_comment"):
            continue
        tokens.append((kind, match.group(), match.start(), match.end()))
    return tokens


def constant_nodes(source: str) -> dict[str, Node]:
    """The values of the `const NAME: Type = value;` items of a table, with their spans."""
    tokens = tokenize(source)
    result = {}
    index = 0
    while index < len(tokens):
        if (
            tokens[index][1] == "const"
            and index + 2 < len(tokens)
            and tokens[index + 1][0] == "ident"
            and tokens[index + 2][1] == ":"
        ):
            name = tokens[index + 1][1]
            index += 3
            depth = 0
            while not (tokens[index][1] == "=" and depth == 0):
                if tokens[index][1] in "([<":
                    depth += 1
                elif tokens[index][1] in ")]>":
                    depth -= 1
                index += 1
            parser = SpanParser(source, tokens, index + 1)
            result[name] = parser.expression()
            parser.take(";")
            index = parser.index
            continue
        index += 1
    return result


def last_segment(name: str) -> str:
    return name.rsplit("::", 1)[-1].strip()


def rust_quote(value: str) -> str:
    out = ['"']
    for character in value:
        code = ord(character)
        if character == "\\":
            out.append("\\\\")
        elif character == '"':
            out.append('\\"')
        elif character == "\n":
            out.append("\\n")
        elif character == "\r":
            out.append("\\r")
        elif character == "\t":
            out.append("\\t")
        elif code < 0x20 or code == 0x7F:
            out.append(f"\\u{{{code:x}}}")
        else:
            out.append(character)
    out.append('"')
    return "".join(out)


def is_none(value) -> bool:
    return value is None or value == native_cases.NONE


def render(value) -> str:
    """A value without a template in the table (a new `Some`, or a new entry of an empty list)."""
    if is_none(value):
        return "None"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        return rust_quote(value)
    if isinstance(value, list):
        return "[" + ", ".join(render(item) for item in value) + "]"
    if isinstance(value, Struct):
        return value.name + " { " + ", ".join(f"{key}: {render(item)}" for key, item in value.fields.items()) + " }"
    if isinstance(value, Call):
        return value.name + "(" + ", ".join(render(item) for item in value.args) + ")"
    if isinstance(value, Path):
        return value.name
    raise ValueError(f"cannot write {value!r}")


def integer_text(template: str, value: int) -> str:
    text = template.lstrip("-")
    if text.lower().startswith("0x"):
        digits = len(text) - 2
        body = f"{value:0{digits}X}" if text[2:] == text[2:].upper() else f"{value:0{digits}x}"
        return "0x" + body
    return str(value)


class Writer:
    def __init__(self, source: str):
        self.source = source

    def text(self, node: Node, expected) -> str:
        """The text of `node` with the value `expected`."""
        edits = sorted(self.edits(node, expected), key=lambda edit: edit[0])
        out = []
        position = node.start
        for start, end, replacement in edits:
            out.append(self.source[position:start])
            out.append(replacement)
            position = end
        out.append(self.source[position:node.end])
        return "".join(out)

    def edits(self, node: Node, expected) -> list[tuple[int, int, str]]:
        whole = [(node.start, node.end, None)]
        # Options: the record has the value or None; the table writes `Some(value)` or `None`.
        if node.kind == "call" and last_segment(node.value) == "Some" and len(node.children) == 1:
            if is_none(expected):
                return [(node.start, node.end, "None")]
            return self.edits(node.children[0], expected)
        if node.kind == "path" and last_segment(node.value) == "None":
            if is_none(expected):
                return []
            return [(node.start, node.end, "Some(" + render(expected) + ")")]
        if node.kind == "str":
            return [] if node.value == expected else [(node.start, node.end, rust_quote(expected))]
        if node.kind == "bool":
            return [] if node.value == expected else [(node.start, node.end, render(expected))]
        if node.kind == "int":
            if node.value == expected:
                return []
            return [(node.start, node.end, integer_text(self.source[node.start:node.end], expected))]
        if node.kind == "path":
            if isinstance(expected, Path):
                if last_segment(node.value) == last_segment(expected.name):
                    return []
                prefix = node.value[: len(node.value) - len(last_segment(node.value))]
                return [(node.start, node.end, prefix + last_segment(expected.name))]
            return [(node.start, node.end, render(expected))]
        if node.kind == "seq" and isinstance(expected, list):
            if len(node.children) == len(expected):
                return [edit for child, item in zip(node.children, expected) for edit in self.edits(child, item)]
            return [(node.start, node.end, self.sequence(node, expected))]
        if node.kind == "struct" and isinstance(expected, Struct):
            names = [key for key, _ in node.children]
            if last_segment(node.value) == last_segment(expected.name) and names == list(expected.fields):
                return [edit for (key, child) in node.children for edit in self.edits(child, expected.fields[key])]
        if node.kind == "call" and isinstance(expected, Call):
            if last_segment(node.value) == last_segment(expected.name) and len(node.children) == len(expected.args):
                return [edit for child, item in zip(node.children, expected.args) for edit in self.edits(child, item)]
            if len(node.children) == len(expected.args):
                # Another variant with the same arguments: keep the path the table writes.
                prefix = node.value[: len(node.value) - len(last_segment(node.value))]
                name_end = node.start + len(node.value)
                return [(node.start, name_end, prefix + last_segment(expected.name))] + [
                    edit for child, item in zip(node.children, expected.args) for edit in self.edits(child, item)
                ]
        if node.kind == "call" and isinstance(expected, Path):
            prefix = node.value[: len(node.value) - len(last_segment(node.value))]
            return [(node.start, node.end, prefix + last_segment(expected.name))]
        return [(start, end, render(expected)) for start, end, _ in whole]

    def sequence(self, node: Node, expected: list) -> str:
        """A list whose length differs from the table's: each entry from the entry at its index,
        or from the last entry of the table, as its template."""
        opening = self.source[node.start:node.first]
        if not node.children:
            return opening + ", ".join(render(item) for item in expected) + self.source[node.end - 1:node.end]
        closing = self.source[node.children[-1].end:node.end]
        separator = ", "
        if len(node.children) > 1:
            separator = self.source[node.children[0].end:node.children[1].start]
        items = []
        for index, item in enumerate(expected):
            template = node.children[min(index, len(node.children) - 1)]
            items.append(self.text(template, item))
        if not expected:
            return opening.rstrip() + closing.lstrip(",").lstrip()
        return opening + separator.join(items) + closing


def provenance_values(record: dict, record_sha256: str) -> dict:
    provenance = record.get("provenance") or {}
    engine = record.get("engine") or {}
    return {
        "SOURCE_SHA256": record_sha256,
        "MEASURED_COMMIT": provenance.get("commit", ""),
        "PROBE_GIT_BLOB": provenance.get("probe_git_blob", ""),
        "ENGINE_VERSION": engine.get("dll_file_version", ""),
    }


def header(table: native_cases.Table, record: dict, record_sha256: str, source_name: str) -> str:
    provenance = record.get("provenance") or {}
    engine = record.get("engine") or {}
    origin = provenance.get("run_url") or "a local run of the oracle"
    windows = engine.get("windows_version", "unknown")
    if isinstance(windows, list):
        windows = ".".join(str(part) for part in windows)
    lines = [
        "// Generated by scripts/winui-rich-text/rich-text-tom-observations.py write-cases; do not edit",
        "// by hand (docs/porting/winui-rich-text.md, section 8). Literal observations of the system",
        f"// Msftedit {engine.get('dll_file_version', 'unknown')} on Windows {windows}, never Rust expectations.",
        f"// Record: {source_name} of {origin}",
        f"// (SHA-256 {record_sha256}); oracle built from {provenance.get('commit', 'unknown')}.",
    ]
    lines += [f"// {note}" if note else "//" for note in table.notes]
    return "\n".join(lines) + "\n"


def leading_comment_end(source: str) -> int:
    """The end of the comment block at the top of a table (`//` and `//!` lines)."""
    position = 0
    for line in source.splitlines(keepends=True):
        if line.startswith("//"):
            position += len(line)
        else:
            break
    return position


def write_table(table: native_cases.Table, record: dict, record_sha256: str, source_name: str, source: str) -> str:
    nodes = constant_nodes(source)
    expected = table.project(record)
    provenance = provenance_values(record, record_sha256)
    for name in table.provenance:
        if name in nodes:
            expected[name] = provenance[name]
    edits = []
    writer = Writer(source)
    for name, value in expected.items():
        if name not in nodes:
            raise ValueError(f"{table.file}: the table has no constant {name}")
        edits += writer.edits(nodes[name], value)
    for start, end, replacement in sorted(edits, key=lambda edit: edit[0], reverse=True):
        source = source[:start] + replacement + source[end:]
    return header(table, record, record_sha256, source_name) + source[leading_comment_end(source):]


def write(record_path: FilePath, root: FilePath) -> list[str]:
    """Writes every case table from the record; returns the tables that changed."""
    data = record_path.read_bytes()
    record = json.loads(data.decode("utf-8"))
    record_sha256 = hashlib.sha256(data).hexdigest()
    changed = []
    for table in native_cases.TABLES:
        path = root / native_cases.CASES_DIRECTORY / table.file
        source = path.read_text(encoding="utf-8")
        result = write_table(table, record, record_sha256, record_path.name, source)
        if result != source:
            path.write_text(result, encoding="utf-8")
            changed.append(table.file)
    return changed
