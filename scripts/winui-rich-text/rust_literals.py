"""Read the constant tables of a Rust source file as Python values.

The case tables of the editor's tests (`src/FerroUI.Controls/rich_text/*_native_cases.rs`) are
constants built from literals: integers, strings, tuples, arrays, struct literals, enum variants
and calls of `const fn` constructors. This module reads those constants without a Rust toolchain,
so that the observation extractor can compare them with a record of the system Msftedit.

Values:
- integers and strings as Python `int` and `str`, `true`/`false` as `bool`;
- tuples and arrays as `list` (a leading `&` is dropped);
- `Name { field: value, ... }` as `Struct(name, fields)`;
- `name(args)` and `Path::Variant(args)` as `Call(name, args)`, with the last path segment as name;
- any other path (`None`, `Primary`, `NativeTarget::Duplicate`) as `Path(name)`, the last segment.
"""

from __future__ import annotations

from dataclasses import dataclass, field
import re


@dataclass(frozen=True)
class Path:
    name: str


@dataclass(frozen=True)
class Call:
    name: str
    args: list = field(hash=False)


@dataclass(frozen=True)
class Struct:
    name: str
    fields: dict = field(hash=False)


TOKEN = re.compile(
    r"""
    (?P<space>\s+)
  | (?P<line_comment>//[^\n]*)
  | (?P<block_comment>/\*.*?\*/)
  | (?P<string>"(?:\\.|[^"\\])*")
  | (?P<number>(?:0x[0-9A-Fa-f_]+|0b[01_]+|0o[0-7_]+|[0-9][0-9_]*)(?:[iu](?:8|16|32|64|128|size))?)
  | (?P<ident>[A-Za-z_][A-Za-z0-9_]*)
  | (?P<path_separator>::)
  | (?P<punct>[()\[\]{},:;&=\-#!<>'.*+/|])
    """,
    re.VERBOSE | re.DOTALL,
)

ESCAPES = {"n": "\n", "r": "\r", "t": "\t", "0": "\0", "\\": "\\", '"': '"', "'": "'"}


def rust_string(literal: str) -> str:
    """The value of a Rust string literal (with its quotes)."""
    result = []
    index = 1
    end = len(literal) - 1
    while index < end:
        char = literal[index]
        index += 1
        if char != "\\":
            result.append(char)
            continue
        escape = literal[index]
        index += 1
        if escape in ESCAPES:
            result.append(ESCAPES[escape])
        elif escape == "u":
            closing = literal.index("}", index)
            result.append(chr(int(literal[index + 1:closing].replace("_", ""), 16)))
            index = closing + 1
        elif escape == "x":
            result.append(chr(int(literal[index:index + 2], 16)))
            index += 2
        elif escape == "\n":
            # A line continuation skips the line break and the leading white space.
            while index < end and literal[index] in " \t\r\n":
                index += 1
        else:
            raise ValueError(f"unsupported string escape \\{escape}")
    return "".join(result)


def rust_integer(literal: str) -> int:
    literal = re.sub(r"[iu](?:8|16|32|64|128|size)$", "", literal).replace("_", "")
    return int(literal, 0)


def tokenize(source: str) -> list[tuple[str, str]]:
    tokens = []
    position = 0
    while position < len(source):
        match = TOKEN.match(source, position)
        if match is None:
            raise ValueError(f"unexpected character {source[position]!r} at offset {position}")
        position = match.end()
        kind = match.lastgroup
        if kind in ("space", "line_comment", "block_comment"):
            continue
        tokens.append((kind, match.group()))
    return tokens


class Parser:
    def __init__(self, tokens: list[tuple[str, str]]):
        self.tokens = tokens
        self.index = 0

    def peek(self, offset: int = 0) -> tuple[str, str]:
        index = self.index + offset
        return self.tokens[index] if index < len(self.tokens) else ("end", "")

    def take(self, value: str | None = None) -> tuple[str, str]:
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
            elif self.peek()[1] != closing:
                raise ValueError(f"expected ',' or {closing!r}, found {self.peek()[1]!r}")
        self.take(closing)
        return values

    def expression(self):
        kind, value = self.peek()
        if value == "&":
            self.take()
            return self.expression()
        if value == "-":
            self.take()
            return -self.expression()
        if value == "[":
            self.take()
            return self.sequence("]")
        if value == "(":
            self.take()
            return self.sequence(")")
        if kind == "string":
            self.take()
            return rust_string(value)
        if kind == "number":
            self.take()
            return rust_integer(value)
        if kind == "ident":
            if value in ("true", "false"):
                self.take()
                return value == "true"
            name = self.take()[1]
            while self.peek()[0] == "path_separator":
                self.take()
                name = self.take()[1]
            if self.peek()[1] == "(":
                self.take()
                return Call(name, self.sequence(")"))
            if self.peek()[1] == "{":
                self.take()
                fields = {}
                while self.peek()[1] != "}":
                    key = self.take()[1]
                    if self.peek()[1] == ":":
                        self.take(":")
                        fields[key] = self.expression()
                    else:
                        raise ValueError(f"field shorthand {key!r} is not supported")
                    if self.peek()[1] == ",":
                        self.take(",")
                self.take("}")
                return Struct(name, fields)
            return Path(name)
        raise ValueError(f"unexpected token {value!r}")


def constants(source: str) -> dict:
    """The `const NAME: Type = value;` items of a source file, by name."""
    tokens = tokenize(source)
    result = {}
    index = 0
    while index < len(tokens):
        kind, value = tokens[index]
        if (
            value == "const"
            and index + 2 < len(tokens)
            and tokens[index + 1][0] == "ident"
            and tokens[index + 2][1] == ":"
        ):
            name = tokens[index + 1][1]
            index += 3
            depth = 0
            # Skip the type up to its `=`; generic brackets and references nest.
            while not (tokens[index][1] == "=" and depth == 0):
                if tokens[index][1] in "([<":
                    depth += 1
                elif tokens[index][1] in ")]>":
                    depth -= 1
                index += 1
            parser = Parser(tokens[index + 1:])
            result[name] = parser.expression()
            parser.take(";")
            index += 1 + parser.index
            continue
        index += 1
    return result
