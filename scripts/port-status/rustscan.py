"""Lightweight, regex/brace based scanner for Rust source files.

It is not a Rust parser. It blanks comments and literals, walks the brace
structure and records, per file:

* type-like items (`struct`, `enum`, `union`, `trait`, `type`, `ferro_class!`)
  with their derives, fields, variants and associated consts;
* functions grouped by the type they belong to (`impl X`, `impl T for X`,
  `trait X`, `ferro_class!` virtuals), including `ferro_property!` definitions;
* trait implementations (`impl Add<Vector> for Point`);
* free functions, consts and statics.

Bodies of functions, `macro_rules!` and `#[cfg(test)] mod tests` are skipped.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field


def blank_comments_and_literals(src: str) -> str:
    """Replaces comments, string and char literals by spaces (newlines kept)."""
    out = []
    i = 0
    n = len(src)
    while i < n:
        c = src[i]
        nxt = src[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            j = src.find("\n", i)
            if j < 0:
                j = n
            out.append(" " * (j - i))
            i = j
        elif c == "/" and nxt == "*":
            depth = 1
            j = i + 2
            while j < n and depth > 0:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            out.append("".join(ch if ch == "\n" else " " for ch in src[i:j]))
            i = j
        elif c == '"' or (c in "rb" and _raw_or_byte_string_start(src, i)):
            j = _skip_string(src, i)
            out.append('"' + "".join(ch if ch == "\n" else " " for ch in src[i + 1:j - 1]) + '"' if j - i >= 2 else " " * (j - i))
            i = j
        elif c == "'":
            # char literal or lifetime
            m = _CHAR_LIT.match(src, i)
            if m:
                out.append(" " * (m.end() - i))
                i = m.end()
            else:
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


_CHAR_LIT = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^\\'\n])'")
_RAW_START = re.compile(r"(?:br|rb|r|b)(#*)\"")
_IDENT_CHAR = re.compile(r"[A-Za-z0-9_]")


def _raw_or_byte_string_start(src: str, i: int) -> bool:
    if i > 0 and _IDENT_CHAR.match(src[i - 1]):
        return False
    return _RAW_START.match(src, i) is not None


def _skip_string(src: str, i: int) -> int:
    n = len(src)
    if src[i] == '"':
        j = i + 1
        while j < n:
            if src[j] == "\\":
                j += 2
            elif src[j] == '"':
                return j + 1
            else:
                j += 1
        return n
    m = _RAW_START.match(src, i)
    assert m
    hashes = m.group(1)
    prefix = src[i:m.end()]
    if "r" in prefix[: len(prefix) - 1 - len(hashes)]:
        end = src.find('"' + hashes, m.end())
        return n if end < 0 else end + 1 + len(hashes)
    # byte string b"..."
    j = m.end()
    while j < n:
        if src[j] == "\\":
            j += 2
        elif src[j] == '"':
            return j + 1
        else:
            j += 1
    return n


@dataclass
class RustType:
    name: str
    kinds: set = field(default_factory=set)        # struct / enum / trait / alias / class / union
    defined: bool = False                           # False when only `impl` blocks were seen
    derives: set = field(default_factory=set)
    fns: set = field(default_factory=set)
    private_fns: set = field(default_factory=set)  # inherent fns without `pub`
    fields: dict = field(default_factory=dict)     # name -> is_pub
    variants: set = field(default_factory=set)
    consts: set = field(default_factory=set)
    properties: set = field(default_factory=set)   # ferro_property! definitions
    traits: list = field(default_factory=list)     # (trait name, trait generic args text)
    line: int = 0

    def merge(self, other: "RustType") -> None:
        self.kinds |= other.kinds
        self.defined = self.defined or other.defined
        self.derives |= other.derives
        self.fns |= other.fns
        self.private_fns |= other.private_fns
        for k, v in other.fields.items():
            self.fields[k] = self.fields.get(k, False) or v
        self.variants |= other.variants
        self.consts |= other.consts
        self.properties |= other.properties
        self.traits += other.traits
        if not self.line:
            self.line = other.line


@dataclass
class RustFile:
    path: str
    types: dict = field(default_factory=dict)       # name -> RustType
    free_fns: set = field(default_factory=set)
    free_consts: set = field(default_factory=set)
    trait_impls: list = field(default_factory=list)  # (trait, args, self type text)
    pub_items: int = 0
    lines: int = 0

    def type(self, name: str) -> RustType:
        t = self.types.get(name)
        if t is None:
            t = self.types[name] = RustType(name)
        return t


_ATTR = re.compile(r"#!?\[(?:[^\[\]]|\[[^\[\]]*\])*\]")
_DERIVE = re.compile(r"#\[\s*derive\s*\(([^)]*)\)")
_VIS = r"(?:pub(?:\s*\([^)]*\))?\s+)?"
_ITEM_HEAD = re.compile(r"^" + _VIS + r"(?:unsafe\s+|auto\s+)*(impl|trait|struct|enum|union|mod)\b\s*(.*)$", re.S)
_FN = re.compile(r"\bfn\s+(r#)?([A-Za-z_]\w*)")
_CONST = re.compile(r"\b(?:const|static)\s+(?:mut\s+)?([A-Za-z_]\w*)\s*:")
_FLAG_CONST = re.compile(r"\bconst\s+([A-Za-z_]\w*)\s*=")
_TYPE_ALIAS = re.compile(r"^" + _VIS + r"type\s+([A-Za-z_]\w*)")
_MACRO = re.compile(r"^(?:[\w:$]+::)?(\w+)!\s*(\w+)?\s*\(?\s*$", re.S)
_MACRO_HEAD = re.compile(r"^(?:[\w:$]+::)?(\w+)!\s*\(?", re.S)
_CLASS_SIMPLE = re.compile(r"\bferro_class!\s*\(\s*([A-Za-z_]\w*)\s*:\s*([A-Za-z_][\w:]*)")
_CLASS_HEAD = re.compile(r"^\s*([A-Za-z_]\w*)\s*:\s*([A-Za-z_][\w:]*)\s*(?:,\s*virtuals\s+([A-Za-z_]\w*))?")
_UNIT_STRUCT = re.compile(r"^" + _VIS + r"struct\s+([A-Za-z_]\w*)")
_TRANSPARENT = ("root", "mod", "macro")
_ITEM_CTX = ("root", "mod", "macro", "impl", "trait", "class_body")


def _strip_generics(s: str) -> str:
    """Removes a leading balanced <...> group."""
    s = s.lstrip()
    if not s.startswith("<"):
        return s
    depth = 0
    i = 0
    while i < len(s):
        ch = s[i]
        if ch == "<":
            depth += 1
        elif ch == ">" and s[i - 1] != "-":
            depth -= 1
            if depth == 0:
                return s[i + 1:]
        i += 1
    return ""


def _split_top(s: str, sep: str) -> list:
    """Splits on `sep` (a word such as ' for ' or ',') outside of <>, () and []."""
    parts = []
    depth = 0
    start = 0
    i = 0
    n = len(s)
    while i < n:
        ch = s[i]
        if ch in "<([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        elif ch == ">" and (i == 0 or s[i - 1] not in "-="):
            depth -= 1
        elif depth == 0 and s.startswith(sep, i):
            parts.append(s[start:i])
            i += len(sep)
            start = i
            continue
        i += 1
    parts.append(s[start:])
    return parts


_TYPE_NAME = re.compile(r"^(?:&\s*(?:'\w+\s+)?(?:mut\s+)?|dyn\s+|\*\s*(?:const|mut)\s+)*((?:[A-Za-z_]\w*\s*::\s*)*)([A-Za-z_]\w*)\s*(.*)$", re.S)


def _type_name(text: str) -> tuple:
    """Returns (last path segment, generic args text) of a type expression."""
    m = _TYPE_NAME.match(text.strip())
    if not m:
        return ("", "")
    return (m.group(2), m.group(3).strip())


def _parse_impl(rest: str) -> tuple:
    """`rest` is the header after the `impl` keyword. Returns (owner, trait, trait args, self text)."""
    rest = _strip_generics(rest)
    rest = re.split(r"\bwhere\b", rest)[0]
    rest = " ".join(rest.split())
    parts = _split_top(rest, " for ")
    if len(parts) >= 2:
        trait_text, self_text = parts[0], parts[-1]
        trait_text = trait_text.lstrip("!? ")
        tname, targs = _type_name(trait_text)
        owner, _ = _type_name(self_text)
        return (owner, tname, targs, self_text.strip())
    owner, _ = _type_name(rest)
    return (owner, None, "", rest.strip())


def _field_names(body: str) -> dict:
    fields = {}
    for part in _split_top(body, ","):
        part = _ATTR.sub(" ", part).strip()
        m = re.match(r"^(pub(?:\s*\([^)]*\))?\s+)?(r#)?([A-Za-z_]\w*)\s*:", part)
        if m:
            fields[m.group(3)] = bool(m.group(1)) or fields.get(m.group(3), False)
    return fields


def _variant_names(body: str) -> set:
    names = set()
    for part in _split_top(body, ","):
        part = _ATTR.sub(" ", part).strip()
        m = re.match(r"^([A-Za-z_]\w*)", part)
        if m:
            names.add(m.group(1))
    return names


def scan_rust(path: str, src: str) -> RustFile:
    rf = RustFile(path)
    rf.lines = src.count("\n") + 1
    text = blank_comments_and_literals(src)
    # stack entries: [kind, owner type name, body start, paren depth to restore, members are public by default]
    stack = [["root", None, 0, 0, False]]
    paren = 0
    seg_start = 0
    n = len(text)
    i = 0
    delim = re.compile(r"[{};()\[\]]")

    def record_decl(seg: str, ctx_kind: str, owner, public: bool = False) -> None:
        """Items terminated by `;` (or found in a header) inside an item context."""
        raw = seg
        seg = _ATTR.sub(" ", seg).strip()
        if not seg:
            return
        if ctx_kind == "struct":
            for m in _FLAG_CONST.finditer(seg):
                rf.type(owner).consts.add(m.group(1))
            return
        if ctx_kind not in _ITEM_CTX:
            return
        m = _CLASS_SIMPLE.search(seg)
        if m:
            t = rf.type(m.group(1))
            t.kinds.add("class")
            t.defined = True
            return
        if re.match(r"^" + _VIS + r"(?:use|extern\s+crate|mod)\b", seg):
            return
        m = _UNIT_STRUCT.match(seg)
        if m:
            t = rf.type(m.group(1))
            t.kinds.add("struct")
            t.defined = True
            for d in _DERIVE.finditer(raw):
                t.derives |= {x.strip().split("::")[-1] for x in d.group(1).split(",") if x.strip()}
            # tuple struct: positional fields are not named
            if seg.startswith("pub"):
                rf.pub_items += 1
            return
        m = _TYPE_ALIAS.match(seg)
        if m and owner is None:
            t = rf.type(m.group(1))
            t.kinds.add("alias")
            t.defined = True
            return
        m = _FN.search(seg)
        if m:
            add_fn(m.group(2), owner, seg, public)
            return
        m = _CONST.search(seg)
        if m:
            if owner is None:
                rf.free_consts.add(m.group(1))
            else:
                rf.type(owner).consts.add(m.group(1))

    def add_fn(name: str, owner, header: str, public: bool = False) -> None:
        is_pub = re.search(r"\bpub\b", header) is not None
        if owner is None:
            rf.free_fns.add(name)
        else:
            rf.type(owner).fns.add(name)
            if not (public or is_pub):
                rf.type(owner).private_fns.add(name)
        if is_pub:
            rf.pub_items += 1

    while i < n:
        m = delim.search(text, i)
        if not m:
            break
        ch = m.group(0)
        pos = m.start()
        i = pos + 1
        ctx = stack[-1]
        if ch in "([":
            paren += 1
            continue
        if ch in ")]":
            paren = max(0, paren - 1)
            continue
        if ch == ";":
            if paren > 0:
                continue
            if ctx[0] in _ITEM_CTX or ctx[0] == "struct":
                record_decl(text[seg_start:pos], ctx[0], ctx[1], ctx[4])
            seg_start = i
            continue
        if ch == "}":
            if len(stack) > 1:
                popped = stack.pop()
                paren = popped[3]
                body = text[popped[2]:pos]
                if popped[0] == "struct" and popped[1]:
                    t = rf.type(popped[1])
                    if not t.consts or ":" in body:
                        for k, v in _field_names(body).items():
                            t.fields[k] = t.fields.get(k, False) or v
                elif popped[0] == "enum" and popped[1]:
                    rf.type(popped[1]).variants |= _variant_names(body)
                elif popped[0] in ("trait", "class_body", "impl"):
                    # trailing declaration without `;`
                    record_decl(text[seg_start:pos], popped[0], popped[1], popped[4])
            seg_start = i
            continue

        # ch == "{"
        raw_header = text[seg_start:pos]
        header = _ATTR.sub(" ", raw_header).strip()
        new_ctx = ["skip", None, i, paren, False]
        kind = ctx[0]
        if kind in _ITEM_CTX:
            owner = ctx[1]
            hm = _ITEM_HEAD.match(header)
            mm = _MACRO_HEAD.match(header)
            if mm and not hm and header[mm.end() - 1:mm.end()] == "(" and ";" in header[mm.end():]:
                # A macro called with parentheses whose content is items, the first block of
                # which opens here (`define_class!( pub struct Name; impl Name { .. } .. );`):
                # the declarations before the block are recorded, and the block is the item
                # its own header says. The items after it are read in the enclosing context.
                parts = header[mm.end():].split(";")
                inner = _ITEM_HEAD.match(parts[-1].strip())
                if inner:
                    for part in parts[:-1]:
                        record_decl(part, "macro", owner, ctx[4])
                    header = parts[-1].strip()
                    hm, mm = inner, None
            if hm and not (mm and not hm):
                kw, rest = hm.group(1), hm.group(2)
                if kw == "impl":
                    o, trait, targs, self_text = _parse_impl(rest)
                    if trait:
                        rf.trait_impls.append((trait, targs, self_text))
                        if o:
                            rf.type(o).traits.append((trait, targs))
                    new_ctx = ["impl", o or None, i, paren, bool(trait)]
                elif kw == "mod":
                    name = rest.split()[0] if rest.split() else ""
                    is_test = name in ("tests", "test") or "cfg(test)" in raw_header.replace(" ", "")
                    new_ctx = ["skip" if is_test else "mod", None, i, paren, False]
                else:
                    nm = re.match(r"([A-Za-z_]\w*)", rest)
                    if nm:
                        t = rf.type(nm.group(1))
                        t.kinds.add({"union": "struct"}.get(kw, kw))
                        t.defined = True
                        if not t.line:
                            t.line = text.count("\n", 0, pos) + 1
                        for d in _DERIVE.finditer(raw_header):
                            t.derives |= {x.strip().split("::")[-1] for x in d.group(1).split(",") if x.strip()}
                        if header.startswith("pub"):
                            rf.pub_items += 1
                        new_ctx = [{"trait": "trait", "enum": "enum"}.get(kw, "struct"), nm.group(1), i, paren, kw == "trait"]
            elif mm and "fn " not in header[mm.end():]:
                macro = mm.group(1)
                if macro == "macro_rules":
                    new_ctx = ["skip", None, i, paren, False]
                elif macro == "ferro_class":
                    new_ctx = ["class_head", None, i, paren, False]
                else:
                    new_ctx = ["macro", owner, i, paren, ctx[4]]
            else:
                fm = _FN.search(header)
                if fm:
                    add_fn(fm.group(2), owner, header, ctx[4])
                    if mm and mm.group(1) == "ferro_property" and owner:
                        rf.type(owner).properties.add(fm.group(2))
                    new_ctx = ["skip", None, i, paren, False]
                else:
                    cm = _CONST.search(header)
                    if cm:
                        if owner is None:
                            rf.free_consts.add(cm.group(1))
                        else:
                            rf.type(owner).consts.add(cm.group(1))
                    new_ctx = ["skip", None, i, paren, False]
        elif kind == "class_head":
            cm = _CLASS_HEAD.match(header)
            if cm:
                t = rf.type(cm.group(1))
                t.kinds.add("class")
                t.defined = True
                if cm.group(3):
                    vt = rf.type(cm.group(3))
                    vt.kinds.add("trait")
                    vt.defined = True
                new_ctx = ["class_body", cm.group(1), i, paren, True]
        elif kind == "enum":
            # struct-like variant: its name is picked up when the enum body is closed
            new_ctx = ["skip", None, i, paren, False]
        stack.append(new_ctx)
        paren = 0
        seg_start = i
    # declarations after the last delimiter are irrelevant
    return rf
