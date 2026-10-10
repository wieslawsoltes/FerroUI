#!/usr/bin/env python3
"""Port status scanner.

Reads the upstream API description produced by scripts/api-extract
(docs/porting/data/upstream-api.json), scans the Rust tree (read-only) and
writes:

  docs/porting/TRACKING.md              master tracking document
  docs/porting/REMAINING.md             what is left to port, in one place
  docs/porting/tracking/<Project>.md    one page per upstream project
  docs/porting/data/port-status.json    machine readable summary

Python 3.11+ (tomllib), standard library only. See scripts/port-status/README.md.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
import re
import sys
import tomllib
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rustscan import RustFile, RustType, scan_rust  # noqa: E402

# --------------------------------------------------------------------------
# Name mapping (docs/porting/PORTING-GUIDE.md)
# --------------------------------------------------------------------------

PRIMITIVES = {
    "double": ["f64"], "float": ["f32"], "int": ["i32"], "uint": ["u32"], "long": ["i64"], "ulong": ["u64"],
    "short": ["i16"], "ushort": ["u16"], "byte": ["u8"], "sbyte": ["i8"], "bool": ["bool"],
    "string": ["String", "str"], "object": ["BoxedValue"], "char": ["char"], "decimal": ["f64"],
    "nint": ["isize"], "nuint": ["usize"], "IntPtr": ["isize"], "Double": ["f64"], "Single": ["f32"],
    "Int32": ["i32"], "String": ["String", "str"],
}


def map_name(name: str) -> str:
    """Upstream identifier -> FerroUI identifier (rule 2 of the porting guide)."""
    return (name.replace("AvNS", "FrnNS").replace("Avalonia", "Ferro").replace("avalonia", "ferro")
            .replace("AVALONIA", "FERRO").replace("Avn", "Frn").replace("avn", "frn").replace("AVN", "FRN"))


def map_project_dir(path: str) -> str:
    parts = []
    for comp in path.split("/"):
        if comp == "Avalonia" or comp.startswith("Avalonia."):
            comp = "FerroUI" + comp[len("Avalonia"):]
        parts.append(comp)
    return "/".join(parts)


def snake(name: str) -> str:
    """PascalCase / camelCase -> snake_case (`Vector3D` -> `vector3d`, `IValueEntry` -> `i_value_entry`)."""
    name = name.lstrip("_") or name
    out = []
    n = len(name)
    for i, c in enumerate(name):
        if c.isupper() and i > 0:
            p = name[i - 1]
            nx = name[i + 1] if i + 1 < n else ""
            if p.islower():
                out.append("_")
            elif p.isdigit():
                if nx.islower():
                    out.append("_")
            elif p.isupper() and nx.islower():
                out.append("_")
        out.append(c.lower() if c.isalnum() or c == "_" else "_")
    return re.sub(r"__+", "_", "".join(out)).strip("_")


def norm(name: str) -> str:
    """Comparison key: case and underscore insensitive."""
    return name.replace("_", "").lower()


# --------------------------------------------------------------------------
# Inputs
# --------------------------------------------------------------------------

def load_upstream(data_dir: str) -> dict:
    with open(os.path.join(data_dir, "upstream-api.json"), encoding="utf-8") as f:
        index = json.load(f)
    if index.get("split"):
        projects = []
        for ref in index["projects"]:
            with open(os.path.join(data_dir, ref["file"]), encoding="utf-8") as f:
                projects.append(json.load(f))
        index["projects"] = projects
    return index


def apply_project_list(index: dict, repo: str) -> None:
    """Overlays the planning data of scripts/api-extract/projects.json (scope, phase, priority, crate) on the
    projects of the extraction, so that moving a project into scope or changing its phase needs no new extraction.
    The detail of a project is not overlaid: it says what the extraction holds."""
    path = os.path.join(repo, "scripts", "api-extract", "projects.json")
    if not os.path.exists(path):
        return
    with open(path, encoding="utf-8") as f:
        config = json.load(f)
    listed = {p["path"]: p for p in config.get("projects", [])}
    # `expanded`: planning data of one project that a `dir/*` entry expands to, where it differs from its siblings
    expanded = {p["path"]: p for p in config.get("expanded", [])}
    for project in index["projects"]:
        entry = listed.get(project["path"])
        if entry is None and project["path"] in expanded:
            entry = expanded[project["path"]]
            fields = ("scope", "phase", "priority")
        elif entry is None:
            # a project of a `dir/*` entry
            parent = project["path"].rsplit("/", 1)[0] + "/*"
            entry = listed.get(parent)
            if entry is None:
                continue
            fields = ("scope", "phase", "priority")
        else:
            fields = ("scope", "phase", "priority", "crate")
        for key in fields:
            if key in entry:
                project[key] = entry[key]
            elif key == "scope":
                project.pop("scope", None)


def load_toml(path: str) -> dict:
    if not os.path.exists(path):
        return {}
    with open(path, "rb") as f:
        return tomllib.load(f)


class Overrides:
    """docs/porting/data/path-overrides.toml"""

    def __init__(self, data: dict):
        self.maps = data.get("map", [])
        self.rust_only = data.get("rust_only", [])
        self.used = set()

    def find(self, project: str, upstream_rel: str):
        for idx, entry in enumerate(self.maps):
            if entry.get("project") not in (None, project):
                continue
            for pattern in entry.get("upstream", []):
                if pattern == upstream_rel or fnmatch.fnmatchcase(upstream_rel, pattern):
                    self.used.add(idx)
                    return entry
        return None

    def rust_only_reason(self, repo_rel: str):
        for entry in self.rust_only:
            if fnmatch.fnmatchcase(repo_rel, entry["path"]):
                return entry.get("reason", "")
        return None


class Waivers:
    """docs/porting/data/member-waivers.toml"""

    def __init__(self, data: dict):
        self.entries = data.get("waive", [])
        self.alias_entries = data.get("alias", [])
        self.hits = Counter()

    def aliases(self, project: str, type_full_name: str) -> dict:
        """Upstream member name -> extra Rust fn names, for one type."""
        out = defaultdict(list)
        for a in self.alias_entries:
            if a.get("project") not in (None, project):
                continue
            tpat = a.get("type", "*")
            if fnmatch.fnmatchcase(type_full_name, tpat) or fnmatch.fnmatchcase(type_full_name.split("<")[0], tpat):
                rust = a["rust"] if isinstance(a["rust"], list) else [a["rust"]]
                out[a["member"]] += rust
        return out

    def find(self, project: str, type_full_name: str, member):
        for idx, w in enumerate(self.entries):
            if w.get("project") not in (None, project):
                continue
            tpat = w.get("type")
            if tpat and not (fnmatch.fnmatchcase(type_full_name, tpat) or fnmatch.fnmatchcase(type_full_name.split("<")[0], tpat)):
                continue
            mpat = w.get("member")
            if member is None:
                if mpat:
                    continue
            else:
                if not mpat:
                    continue
                if not (fnmatch.fnmatchcase(member["name"], mpat) or fnmatch.fnmatchcase(member.get("signature", ""), mpat)):
                    continue
            self.hits[idx] += 1
            return w.get("reason", "waived")
        return None


# --------------------------------------------------------------------------
# Rust tree
# --------------------------------------------------------------------------

AUX_DIRS = {"tests", "examples", "benches", "target", "node_modules"}


class RustCrate:
    def __init__(self, repo: str, root: str, nested_roots=()):
        self.repo = repo
        self.root = root                                # repo-relative
        self.files: dict[str, RustFile] = {}            # crate-relative path -> RustFile
        self.aux: list[str] = []                        # tests, examples, build scripts
        self.by_key: dict[tuple, list[str]] = defaultdict(list)
        self.defined: dict[str, list[str]] = defaultdict(list)
        self.mentions: dict[str, list[str]] = defaultdict(list)
        self.all_paths: set[str] = set()
        abs_root = os.path.join(repo, root)
        self.exists = os.path.isdir(abs_root)
        if not self.exists:
            return
        nested = {os.path.join(repo, n) for n in nested_roots}
        for dirpath, dirnames, filenames in os.walk(abs_root):
            dirnames[:] = sorted(d for d in dirnames if d != "target" and not d.startswith(".") and os.path.join(dirpath, d) not in nested)
            for fn in sorted(filenames):
                rel = os.path.relpath(os.path.join(dirpath, fn), abs_root).replace(os.sep, "/")
                self.all_paths.add(rel)
                if not fn.endswith(".rs"):
                    continue
                comps = rel.split("/")
                # Test files are `<file>_tests.rs` (porting guide rule 1). A name that ends in `_test.rs` is not
                # one: `render_data_stream_hit_test.rs` and `i_custom_hit_test.rs` are ported files.
                if AUX_DIRS & set(comps[:-1]) or fn == "build.rs" or fn.endswith("_tests.rs"):
                    self.aux.append(rel)
                    continue
                try:
                    with open(os.path.join(dirpath, fn), encoding="utf-8") as f:
                        src = f.read()
                except (OSError, UnicodeDecodeError):
                    continue
                rf = scan_rust(rel, src)
                self.files[rel] = rf
                key = (tuple(norm(c) for c in comps[:-1]), norm(fn[:-3]))
                self.by_key[key].append(rel)
                for name, t in rf.types.items():
                    self.mentions[name].append(rel)
                    if t.defined:
                        self.defined[name].append(rel)

    def lookup(self, dir_comps: tuple, stem: str):
        hits = self.by_key.get((tuple(norm(c) for c in dir_comps), norm(stem)))
        return hits[0] if hits else None


class TypeView:
    """Everything known about one Rust type name across a set of files."""

    SUFFIXES = ("", "Impl", "Ext", "ImplExt", "Dyn")

    def __init__(self, crate: RustCrate, names: list[str], files: list[str]):
        self.type = RustType(names[0] if names else "")
        self.free_fns: set[str] = set()
        self.free_consts: set[str] = set()
        self.trait_impls: list = []
        self.names = names
        for rel in files:
            rf = crate.files.get(rel)
            if rf is None:
                continue
            self.free_fns |= rf.free_fns
            self.free_consts |= rf.free_consts
            self.trait_impls += rf.trait_impls
            for name in names:
                for suffix in self.SUFFIXES:
                    t = rf.types.get(name + suffix)
                    if t is not None:
                        self.type.merge(t)
        t = self.type
        self.fn_norm = {norm(f): f for f in t.fns}
        self.field_norm = {norm(f): f for f in t.fields}
        self.const_norm = {norm(f): f for f in (t.consts | t.variants)}
        self.free_norm = {norm(f): f for f in (self.free_fns | self.free_consts)}
        # fns of every trait declared in the same files: abstract/virtual members often move to a helper trait
        self.file_trait_fns = {}
        for rel in files:
            rf = crate.files.get(rel)
            if rf is None:
                continue
            for tname, rt in rf.types.items():
                if "trait" in rt.kinds:
                    for f in rt.fns:
                        self.file_trait_fns.setdefault(norm(f), f"{tname}::{f}")
        self.trait_names = {tr[0] for tr in t.traits} | t.derives

    def has_trait(self, *names: str) -> bool:
        return any(n in self.trait_names for n in names)

    def trait_impl_count(self, trait: str) -> int:
        words = set(self.names)
        count = 0
        for tname, targs, self_text in self.trait_impls:
            if tname != trait:
                continue
            if words & set(re.findall(r"[A-Za-z_]\w*", self_text + " " + targs)) or "Self" in targs:
                count += 1
        return count


# --------------------------------------------------------------------------
# Matching
# --------------------------------------------------------------------------

ACCESS_RANK = {"public": 0, "explicit": 1, "protected": 1, "protected internal": 1, "internal": 2, "private protected": 3, "static": 2}

BINARY_OPS = {
    "+": ["Add"], "-": ["Sub"], "*": ["Mul"], "/": ["Div"], "%": ["Rem"], "&": ["BitAnd"], "|": ["BitOr"], "^": ["BitXor"],
    "<<": ["Shl"], ">>": ["Shr"], "==": ["PartialEq", "Eq"], "!=": ["PartialEq", "Eq"],
    "<": ["PartialOrd", "Ord"], ">": ["PartialOrd", "Ord"], "<=": ["PartialOrd", "Ord"], ">=": ["PartialOrd", "Ord"],
}
UNARY_OPS = {"-": ["Neg"], "!": ["Not"], "~": ["Not"], "+": []}

# Methods that the porting guide maps to a trait instead of a method.
TRAIT_METHODS = {
    "ToString": ["Display"],
    "Equals": ["PartialEq", "Eq"],
    "GetHashCode": ["Hash"],
    "CompareTo": ["PartialOrd", "Ord"],
    "Clone": ["Clone"],
    "GetEnumerator": ["IntoIterator", "Iterator"],
    "Parse": ["FromStr"],
    "TryParse": ["FromStr"],
    "Dispose": ["Drop"],
}
# Alternative method names accepted for a C# method.
METHOD_ALIASES = {
    "TryParse": ["try_parse", "parse"],
    "GetEnumerator": ["iter", "into_iter"],
    "Deconstruct": ["deconstruct"],
}
CTOR_NAMES = ("new", "construct", "create", "default")


def type_words(type_text: str) -> set[str]:
    """Rust-side words that may stand for a C# type expression."""
    words = set()
    for w in re.findall(r"[A-Za-z_]\w*", type_text):
        words.add(map_name(w))
        for p in PRIMITIVES.get(w, []):
            words.add(p)
    return words


def match_members(utype: dict, view: TypeView, aliases: dict | None = None) -> list:
    """Returns [(member, status, how)] for the members of one upstream type. status: present | missing."""
    members = utype.get("members", [])
    results: dict[int, tuple] = {}
    uname = utype["name"]

    # names that are taken by an exact upstream member and must not satisfy an overload of another one
    reserved = set()
    for m in members:
        s = norm(snake(m["name"]))
        reserved.add(s)
        if m["kind"] == "property":
            reserved.add("set" + s)
            reserved.add("get" + s)

    aliases = aliases or {}
    groups: dict[tuple, list[int]] = defaultdict(list)
    for idx, m in enumerate(members):
        k = m["kind"]
        if m.get("explicitInterface") and k in ("method", "property", "event", "indexer"):
            # explicit interface implementations are matched by name only: in Rust they are the
            # same-named fn of a trait impl and do not count as one more overload
            groups[("explicit", idx)].append(idx)
        elif k == "method":
            groups[("method", m["name"])].append(idx)
        elif k == "ctor":
            groups[("ctor", "")].append(idx)
        elif k == "operator" and "conversion" not in m:
            groups[("operator", m["name"], min(m.get("params", 2), 2))].append(idx)
        else:
            groups[(k, idx)].append(idx)

    t = view.type
    for key, idxs in groups.items():
        kind = key[0]
        idxs.sort(key=lambda i: (ACCESS_RANK.get(members[i]["accessibility"], 2), i))
        if kind == "method":
            name = key[1]
            s = snake(name)
            pool = []
            if norm(s) in view.fn_norm:
                pool.append(("fn", view.fn_norm[norm(s)]))
            for alias in METHOD_ALIASES.get(name, []):
                if not pool and norm(alias) in view.fn_norm:
                    pool.append(("fn", view.fn_norm[norm(alias)]))
            for f in aliases.get(name, []):
                if f in t.fns and ("fn", f) not in pool:
                    pool.append(("alias fn", f))
                elif f not in t.fns and f in view.free_fns and ("free fn", f) not in pool:
                    pool.append(("alias free fn", f))
            for f in sorted(t.fns):
                if f.startswith(s + "_") and norm(f) not in reserved and f not in t.private_fns and all(f != p[1] for p in pool):
                    pool.append(("overload fn", f))
            traits = TRAIT_METHODS.get(name)
            if traits and view.has_trait(*traits):
                # the trait stands for every overload, except for ToString where it is the parameterless one
                hit = "/".join(tr for tr in traits if tr in view.trait_names)
                covered = [i for i in idxs if name != "ToString" or members[i].get("params", 0) == 0]
                for i in covered:
                    results[i] = ("present", f"trait `{hit}`")
                idxs = [i for i in idxs if i not in covered]
                pool = [p for p in pool if p[1] not in ("to_string", "fmt")]
            if not pool and norm(s) in view.free_norm:
                pool.append(("free fn", view.free_norm[norm(s)]))
            if not pool and norm(s) in view.file_trait_fns and any(
                    {"abstract", "virtual", "override"} & set(members[i].get("modifiers", [])) for i in idxs):
                pool.append(("trait fn", view.file_trait_fns[norm(s)]))
            if not pool and s.startswith("get_") and len(idxs) == 1 and members[idxs[0]].get("params", 0) == 0:
                # parameterless `GetFoo()` ported as the getter `foo()`
                bare = norm(s[4:])
                if bare in view.fn_norm and bare not in reserved:
                    pool.append(("getter fn", view.fn_norm[bare]))
            if not pool and norm(s) in view.field_norm:
                pool.append(("field", view.field_norm[norm(s)]))
            for n, i in enumerate(idxs):
                if n < len(pool):
                    results[i] = ("present", f"{pool[n][0]} `{pool[n][1]}`")
                else:
                    results[i] = ("missing", f"{len(pool)} of {len(idxs)} overloads found" if pool else "")
        elif kind == "ctor":
            pool = []
            seen = set()
            ordered = [f for f in ("new", "construct") if f in t.fns]
            ordered += sorted(f for f in t.fns if f.startswith("new_"))
            ordered += sorted(f for f in t.fns if f in ("create", "default") or (f.startswith("from_") and f != "from_str"))
            for f in ordered:
                if norm(f) in reserved or (f in t.private_fns and f not in ("new", "construct")):
                    continue
                canonical = "new" if f == "construct" else f
                if canonical in seen:
                    continue
                seen.add(canonical)
                pool.append(("fn", f))
            for f in aliases.get(".ctor", []):
                # a constructor the port names after what it takes (`with_parent`): an alias names the function
                if f in t.fns and f not in seen:
                    seen.add(f)
                    pool.append(("alias fn", f))
            if view.has_trait("Default") and "default" not in seen:
                pool.append(("trait", "Default"))
            for tname, targs in t.traits:
                if tname in ("From", "TryFrom"):
                    pool.append(("trait", f"{tname}{targs}"))
            for n, i in enumerate(idxs):
                if n < len(pool):
                    results[i] = ("present", f"{pool[n][0]} `{pool[n][1]}`")
                else:
                    results[i] = ("missing", f"{len(pool)} of {len(idxs)} constructors found" if pool else "")
        elif kind == "operator" and len(key) == 3:
            token = key[1].split(" ", 1)[1]
            arity = key[2]
            traits = (UNARY_OPS if arity == 1 else BINARY_OPS).get(token, [])
            if any(tr in t.derives for tr in traits):
                for i in idxs:
                    results[i] = ("present", "derive")
                continue
            count = 0
            hit = ""
            for tr in traits:
                c = view.trait_impl_count(tr)
                if c:
                    count += c
                    hit = tr
            for n, i in enumerate(idxs):
                if n < count or (count and token in ("==", "!=", "<", ">", "<=", ">=")):
                    results[i] = ("present", f"trait `{hit}`")
                else:
                    results[i] = ("missing", f"{count} of {len(idxs)} `{traits[0] if traits else token}` impls found" if count else "")
        elif kind == "explicit":
            i = idxs[0]
            m = members[i]
            if m["kind"] == "method":
                s = snake(m["name"])
                traits = TRAIT_METHODS.get(m["name"])
                if norm(s) in view.fn_norm:
                    results[i] = ("present", f"fn `{view.fn_norm[norm(s)]}`")
                elif traits and view.has_trait(*traits):
                    results[i] = ("present", "trait `" + "/".join(tr for tr in traits if tr in view.trait_names) + "`")
                elif any(norm(a) in view.fn_norm for a in METHOD_ALIASES.get(m["name"], [])):
                    results[i] = ("present", "fn `" + next(view.fn_norm[norm(a)] for a in METHOD_ALIASES[m["name"]] if norm(a) in view.fn_norm) + "`")
                else:
                    results[i] = alias_single(m, view, aliases) or ("missing", "")
            else:
                results[i] = match_single(m, uname, view, aliases)
        else:
            i = idxs[0]
            results[i] = match_single(members[i], uname, view, aliases)

    return [(m, *results[i]) for i, m in enumerate(members)]


def alias_single(m: dict, view: TypeView, aliases: dict) -> tuple | None:
    """A member that is not an overload set (a property, a field, an event, an indexer, an explicit interface
    member, a static constructor) and that the port has under a name an `[[alias]]` gives: present when an item
    of that name exists on the type or, for a type ported as a module, in its file."""
    t = view.type
    for f in aliases.get(m["name"], []):
        if f in t.fns:
            return ("present", f"alias fn `{f}`")
        if f in t.fields:
            return ("present", f"alias field `{f}`")
        if f in t.consts or f in t.variants:
            return ("present", f"alias const `{f}`")
        if f in view.free_fns or f in view.free_consts:
            return ("present", f"alias free item `{f}`")
    return None


def match_single(m: dict, uname: str, view: TypeView, aliases: dict | None = None) -> tuple:
    result = match_single_by_rule(m, uname, view)
    if result[0] == "missing" and aliases:
        return alias_single(m, view, aliases) or result
    return result


def match_single_by_rule(m: dict, uname: str, view: TypeView) -> tuple:
    kind = m["kind"]
    t = view.type
    if kind == "operator":  # conversion
        ptypes = m.get("paramTypes", [""])
        src, dst = (ptypes[0] if ptypes else ""), m.get("type", "")
        self_words = set(view.names) | {"Self"}
        other = dst if re.sub(r"[<?].*", "", src) == uname else src
        other_words = type_words(other) - {map_name(uname)}
        for tname, targs, self_text in view.trait_impls:
            if tname not in ("From", "Into", "TryFrom", "TryInto"):
                continue
            words = set(re.findall(r"[A-Za-z_]\w*", self_text + " " + targs))
            if words & self_words and (words & other_words or not other_words):
                return ("present", f"trait `{tname}{targs} for {self_text}`")
        for w in sorted(other_words):
            for prefix in ("to_", "from_", "as_", "into_"):
                if norm(prefix + snake(w)) in view.fn_norm:
                    return ("present", f"fn `{view.fn_norm[norm(prefix + snake(w))]}`")
        return ("missing", "")
    if kind == "static ctor":
        for f in ("static_constructor", "class_init", "static_init"):
            if f in t.fns:
                return ("present", f"fn `{f}`")
        return ("missing", "")
    if kind == "finalizer":
        return ("present", "trait `Drop`") if view.has_trait("Drop") else ("missing", "")
    if kind == "enum member":
        key = norm(m["name"])
        if key in view.const_norm:
            return ("present", f"variant `{view.const_norm[key]}`")
        return ("missing", "")
    if kind == "indexer":
        if view.has_trait("Index", "IndexMut"):
            return ("present", "trait `Index`")
        for f in ("get", "get_item", "item", "get_at", "at", "index"):
            if f in t.fns:
                return ("present", f"fn `{f}`")
        return ("missing", "")
    if kind == "event":
        s = snake(m["name"])
        for cand in (s, "add_" + s, "subscribe_" + s, s + "_event"):
            if norm(cand) in view.fn_norm:
                return ("present", f"fn `{view.fn_norm[norm(cand)]}`")
        if norm(s) in view.field_norm:
            return ("present", f"field `{view.field_norm[norm(s)]}`")
        return ("missing", "")
    if kind in ("property", "field"):
        s = snake(m["name"])
        key = norm(s)
        accessors = m.get("accessors", {"get": m["accessibility"]})
        needs_get = "get" in accessors or kind == "field"
        needs_set = kind == "property" and accessors.get("set") not in (None, "private", "private protected")
        has_setter = norm("set_" + s) in view.fn_norm
        if needs_get:
            if key in view.fn_norm or norm("get_" + s) in view.fn_norm:
                getter = view.fn_norm.get(key) or view.fn_norm[norm("get_" + s)]
                if needs_set and not has_setter:
                    return ("missing", f"getter `{getter}` found, setter `set_{s}` missing")
                return ("present", f"fn `{getter}`")
            if key in view.field_norm:
                return ("present", f"field `{view.field_norm[key]}`")
            if key in view.const_norm:
                return ("present", f"const `{view.const_norm[key]}`")
            if key in view.free_norm:
                return ("present", f"free item `{view.free_norm[key]}`")
            return ("missing", "")
        if has_setter:
            return ("present", f"fn `set_{s}`")
        return ("missing", "")
    return ("missing", "")


# --------------------------------------------------------------------------
# Project scan
# --------------------------------------------------------------------------

def flatten_types(types: list, out=None) -> list:
    out = [] if out is None else out
    for t in types:
        out.append(t)
        flatten_types(t.get("nestedTypes", []), out)
    return out


def rust_candidates(rel: str) -> list:
    """Candidate (dir components, stem, note) for an upstream file, in order of preference."""
    comps = rel.split("/")
    dirs = tuple(snake(map_name(c)) for c in comps[:-1])
    stem = comps[-1].rsplit(".", 1)[0]
    note = ""
    if "`" in stem:
        stem = re.sub(r"`\d+", "", stem)
        note = "generic arity merged"
    stem = map_name(stem)
    parts = [p for p in stem.split(".") if p]
    cands = []
    full = snake("_".join(snake(p) for p in parts))
    cands.append((dirs, full, note))
    if len(parts) > 1:
        cands.append((dirs, snake(parts[0]), (note + "; " if note else "") + "partial merged into main file"))
    base = parts[0] if parts else stem
    if len(parts) == 1 and re.match(r"^I[A-Z][a-z0-9]", base):
        cands.append((dirs, snake(base[1:]), (note + "; " if note else "") + "interface merged into implementation file"))
    return cands


def scan_project(project: dict, repo: str, overrides: Overrides, waivers: Waivers, nested_roots) -> dict:
    name = project["name"]
    rust_root = project.get("rust") or map_project_dir(project["path"])
    crate = RustCrate(repo, rust_root, [n for n in nested_roots if n != rust_root and n.startswith(rust_root + "/")])
    detail = project["detail"]
    result = {
        "name": name, "path": project["path"], "rust": rust_root, "detail": detail, "scope": project.get("scope", "in"),
        "crate": project.get("crate") or ("ferroui-" + map_project_dir(project["path"]).split("/")[-1].split(".", 1)[-1].lower().replace(".", "-")),
        "phase": project.get("phase", ""), "priority": project.get("priority", ""),
        "rustExists": crate.exists, "files": [], "extra": {}, "idl": [], "how": Counter(),
    }
    mapped_rust: set[str] = set()

    if detail == "files":
        for f in project["files"]:
            target = map_name(f["path"])
            note = ""
            ov = overrides.find(name, f["path"])
            if ov is not None and ov.get("kind") == "not-applicable":
                result["files"].append({"path": f["path"], "expected": "-", "rust": [], "status": "n/a",
                                        "note": "not-applicable: " + ov.get("reason", ""), "types": []})
                continue
            if ov is not None and ov.get("rust"):
                target = ov["rust"] if isinstance(ov["rust"], str) else ov["rust"][0]
                note = ov.get("kind", "renamed") + (": " + ov["reason"] if ov.get("reason") else "")
            exists = target in crate.all_paths
            if exists:
                mapped_rust.add(target)
            result["files"].append({"path": f["path"], "expected": target, "rust": [target] if exists else [],
                                    "status": "present" if exists else "missing", "note": note, "types": []})
        result["rustOnly"] = []
        result["rustAux"] = []
        return result

    for f in project["files"]:
        rel = f["path"]
        fr = {"path": rel, "types": [], "note": "", "rust": [], "lines": f.get("lines", 0)}
        types = flatten_types(f.get("types", []))
        cands = rust_candidates(rel)
        fr["expected"] = "/".join(cands[0][0] + (cands[0][1] + ".rs",))
        ov = overrides.find(name, rel)
        members_mode = "scan"
        if ov is not None:
            kind = ov.get("kind", "merged")
            fr["note"] = kind + (": " + ov["reason"] if ov.get("reason") else "")
            members_mode = ov.get("members", "waive" if kind in ("replaced", "not-applicable") else "scan")
            fr["typeMap"] = ov.get("types", {})
            if kind == "not-applicable":
                fr["status"] = "n/a"
            else:
                targets = ov["rust"] if isinstance(ov.get("rust"), list) else [ov["rust"]]
                fr["expected"] = targets[0]
                fr["rust"] = [x for x in targets if x in crate.files or x in crate.all_paths]
                fr["status"] = "present" if fr["rust"] else "missing"
        else:
            for dirs, stem, note in cands:
                hit = crate.lookup(dirs, stem)
                if hit and "interface merged" in note:
                    # only when the implementation file really declares the interface
                    defined = {n for n, t in crate.files[hit].types.items() if t.defined}
                    if not any(map_name(ut["name"]) in defined for ut in types):
                        hit = None
                if hit:
                    fr["rust"] = [hit]
                    fr["note"] = note
                    break
            fr["status"] = "present" if fr["rust"] else "missing"
            if not types and fr["status"] == "missing":
                fr["status"] = "n/a"
                fr["note"] = "no non-private types (assembly attributes, global usings or file-local helpers)"
        mapped_rust.update(fr["rust"])

        for ut in types:
            # A project in scope whose extraction holds no members has its types looked for all the same.
            type_detail = "types-scan" if detail == "types" and result["scope"] == "in" else detail
            tr = scan_type(ut, name, crate, fr, members_mode, waivers, type_detail, result["how"])
            fr["types"].append(tr)
        if fr["status"] == "missing":
            # types found elsewhere in the crate: suggest an override
            found = sorted({tr["where"] for tr in fr["types"] if tr["status"] == "present" and tr.get("where")})
            if found:
                fr["note"] = "types found in " + ", ".join(f"`{x}`" for x in found) + " (add to path-overrides.toml)"
                fr["hint"] = found
        result["files"].append(fr)

    # other (non C#) files
    for label, paths in sorted(project.get("extraFiles", {}).items()):
        entries = []
        for p in paths:
            target = map_name(p)
            entries.append({"path": p, "expected": target, "status": "present" if target in crate.all_paths else "missing"})
        result["extra"][label] = entries

    # IDL contracts
    for n, idl in enumerate(project.get("idl", [])):
        rust_idl_names = project.get("rustIdl", [])
        rust_idl = rust_idl_names[n] if n < len(rust_idl_names) else map_name(idl["file"])
        rust_path = os.path.join(repo, rust_root, rust_idl)
        rust_ifaces = {}
        rust_enums = {}
        rust_structs = set()
        if os.path.exists(rust_path):
            with open(rust_path, encoding="utf-8") as fh:
                text = fh.read()
            text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
            text = re.sub(r"//[^\n]*", " ", text)
            for m in re.finditer(r"\b(interface|enum|struct)\s+(\w+)\s*(?::\s*\w+)?\s*\{([^{}]*)\}", text):
                if m.group(1) == "interface":
                    rust_ifaces[m.group(2)] = Counter(re.findall(r"(\w+)\s*\(", m.group(3)))
                elif m.group(1) == "enum":
                    rust_enums[m.group(2)] = set(re.findall(r"(?:^|,)\s*(\w+)", m.group(3)))
                else:
                    rust_structs.add(m.group(2))
        entry = {"file": idl["file"], "rust": rust_idl, "rustExists": os.path.exists(rust_path), "interfaces": [], "enums": [], "structs": []}
        for iface in idl["interfaces"]:
            target = map_name(iface["name"])
            have = rust_ifaces.get(target)
            want = Counter(map_name(m["name"]) for m in iface["methods"])
            present = sum(min(c, have.get(k, 0)) for k, c in want.items()) if have is not None else 0
            missing = sorted(k for k, c in want.items() if have is None or have.get(k, 0) < c)
            entry["interfaces"].append({"name": iface["name"], "rust": target, "status": "present" if have is not None else "missing",
                                        "methods": len(iface["methods"]), "present": present, "missing": missing})
        for en in idl["enums"]:
            target = map_name(en["name"])
            have = rust_enums.get(target)
            want = [map_name(x) for x in en["members"]]
            entry["enums"].append({"name": en["name"], "rust": target, "status": "present" if have is not None else "missing",
                                   "members": len(want), "present": sum(1 for x in want if have and x in have)})
        for st in idl["structs"]:
            target = map_name(st["name"])
            entry["structs"].append({"name": st["name"], "rust": target, "status": "present" if target in rust_structs else "missing"})
        result["idl"].append(entry)

    # Rust files without an upstream counterpart
    rust_only = []
    for rel in sorted(crate.files):
        if rel in mapped_rust:
            continue
        base = rel.rsplit("/", 1)[-1]
        if base in ("mod.rs", "lib.rs", "main.rs"):
            continue
        repo_rel = rust_root + "/" + rel
        reason = overrides.rust_only_reason(repo_rel)
        rf = crate.files[rel]
        rust_only.append({"path": rel, "reason": reason, "types": sorted(n for n, t in rf.types.items() if t.defined)})
    result["rustOnly"] = rust_only
    result["rustAux"] = sorted(crate.aux)
    return result


def scan_type(ut: dict, project: str, crate: RustCrate, fr: dict, members_mode: str, waivers: Waivers, detail: str, how: Counter) -> dict:
    uname = ut["name"]
    names = [map_name(uname)]
    renamed = fr.get("typeMap", {}).get(uname)
    if renamed:
        names.insert(0, renamed)
    outer = ut.get("outer")
    if outer:
        outer_flat = "".join(re.sub(r"<.*?>", "", p) for p in outer.split("."))
        names.append(map_name(outer_flat) + map_name(uname))
    display = (outer + "." if outer else "") + uname + (f"<{', '.join(ut['typeParameters'])}>" if ut.get("typeParameters") else "")
    members = ut.get("members", [])
    tr = {
        "name": display, "kind": ut["kind"], "fullName": ut["fullName"], "accessibility": ut["accessibility"],
        "status": "missing", "where": None, "members": [], "memberCount": len(members) if detail == "full" else ut.get("memberCount", 0),
        "static": "static" in ut.get("modifiers", []),
    }
    if fr["status"] == "n/a":
        tr["status"] = "n/a"
        tr["members"] = [{"m": m, "status": "n/a", "how": ""} for m in members]
        return tr
    if detail not in ("full", "types-scan"):
        return tr

    in_files = [rel for rel in fr["rust"] if rel in crate.files]
    found_name = None
    where = None
    for cand in names:
        for rel in in_files:
            t = crate.files[rel].types.get(cand)
            if t is not None and t.defined:
                found_name, where = cand, None
                break
        if found_name:
            break
    if not found_name:
        # crate-wide fallback (top-level types by their own name, nested types by Outer+Inner only)
        fallback = names[1:] if outer else names
        for cand in fallback:
            if crate.defined.get(cand):
                found_name, where = cand, crate.defined[cand][0]
                break

    view_files = list(in_files)
    if found_name:
        definers = crate.defined.get(found_name, [])
        if len(definers) <= 1:
            for rel in crate.mentions.get(found_name, []):
                if rel not in view_files:
                    view_files.append(rel)
        elif where and where not in view_files:
            view_files.append(where)
    view = TypeView(crate, [found_name] if found_name else names, view_files)

    aliases = waivers.aliases(project, ut["fullName"])
    for m in members:
        # Rule 2 of the porting guide renames the framework in every identifier, members included:
        # `AvaloniaPropertyType` is looked for as `ferro_property_type` too.
        renamed = map_name(m["name"])
        if renamed != m["name"]:
            s = snake(renamed)
            for cand in (s, s.upper()):
                if cand not in aliases[m["name"]]:
                    aliases[m["name"]].append(cand)
    results = match_members(ut, view, aliases) if members else []
    if not found_name and tr["static"] and in_files and any(st == "present" for _, st, _ in results):
        # static class ported as free functions / constants of the module
        found_name = names[0]
        tr["asModule"] = True
    if not found_name and detail == "types-scan" and tr["static"] and in_files:
        # Without the members of the type there is nothing to look for in the module: a static class
        # counts as ported with its file.
        found_name = names[0]
        tr["asModule"] = True
    if not found_name and ut["kind"] == "delegate" and in_files:
        pass

    if found_name:
        tr["status"] = "present"
        tr["where"] = where
        tr["rustName"] = found_name
    else:
        reason = waivers.find(project, ut["fullName"], None)
        if reason or members_mode == "waive":
            tr["status"] = "waived"
            tr["reason"] = reason or fr["note"]

    for m, status, note in results:
        if not found_name:
            status, note = "missing", ""
        if status == "present":
            how[note.split(" `")[0]] += 1
        else:
            reason = waivers.find(project, ut["fullName"], m)
            if reason is None and (members_mode == "waive" or tr["status"] == "waived"):
                reason = tr.get("reason") or fr["note"]
            if reason is not None:
                status, note = "waived", reason
        tr["members"].append({"m": m, "status": status, "how": note})
    return tr


# --------------------------------------------------------------------------
# Counting
# --------------------------------------------------------------------------

def empty_counts() -> dict:
    return {"files": [0, 0, 0], "types": [0, 0, 0], "members": [0, 0, 0], "properties": [0, 0], "events": [0, 0], "interfaces": [0, 0], "na_files": 0}


def add_counts(a: dict, b: dict) -> None:
    for k, v in b.items():
        if isinstance(v, list):
            for i, x in enumerate(v):
                a[k][i] += x
        else:
            a[k] += v


def file_counts(fr: dict, detail: str) -> dict:
    """[present, total (without n/a), waived]"""
    c = empty_counts()
    if fr["status"] == "n/a":
        c["na_files"] = 1
        return c
    c["files"][1] = 1
    if fr["status"] == "present":
        c["files"][0] = 1
    for tr in fr["types"]:
        c["types"][1] += 1
        if tr["status"] == "present":
            c["types"][0] += 1
        elif tr["status"] == "waived":
            c["types"][2] += 1
        if tr["kind"] == "interface":
            c["interfaces"][1] += 1
            c["interfaces"][0] += tr["status"] == "present"
        if detail != "full":
            c["members"][1] += tr["memberCount"]
            continue
        for mr in tr["members"]:
            c["members"][1] += 1
            if mr["status"] == "present":
                c["members"][0] += 1
            elif mr["status"] == "waived":
                c["members"][2] += 1
            m = mr["m"]
            if m.get("avaloniaProperty"):
                c["properties"][1] += 1
                c["properties"][0] += mr["status"] == "present"
            if m.get("routedEvent"):
                c["events"][1] += 1
                c["events"][0] += mr["status"] == "present"
    return c


def pct(present: int, total: int, waived: int = 0) -> str:
    denom = total - waived
    if denom <= 0:
        return "-"
    return f"{100.0 * present / denom:.1f}%"


def ratio(c: list) -> str:
    s = f"{c[0]}/{c[1]}"
    if len(c) > 2 and c[2]:
        s += f" ({c[2]} waived)"
    return s


# --------------------------------------------------------------------------
# Rendering
# --------------------------------------------------------------------------

def esc(text: str) -> str:
    return text.replace("|", "\\|").replace("<", "&lt;").replace(">", "&gt;")


def code(text: str) -> str:
    """Inline code that is safe inside a markdown table."""
    text = text.replace("|", "\\|")
    if "`" in text:
        return "`` " + text + " ``"
    return "`" + text + "`"


def page_name(project_name: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", project_name).strip("_") + ".md"


def member_label(mr: dict) -> str:
    m = mr["m"]
    sig = m.get("signature") or m["name"]
    if len(sig) > 110:
        sig = sig[:107] + "..."
    label = code(sig)
    acc = m["accessibility"]
    extra = []
    if acc not in ("public",):
        extra.append(acc)
    if mr["how"] and mr["status"] == "missing":
        extra.append(mr["how"])
    if extra:
        label += " *(" + "; ".join(extra) + ")*"
    return label


def render_project(pr: dict, counts: dict, commit: str) -> str:
    detail = pr["detail"]
    L = []
    w = L.append
    w(f"# {pr['name']}")
    w("")
    w("<!-- Generated by scripts/port-status/run.sh. Do not edit: change the code, docs/porting/data/path-overrides.toml or member-waivers.toml and regenerate. -->")
    w("")
    w(f"Generated from upstream commit `{commit}`. Back to [TRACKING.md](../TRACKING.md).")
    w("")
    w("| | |")
    w("|---|---|")
    w(f"| Upstream | `{pr['path']}` |")
    w(f"| FerroUI | `{pr['rust']}` ({'exists' if pr['rustExists'] else 'not created yet'}) |")
    w(f"| Crate | {code(pr['crate']) if not pr['crate'].startswith('(') else pr['crate']} |")
    w(f"| Phase / priority | {pr['phase']} / {pr['priority']} |")
    w(f"| Files | {ratio(counts['files'])} ({pct(*counts['files'])}){', ' + str(counts['na_files']) + ' not applicable' if counts['na_files'] else ''} |")
    if detail != "files":
        w(f"| Types | {ratio(counts['types'])} ({pct(*counts['types'])}) |")
        w(f"| Members | {ratio(counts['members'])} ({pct(*counts['members'])}) |")
    if detail == "full":
        w(f"| Contracts (interfaces) | {ratio(counts['interfaces'])} |")
        w(f"| Property registrations | {ratio(counts['properties'])} |")
        w(f"| Routed events | {ratio(counts['events'])} |")
    w("")
    if detail == "types" and pr["scope"] == "in":
        w("This backend is being ported. It is tracked at file and type granularity: the members of its types have not been extracted from upstream yet, so member counts are totals and the member column stays at zero.")
        w("")
    elif detail == "types":
        w("This backend is outside the current porting scope. It is tracked at file and type granularity only; member counts are totals.")
        w("")
    if detail == "files":
        w("Non-C# sources, tracked as a plain file list (names mapped with the rename rules of `scripts/sync-native.sh`).")
        w("")

    # contracts
    if detail == "full":
        ifaces = []
        for fr in pr["files"]:
            for tr in fr["types"]:
                if tr["kind"] == "interface":
                    ifaces.append((tr["fullName"], fr, tr))
        if ifaces:
            ifaces.sort(key=lambda x: (x[0], x[1]["path"]))
            w("## Contracts")
            w("")
            w("Every interface of the project. Each becomes a `pub trait` with the same name (the `I` prefix is kept).")
            w("")
            w("| Interface | Access | Upstream file | Members | Status |")
            w("|---|---|---|---|---|")
            for full, fr, tr in ifaces:
                present = sum(1 for mr in tr["members"] if mr["status"] == "present")
                waived = sum(1 for mr in tr["members"] if mr["status"] == "waived")
                total = len(tr["members"])
                status = tr["status"]
                if status == "present":
                    status = "present" if present + waived == total else "partial"
                mtxt = f"{present}/{total}" + (f" ({waived} waived)" if waived else "")
                w(f"| {code(full)} | {tr['accessibility']} | `{fr['path']}` | {mtxt} | {status} |")
            w("")

    # files by directory
    by_dir = defaultdict(list)
    for fr in pr["files"]:
        d = fr["path"].rsplit("/", 1)[0] if "/" in fr["path"] else ""
        by_dir[d].append(fr)
    w("## Files")
    w("")
    for d in sorted(by_dir):
        files = sorted(by_dir[d], key=lambda fr: fr["path"])
        dc = empty_counts()
        for fr in files:
            add_counts(dc, file_counts(fr, detail))
        title = d if d else "(project root)"
        if detail == "files":
            w(f"### `{title}` - {ratio(dc['files'])} files")
        else:
            w(f"### `{title}` - files {ratio(dc['files'])}, types {ratio(dc['types'])}, members {ratio(dc['members'])}")
        w("")
        if detail == "files":
            w("| Upstream file | FerroUI file | Status | Notes |")
            w("|---|---|---|---|")
            for fr in files:
                w(f"| `{fr['path'].rsplit('/', 1)[-1]}` | `{fr['expected']}` | {fr['status']} | {esc(fr['note'])} |")
            w("")
            continue
        if detail == "types" and pr["scope"] == "in":
            # In scope without extracted members: the file and each of its types, found or not.
            w("| Upstream file | Rust file | Status | Types | Missing types | Members (total) | Notes |")
            w("|---|---|---|---|---|---:|---|")
            for fr in files:
                fc = file_counts(fr, detail)
                rust = ", ".join(f"`{x}`" for x in fr["rust"]) if fr["rust"] else (f"`{fr['expected']}`" if fr["status"] != "n/a" else "-")
                status = fr["status"]
                if status == "present" and fc["types"][0] + fc["types"][2] != fc["types"][1]:
                    status = "partial"
                missing = ", ".join(code(tr["name"]) for tr in fr["types"] if tr["status"] == "missing") or "-"
                types_cell = f"{fc['types'][0]}/{fc['types'][1]}" + (f" ({fc['types'][2]} waived)" if fc["types"][2] else "")
                w(f"| `{fr['path'].rsplit('/', 1)[-1]}` | {rust} | {status} | {types_cell} | {missing} | {fc['members'][1]} | {esc(fr['note'])} |")
            w("")
            continue
        if detail == "types":
            w("| Upstream file | Types | Members | Status |")
            w("|---|---|---|---|")
            for fr in files:
                names = ", ".join(code(tr["name"]) for tr in fr["types"]) or "-"
                w(f"| `{fr['path'].rsplit('/', 1)[-1]}` | {names} | {sum(tr['memberCount'] for tr in fr['types'])} | not started |")
            w("")
            continue
        w("| Upstream file | Rust file | Status | Types | Members | Notes |")
        w("|---|---|---|---|---|---|")
        partial = []
        for fr in files:
            fc = file_counts(fr, detail)
            rust = ", ".join(f"`{x}`" for x in fr["rust"]) if fr["rust"] else (f"`{fr['expected']}`" if fr["status"] != "n/a" else "-")
            status = fr["status"]
            if status == "present":
                done = fc["types"][0] + fc["types"][2] == fc["types"][1] and fc["members"][0] + fc["members"][2] == fc["members"][1]
                status = "present" if done else "partial"
            elif status == "missing" and fc["types"][0]:
                status = "missing (types found elsewhere)"
            if status in ("partial", "missing (types found elsewhere)"):
                partial.append(fr)
            if fr["status"] == "n/a":
                w(f"| `{fr['path'].rsplit('/', 1)[-1]}` | {rust} | n/a | - | - | {esc(fr['note'])} |")
            else:
                w(f"| `{fr['path'].rsplit('/', 1)[-1]}` | {rust} | {status} | {ratio(fc['types'])} | {ratio(fc['members'])} | {esc(fr['note'])} |")
        w("")
        for fr in partial:
            missing_total = 0
            lines = []
            for tr in fr["types"]:
                miss = [mr for mr in tr["members"] if mr["status"] == "missing"]
                waived = [mr for mr in tr["members"] if mr["status"] == "waived"]
                if tr["status"] == "missing":
                    missing_total += 1 + len(miss)
                    lines.append(f"- {code(tr['name'])} ({tr['kind']}, {tr['accessibility']}): **type missing** ({len(tr['members'])} members)")
                    continue
                if tr["status"] == "waived":
                    lines.append(f"- {code(tr['name'])} ({tr['kind']}): waived - {esc(tr.get('reason', ''))}")
                    continue
                if not miss and not waived:
                    continue
                missing_total += len(miss)
                head = f"- {code(tr['name'])} ({tr['kind']})"
                if tr.get("where"):
                    head += f" in `{tr['where']}`"
                if tr.get("asModule"):
                    head += " (ported as module-level items)"
                if miss:
                    lines.append(head + f": {len(miss)} missing")
                    for mr in miss:
                        lines.append(f"  - {member_label(mr)}")
                else:
                    lines.append(head + ": complete")
                if waived:
                    by_reason = defaultdict(list)
                    for mr in waived:
                        by_reason[mr["how"]].append(mr["m"]["name"])
                    for reason in sorted(by_reason):
                        lines.append(f"  - waived ({esc(reason)}): " + ", ".join(code(n) for n in by_reason[reason]))
            if not lines:
                continue
            w(f"<details><summary><code>{esc(fr['path'].rsplit('/', 1)[-1])}</code> - {missing_total} missing</summary>")
            w("")
            L.extend(lines)
            w("")
            w("</details>")
            w("")

    for label, entries in sorted(pr.get("extra", {}).items()):
        present = sum(1 for e in entries if e["status"] == "present")
        w(f"## Other files: {label} - {present}/{len(entries)}")
        w("")
        w("<details><summary>File list</summary>")
        w("")
        w("| Upstream file | FerroUI file | Status |")
        w("|---|---|---|")
        for e in sorted(entries, key=lambda e: e["path"]):
            w(f"| `{e['path']}` | `{e['expected']}` | {e['status']} |")
        w("")
        w("</details>")
        w("")

    for idl in pr.get("idl", []):
        ip = sum(1 for i in idl["interfaces"] if i["status"] == "present")
        mp = sum(i["present"] for i in idl["interfaces"])
        mt = sum(i["methods"] for i in idl["interfaces"])
        w(f"## IDL contracts: `{idl['file']}` -> `{idl['rust']}`")
        w("")
        w(f"Interfaces {ip}/{len(idl['interfaces'])}, methods {mp}/{mt}, enums {sum(1 for e in idl['enums'] if e['status'] == 'present')}/{len(idl['enums'])}, "
          f"structs {sum(1 for s in idl['structs'] if s['status'] == 'present')}/{len(idl['structs'])}. Names are compared after the `Avn` -> `Frn` rename.")
        w("")
        w("| Interface | FerroUI name | Methods | Status | Missing methods |")
        w("|---|---|---|---|---|")
        for i in sorted(idl["interfaces"], key=lambda i: i["name"]):
            w(f"| `{i['name']}` | `{i['rust']}` | {i['present']}/{i['methods']} | {i['status']} | {', '.join(code(x) for x in i['missing']) if i['status'] == 'present' else ''} |")
        w("")
        w("| Enum | FerroUI name | Members | Status |")
        w("|---|---|---|---|")
        for e in sorted(idl["enums"], key=lambda e: e["name"]):
            w(f"| `{e['name']}` | `{e['rust']}` | {e['present']}/{e['members']} | {e['status']} |")
        w("")
        w("| Struct | FerroUI name | Status |")
        w("|---|---|---|")
        for s in sorted(idl["structs"], key=lambda s: s["name"]):
            w(f"| `{s['name']}` | `{s['rust']}` | {s['status']} |")
        w("")

    if pr.get("rustOnly") or pr.get("rustAux"):
        w("## Rust-only files")
        w("")
        w("Rust sources of this crate that no upstream file maps to. Give each a reason in `docs/porting/data/path-overrides.toml` (`[[rust_only]]`), or map upstream files to it (`[[map]]`).")
        w("")
        if pr.get("rustOnly"):
            w("| Rust file | Reason | Types defined |")
            w("|---|---|---|")
            for ro in pr["rustOnly"]:
                types = ", ".join(code(t) for t in ro["types"][:12]) + (f", ... ({len(ro['types'])} total)" if len(ro["types"]) > 12 else "")
                w(f"| `{ro['path']}` | {esc(ro['reason']) if ro['reason'] is not None else '**unmapped**'} | {types} |")
            w("")
        if pr.get("rustAux"):
            w("Tests, examples and build scripts (not scanned): " + ", ".join(f"`{x}`" for x in pr["rustAux"]) + ".")
            w("")
    return "\n".join(L).rstrip() + "\n"


def render_tracking(index: dict, projects: list, counts: dict, repo: str, workspace_members: list) -> str:
    commit = index["upstreamCommit"]
    L = []
    w = L.append
    w("# FerroUI port tracking")
    w("")
    w("<!-- Generated by scripts/port-status/run.sh. Do not edit: change the code, docs/porting/data/path-overrides.toml or member-waivers.toml and regenerate. -->")
    w("")
    sub = ", ".join(f"`{k}` at `{v}`" for k, v in sorted(index.get("submoduleCommits", {}).items()))
    w(f"Generated from upstream commit `{commit}`" + (f" ({sub})" if sub else "") + ".")
    w("")
    w("The master status of the port: every upstream project, file, type and member (public, protected and internal), "
      "and whether its FerroUI counterpart exists. The mapping rules are those of [PORTING-GUIDE.md](PORTING-GUIDE.md). "
      "What is left to port, in one place: [REMAINING.md](REMAINING.md). The audit of what is declared not ported: [waiver-audit.md](waiver-audit.md).")
    w("")
    render_headline(L, projects, counts)
    w("## How to regenerate")
    w("")
    w("```sh")
    w("scripts/port-status/run.sh                 # upstream checkout expected at ../Avalonia")
    w("UPSTREAM=/path/to/Avalonia scripts/port-status/run.sh")
    w("scripts/port-status/run.sh --force         # re-run the upstream extractor even if its JSON is fresh")
    w("scripts/port-status/run.sh --check         # exit 1 when a generated document is out of date; writes nothing")
    w("```")
    w("")
    w("1. `scripts/api-extract` (.NET, Roslyn syntax trees) reads the upstream sources listed in `scripts/api-extract/projects.json` "
      "and writes `docs/porting/data/upstream-api.json`. It runs only when that file is missing, when the project list or the extractor changed, "
      "or when `TRACKED_COMMIT` names another commit. It reads the tracked commit (the one named above), not the `HEAD` of the checkout: "
      "`run.sh` exports the tree of that commit, and of its submodules at the commits the tree names, into a temporary directory and extracts from there.")
    w("2. `scripts/port-status/port_status.py` (Python, standard library) reads that JSON, scans the Rust tree read-only and writes this file, "
      "`REMAINING.md`, `docs/porting/tracking/*.md` and `docs/porting/data/port-status.json`. `REMAINING.md` also reads the totals of the test reports "
      "(`docs/porting/data/test-gaps-*.txt`, written by `scripts/port-status/test_gaps.py --all`) and the list of samples (`docs/porting/data/samples.toml`).")
    w("")
    w("Inputs you edit by hand:")
    w("")
    w("- `docs/porting/data/path-overrides.toml` - files that do not follow the default path rule (merged, renamed, replaced, not applicable) and Rust-only files.")
    w("- `docs/porting/data/member-waivers.toml` - types and members that are intentionally not ported, with the reason (`[[waive]]`), "
      "and members the port has under a name the rules cannot derive (`[[alias]]`, checked at every run).")
    w("- `docs/porting/data/samples.toml` - the upstream samples and where each is ported.")
    w("- `scripts/api-extract/projects.json` - the project list, target crates, phases and priorities.")
    w("")
    w("`scripts/port-status/port_status.py --explain <Project> <File.cs>` prints how every member of one file was matched.")
    w("")
    w("## Legend")
    w("")
    w("| Level | Status | Meaning |")
    w("|---|---|---|")
    w("| File | present | The mapped Rust file exists (default rule or an override). Shown as *partial* in project pages while types or members are missing. |")
    w("| File | missing | No Rust file at the mapped path. When its types were found elsewhere the page says so; add an override. |")
    w("| File | n/a | Not applicable in Rust (override with a reason), or the file declares no non-private type. Excluded from the totals. |")
    w("| Type | present | A `struct`, `enum`, `trait`, `type` alias or `ferro_class!` with the mapped name exists in the mapped file (or elsewhere in the crate). |")
    w("| Type | waived | Listed in `member-waivers.toml`, or the file is replaced by a different design. Excluded from the percentage. |")
    w("| Member | present | An item with the mapped name exists on the Rust type: property `Foo` -> `foo()` (+ `set_foo()` when settable) or a field `foo`; "
      "`FooProperty` -> `foo_property()`; `FooEvent` -> `foo_event()`; method `DoIt` -> `do_it()` (overloads: `do_it_*`); constructors -> `new`/`new_*`/`from_*`/`From`/`Default`; "
      "operators -> `std::ops` traits; `ToString` -> `Display`; `Equals` -> `PartialEq`; `GetHashCode` -> `Hash`; `Parse` -> `FromStr`; enum members -> variants or flag constants. |")
    w("| Member | missing | No such item. For overloads the tool counts matches: it knows how many are missing, not which. |")
    w("| Member | waived | Listed in `member-waivers.toml` with a reason. Excluded from the percentage. |")
    w("")
    w("Percentages are `present / (total - waived)`. Member matching is by name with regular expressions, not by signature or behaviour: "
      "*present* means \"an item with the right name exists\", not \"reviewed and equivalent\". Private members are not tracked.")
    w("")

    in_scope = [p for p in projects if p["scope"] == "in"]
    out_scope = [p for p in projects if p["scope"] != "in"]
    total = empty_counts()
    for p in in_scope:
        if p["detail"] == "full":
            add_counts(total, counts[p["name"]])
    other_files = [0, 0]
    for p in in_scope:
        if p["detail"] == "files":
            other_files[0] += counts[p["name"]]["files"][0]
            other_files[1] += counts[p["name"]]["files"][1]
        for entries in p.get("extra", {}).values():
            other_files[0] += sum(1 for e in entries if e["status"] == "present")
            other_files[1] += len(entries)
    out_total = empty_counts()
    for p in out_scope:
        add_counts(out_total, counts[p["name"]])

    w("## Totals (projects in scope with member detail)")
    w("")
    w("| | Ported | Total | Waived | % |")
    w("|---|---:|---:|---:|---:|")
    for label, key in (("C# files", "files"), ("Types", "types"), ("Members", "members")):
        c = total[key]
        w(f"| {label} | {c[0]} | {c[1]} | {c[2]} | {pct(*c)} |")
    for label, key in (("Contracts (interfaces)", "interfaces"), ("Property registrations", "properties"), ("Routed events", "events")):
        c = total[key]
        w(f"| {label} | {c[0]} | {c[1]} | - | {pct(c[0], c[1])} |")
    w(f"| Other files (native sources, XAML, TypeScript, fonts) | {other_files[0]} | {other_files[1]} | - | {pct(*other_files)} |")
    w("")
    w(f"{total['na_files']} upstream files are not applicable and not counted. "
      f"Out of the current scope (below): {out_total['files'][1]} files, {out_total['types'][1]} types, {out_total['members'][1]} members.")
    w("")

    def project_rows(ps):
        for p in sorted(ps, key=lambda p: p["path"]):
            c = counts[p["name"]]
            link = f"[{p['name']}](tracking/{page_name(p['name'])})"
            crate = code(p["crate"]) if not p["crate"].startswith("(") else p["crate"]
            if p["detail"] == "files":
                yield f"| {link} | `{p['path']}` | `{p['rust']}` | {crate} | {ratio(c['files'])} | - | - | {pct(*c['files'])} | {p['phase']} | {p['priority']} |"
            else:
                mp = pct(*c["members"]) if p["detail"] == "full" and p["scope"] == "in" else "0.0%"
                yield (f"| {link} | `{p['path']}` | `{p['rust']}` | {crate} | {ratio(c['files'])} | {ratio(c['types'])} | {ratio(c['members'])} | {mp} | {p['phase']} | {p['priority']} |")

    w("## Projects")
    w("")
    w("The % column is member coverage (file coverage for plain file lists).")
    w("")
    w("| Project | Upstream path | FerroUI path | Crate | Files | Types | Members | % | Phase | Priority |")
    w("|---|---|---|---|---:|---:|---:|---:|---|---|")
    L.extend(project_rows(in_scope))
    w("")
    extras = [(p, label, entries) for p in sorted(in_scope, key=lambda p: p["path"]) for label, entries in sorted(p.get("extra", {}).items())]
    if extras:
        w("Non-C# files that belong to these projects:")
        w("")
        w("| Project | Kind | Ported | Total |")
        w("|---|---|---:|---:|")
        for p, label, entries in extras:
            w(f"| [{p['name']}](tracking/{page_name(p['name'])}) | {label} | {sum(1 for e in entries if e['status'] == 'present')} | {len(entries)} |")
        w("")
    idls = [(p, idl) for p in in_scope for idl in p.get("idl", [])]
    for p, idl in idls:
        ip = sum(1 for i in idl["interfaces"] if i["status"] == "present")
        mp = sum(i["present"] for i in idl["interfaces"])
        mt = sum(i["methods"] for i in idl["interfaces"])
        w(f"Native interop contract `{p['path']}/{idl['file']}` -> `{p['rust']}/{idl['rust']}`: interfaces {ip}/{len(idl['interfaces'])}, methods {mp}/{mt}, "
          f"enums {sum(1 for e in idl['enums'] if e['status'] == 'present')}/{len(idl['enums'])}, structs {sum(1 for s in idl['structs'] if s['status'] == 'present')}/{len(idl['structs'])} "
          f"(details in [{p['name']}](tracking/{page_name(p['name'])})).")
        w("")

    w("### Not started / out of current scope")
    w("")
    w("Tracked so that the size of the remaining work is known: files, types and member totals, and the members themselves for the projects extracted with full detail.")
    w("")
    w("| Project | Upstream path | FerroUI path | Crate | Files | Types | Members | % | Phase | Priority |")
    w("|---|---|---|---|---:|---:|---:|---:|---|---|")
    L.extend(project_rows(out_scope))
    w("")

    # structure mapping
    w("## Project structure")
    w("")
    w("The cargo workspace mirrors the upstream solution layout: same directories, `Avalonia.X` -> `FerroUI.X`, one crate per upstream project "
      "(`[lib] path = \"lib.rs\"`, sources directly in the crate directory, directories and files in `snake_case`).")
    w("")
    tracked = {p["path"]: p for p in projects}
    layout = index.get("solutionLayout", {})
    members = set(workspace_members)

    def exists(path: str) -> bool:
        return os.path.isdir(os.path.join(repo, path))

    def state(path: str) -> str:
        if path in members:
            return "workspace member"
        if os.path.exists(os.path.join(repo, path, "Cargo.toml")):
            return "crate (not in workspace)"
        if exists(path):
            return "directory exists"
        return "not created"

    notes = {
        "src": "Libraries.",
        "native": "Objective-C++ sources of the macOS backend, re-imported with `scripts/sync-native.sh`.",
        "external": "Third-party code ported in place (same relative paths as upstream).",
        "samples": "Sample applications. Same directory names; binary crates.",
        "tests": "Upstream unit tests are ported next to the code (`#[cfg(test)] mod tests` or `<file>_tests.rs`, porting guide rule 1); "
                 "cross-crate, render and integration test projects get a crate under `tests/`.",
    }
    for top in ("src", "native", "external", "samples", "tests"):
        entries = layout.get(top, [])
        if not entries:
            continue
        w(f"### `{top}/`")
        w("")
        w(notes[top])
        w("")
        w("| Upstream project | C# files | FerroUI path | State | Tracking |")
        w("|---|---:|---|---|---|")
        for e in entries:
            path = e["path"]
            tp = tracked.get(path)
            if tp is not None:
                rust = tp["rust"]
                tracking = f"[{tp['name']}](tracking/{page_name(tp['name'])})" + ("" if tp["scope"] == "in" else " (out of scope)")
            else:
                rust = map_project_dir(path)
                tracking = "not tracked"
            if top == "tests" and not exists(rust):
                st = "not created"
            else:
                st = state(rust)
            w(f"| `{path}` | {e['csFiles']} | `{rust}` | {st} | {tracking} |")
        w("")

    # Rust-only crates
    upstream_rust_dirs = {p["rust"] for p in projects} | {map_project_dir(e["path"]) for entries in layout.values() for e in entries}
    rust_only_crates = sorted(m for m in workspace_members if m not in upstream_rust_dirs)
    w("### Cargo workspace members")
    w("")
    w("| Workspace member | Upstream project |")
    w("|---|---|")
    by_rust = {p["rust"]: p for p in projects}
    layout_by_rust = {map_project_dir(e["path"]): e["path"] for entries in layout.values() for e in entries}
    for m in sorted(workspace_members):
        if m in by_rust:
            w(f"| `{m}` | `{by_rust[m]['path']}` |")
        elif m in layout_by_rust:
            w(f"| `{m}` | `{layout_by_rust[m]}` (not tracked) |")
        else:
            w(f"| `{m}` | none (FerroUI only) |")
    w("")
    if rust_only_crates:
        w("FerroUI-only crates have no upstream source directory (for example the MicroCom code generator, which upstream consumes as a NuGet package).")
        w("")

    w("## Rust-only files")
    w("")
    ro_total = sum(len(p.get("rustOnly", [])) for p in projects)
    ro_unmapped = sum(1 for p in projects for ro in p.get("rustOnly", []) if ro["reason"] is None)
    w(f"{ro_total} Rust source files have no upstream counterpart ({ro_unmapped} without a recorded reason). They are listed at the end of each project page.")
    w("")
    w("| Project | Rust file | Reason |")
    w("|---|---|---|")
    for p in sorted(projects, key=lambda p: p["path"]):
        for ro in p.get("rustOnly", []):
            w(f"| [{p['name']}](tracking/{page_name(p['name'])}) | `{p['rust']}/{ro['path']}` | {esc(ro['reason']) if ro['reason'] is not None else '**unmapped**'} |")
    w("")
    return "\n".join(L).rstrip() + "\n"


# --------------------------------------------------------------------------
# The two headline numbers and REMAINING.md
# --------------------------------------------------------------------------

def na_size(pr: dict) -> list:
    """[files, types, members] of the files of a project that are marked not applicable."""
    size = [0, 0, 0]
    for fr in pr["files"]:
        if fr["status"] != "n/a":
            continue
        size[0] += 1
        size[1] += len(fr["types"])
        size[2] += sum(len(tr["members"]) or tr.get("memberCount", 0) for tr in fr["types"])
    return size


def overall_numbers(projects: list, counts: dict) -> dict:
    """The totals behind the two headline numbers.

    `scope`: the projects in scope whose members were extracted: present / (total - waived).
    `all`: every C# project of the extraction, in scope or not: present / every member, where every member
    includes the waived ones, the ones of files marked not applicable and the ones of projects out of scope."""
    scope = empty_counts()
    present = 0
    everything = [0, 0, 0]          # files, types, members
    na = [0, 0, 0]
    out = [0, 0, 0]
    scope_projects = []
    for pr in projects:
        if pr["detail"] == "files":
            continue
        c = counts[pr["name"]]
        n = na_size(pr)
        for i, key in enumerate(("files", "types", "members")):
            everything[i] += c[key][1] + n[i]
            na[i] += n[i]
            if pr["scope"] != "in":
                out[i] += c[key][1] + n[i]
        present += c["members"][0]
        if pr["scope"] == "in" and pr["detail"] == "full":
            add_counts(scope, c)
            scope_projects.append(pr["name"])
    return {"scope": scope, "scopeProjects": scope_projects, "present": present, "all": everything, "na": na, "out": out}


def share(present: int, total: int) -> str:
    return f"{100.0 * present / total:.1f}%" if total else "-"


def render_headline(L: list, projects: list, counts: dict) -> None:
    w = L.append
    o = overall_numbers(projects, counts)
    m = o["scope"]["members"]
    missing = m[1] - m[0] - m[2]
    w("## Headline")
    w("")
    w("Two numbers, because one hides what the other shows.")
    w("")
    w(f"1. **Of what is in scope: {pct(*m)}.** The {len(o['scopeProjects'])} upstream projects that are in scope have {m[1]} members "
      f"(public, protected and internal) in files that apply to the port. {m[0]} have a counterpart, {missing} are missing and {m[2]} are waived: "
      f"declared not ported, each with a reason ([waiver-audit.md](waiver-audit.md)). The percentage is `present / (total - waived)`. "
      f"It says nothing about projects that are out of scope, and it counts a waived member as if it did not exist.")
    w(f"2. **Of everything upstream has: {share(o['present'], o['all'][2])}.** Every C# source project of the extraction, in scope or not, "
      f"has {o['all'][2]} members; the port has a counterpart for {o['present']}. The total includes the {m[2]} waived members, "
      f"the {o['na'][2]} members of files marked not applicable and the {o['out'][2]} members of projects that are out of scope or not started. "
      f"The rest of that distance is what [REMAINING.md](REMAINING.md) lists; part of it is never ported by design (the waived and not applicable members), "
      f"so this number does not reach 100.")
    w("")
    w("Both numbers match names, not behaviour (Legend, below). Projects the extraction does not read (analyzers, generators of upstream's own build, "
      "the D-Bus library) are listed in REMAINING.md with their size in files.")
    w("")


TEST_PROJECTS = [
    # short name of docs/porting/data/test-gaps-<short>.txt -> upstream test project (scripts/port-status/test_gaps.py, PROJECTS)
    ("base", "tests/Avalonia.Base.UnitTests"), ("controls", "tests/Avalonia.Controls.UnitTests"), ("markup", "tests/Avalonia.Markup.UnitTests"),
    ("markup-xaml", "tests/Avalonia.Markup.Xaml.UnitTests"), ("skia", "tests/Avalonia.Skia.UnitTests"), ("headless", "tests/Avalonia.Headless.UnitTests"),
    ("themes", "tests/Avalonia.Themes.UnitTests"), ("build-tasks", "tests/Avalonia.Build.Tasks.UnitTest"),
    ("designer-support", "tests/Avalonia.DesignerSupport.Tests"), ("render", "tests/Avalonia.RenderTests"),
]


def load_test_totals(data_dir: str) -> list:
    """The totals at the top of each report of test_gaps.py: [(project path, short, totals or None)]."""
    out = []
    for short, project in TEST_PROJECTS:
        path = os.path.join(data_dir, f"test-gaps-{short}.txt")
        totals = None
        if os.path.exists(path):
            with open(path, encoding="utf-8") as f:
                head = f.read(2000)
            t = re.search(r"^Tests: (\d+) in (\d+) files", head, re.M)
            p = re.search(r"^Present: (\d+)", head, re.M)
            wv = re.search(r"^Waived: (\d+)", head, re.M)
            ms = re.search(r"^Missing: (\d+) in (\d+) files", head, re.M)
            c = re.search(r"at ([0-9a-f]{40})", head)
            if t and p and wv and ms:
                totals = {"tests": int(t.group(1)), "files": int(t.group(2)), "present": int(p.group(1)), "waived": int(wv.group(1)),
                          "missing": int(ms.group(1)), "missingFiles": int(ms.group(2)), "commit": c.group(1) if c else ""}
        out.append((project, short, totals))
    return out


def render_remaining(index: dict, projects: list, counts: dict, repo: str, data_dir: str) -> str:
    commit = index["upstreamCommit"]
    L = []
    w = L.append
    o = overall_numbers(projects, counts)
    m = o["scope"]["members"]
    w("# What is left to port")
    w("")
    w("<!-- Generated by scripts/port-status/run.sh (port_status.py). Do not edit: change the code or the data files and regenerate. -->")
    w("")
    w(f"Generated from upstream commit `{commit}`. The single place to look for what the port does not have yet. "
      "It is written from the same data as [TRACKING.md](TRACKING.md) and the pages under `tracking/`, which have the detail of every file; "
      "nothing here is written by hand. What is declared not ported, and why, is audited in [waiver-audit.md](waiver-audit.md).")
    w("")
    w("## The two numbers")
    w("")
    w("| | Present | Total | Missing | Waived | Not applicable | Out of scope | Share |")
    w("|---|---:|---:|---:|---:|---:|---:|---:|")
    w(f"| Members of the projects in scope | {m[0]} | {m[1]} | {m[1] - m[0] - m[2]} | {m[2]} | - | - | {pct(*m)} of total less waived |")
    in_all = o["all"][2] - o["na"][2] - o["out"][2]
    w(f"| Members of every upstream source project of the extraction | {o['present']} | {o['all'][2]} | {in_all - m[0] - m[2]} | {m[2]} | {o['na'][2]} | {o['out'][2]} | "
      f"{share(o['present'], o['all'][2])} of total |")
    w("")
    w("The first row is the headline of the tracking: it leaves out the waived members and everything out of scope. "
      "The second row leaves out nothing: every member of every C# project the extraction reads, whether or not the port will ever have it. "
      "A member is *present* when an item of the mapped name exists; names are matched, not behaviour.")
    w("")

    # ---- per project
    w("## Upstream source projects")
    w("")
    w("Files, types and members as present / missing / waived. *n/a* counts the files marked not applicable (and their members), which the totals of the tracking leave out. "
      "For a project that is out of scope the three columns give its size.")
    w("")
    w("| Project | Scope | Files | Types | Members | n/a files (members) | Member share of all |")
    w("|---|---|---:|---:|---:|---:|---:|")

    def triple(c):
        return f"{c[0]} / {c[1] - c[0] - c[2]} / {c[2]}"

    cs_projects = [p for p in sorted(projects, key=lambda p: (p["scope"] != "in", p["path"])) if p["detail"] != "files"]
    for pr in cs_projects:
        c = counts[pr["name"]]
        n = na_size(pr)
        link = f"[{pr['name']}](tracking/{page_name(pr['name'])})"
        all_members = c["members"][1] + n[2]
        if pr["scope"] != "in":
            started = "started" if pr["rustExists"] else "not started"
            w(f"| {link} | out ({started}) | {c['files'][1]} files | {c['types'][1]} types | {c['members'][1]} members | "
              f"{n[0]} ({n[2]}) | {share(c['members'][0], all_members)} |")
            continue
        scope = "in"
        if c["files"][1] == 0:
            scope = "in: every file not applicable"
        elif pr["detail"] != "full":
            scope = "in (members not extracted)"
        w(f"| {link} | {scope} | {triple(c['files'])} | {triple(c['types'])} | {triple(c['members']) if pr['detail'] == 'full' else '0 / ' + str(c['members'][1]) + ' / 0'} | "
          f"{n[0]} ({n[2]}) | {share(c['members'][0], all_members)} |")
    w("")
    other = [p for p in sorted(projects, key=lambda p: p["path"]) if p["detail"] == "files" or p.get("extra")]
    if other:
        w("Files that are not C#:")
        w("")
        w("| Project | Kind | Present | Missing |")
        w("|---|---|---:|---:|")
        for pr in other:
            link = f"[{pr['name']}](tracking/{page_name(pr['name'])})"
            if pr["detail"] == "files":
                c = counts[pr["name"]]["files"]
                w(f"| {link} | native sources | {c[0]} | {c[1] - c[0]} |")
            for label, entries in sorted(pr.get("extra", {}).items()):
                present = sum(1 for e in entries if e["status"] == "present")
                w(f"| {link} | {label} | {present} | {len(entries) - present} |")
        w("")
    for pr in projects:
        for idl in pr.get("idl", []):
            missing_methods = sum(i["methods"] - i["present"] for i in idl["interfaces"])
            w(f"Native interop contract `{idl['file']}` of {pr['name']}: {sum(1 for i in idl['interfaces'] if i['status'] != 'present')} interfaces and "
              f"{missing_methods} methods missing of {len(idl['interfaces'])} and {sum(i['methods'] for i in idl['interfaces'])}.")
            w("")

    tracked = {p["path"] for p in projects}
    layout = index.get("solutionLayout", {})
    untracked = [e for top in ("src", "native", "external") for e in layout.get(top, []) if e["path"] not in tracked]
    if untracked:
        w("### Upstream projects the extraction does not read")
        w("")
        w("Directories of `src`, `native` and `external` that hold a project and are not in `scripts/api-extract/projects.json`: "
          "their types and members are in neither number above. Size in C# files.")
        w("")
        w("| Upstream project | C# files | FerroUI directory |")
        w("|---|---:|---|")
        for e in untracked:
            rust = map_project_dir(e["path"])
            w(f"| `{e['path']}` | {e['csFiles']} | {'`' + rust + '`' if os.path.isdir(os.path.join(repo, rust)) else 'none'} |")
        w("")

    # ---- tests
    w("## Upstream test projects")
    w("")
    w("Counted by `scripts/port-status/test_gaps.py --all` (its rules are at its top), read here from the totals of its reports in `docs/porting/data/`. "
      "A test is present when the port has a test function of its name.")
    w("")
    w("| Upstream test project | Tests | Present | Missing | Waived | Report |")
    w("|---|---:|---:|---:|---:|---|")
    counted = set()
    sums = [0, 0, 0, 0]
    stale_reports = []
    for project, short, t in load_test_totals(data_dir):
        counted.add(project)
        if t is None:
            w(f"| `{project}` | - | - | - | - | no report: run `test_gaps.py --all` |")
            continue
        for i, k in enumerate(("tests", "present", "missing", "waived")):
            sums[i] += t[k]
        if t["commit"] and t["commit"] != commit:
            stale_reports.append(short)
        w(f"| `{project}` | {t['tests']} | {t['present']} | {t['missing']} | {t['waived']} | `data/test-gaps-{short}.txt` |")
    w(f"| Total | {sums[0]} | {sums[1]} | {sums[2]} | {sums[3]} | |")
    w("")
    if stale_reports:
        w("Reports written for another upstream commit: " + ", ".join(f"`{s}`" for s in stale_reports) + ".")
        w("")
    not_counted = [e for e in layout.get("tests", []) if e["path"] not in counted]
    if not_counted:
        w("Test projects that are not counted (no report; `CONTINUATION.md` has the reasons for those that were looked at):")
        w("")
        w("| Upstream test project | C# files | FerroUI directory |")
        w("|---|---:|---|")
        for e in not_counted:
            rust = map_project_dir(e["path"])
            w(f"| `{e['path']}` | {e['csFiles']} | {'`' + rust + '`' if os.path.isdir(os.path.join(repo, rust)) else 'none'} |")
        w("")

    # ---- samples
    w("## Upstream samples")
    w("")
    w("From `docs/porting/data/samples.toml`; a sample counts as ported when the directory the file names exists.")
    w("")
    w("| Upstream sample | C# files | State | FerroUI directory | Note |")
    w("|---|---:|---|---|---|")
    listed = {s["upstream"]: s for s in load_toml(os.path.join(data_dir, "samples.toml")).get("sample", [])}
    ported = 0
    samples = layout.get("samples", [])
    for e in samples:
        s = listed.get(e["path"], {})
        rust = s.get("rust")
        if rust and os.path.isdir(os.path.join(repo, rust)):
            state = "ported"
            ported += 1
        elif rust:
            state = "not ported (the directory named does not exist)"
        else:
            state = s.get("state", "not ported")
        w(f"| `{e['path']}` | {e['csFiles']} | {state} | {'`' + rust + '`' if rust else '-'} | {esc(s.get('note', '' if s else 'not listed in samples.toml'))} |")
    w("")
    w(f"{ported} of {len(samples)} samples are ported.")
    w("")

    # ---- missing lists
    w("## Missing, by project")
    w("")
    w("Files without a Rust file, types without a Rust type, and members without a Rust item, for the projects in scope. "
      "Names only: the page of each project has the signatures and, for files that exist in part, the members by file.")
    w("")
    any_missing = False
    for pr in cs_projects:
        if pr["scope"] != "in" or pr["detail"] not in ("full", "types"):
            continue
        files = [fr["path"] for fr in pr["files"] if fr["status"] == "missing"]
        types = []
        members = defaultdict(list)
        in_missing_types = 0
        for fr in pr["files"]:
            for tr in fr["types"]:
                if tr["status"] == "missing":
                    types.append(tr["name"])
                    in_missing_types += len(tr["members"]) or tr.get("memberCount", 0)
                elif tr["status"] == "present":
                    for mr in tr["members"]:
                        if mr["status"] == "missing":
                            members[tr["name"]].append(mr["m"]["name"])
        loose = sum(len(v) for v in members.values())
        if not files and not types and not loose:
            continue
        any_missing = True
        w(f"### [{pr['name']}](tracking/{page_name(pr['name'])})")
        w("")
        w(f"{len(files)} files, {len(types)} types and {in_missing_types + loose} members missing "
          f"({in_missing_types} members of the missing types, {loose} members of types that exist).")
        w("")
        if files:
            w(f"- **Files ({len(files)}):** " + ", ".join(f"`{f}`" for f in sorted(files)))
        if types:
            names = sorted(set(types))
            shown = names[:120]
            w(f"- **Types ({len(types)}):** " + ", ".join(code(n) for n in shown) + (f", and {len(names) - len(shown)} more" if len(names) > len(shown) else ""))
        if loose:
            if loose <= 60:
                items = [f"{t}.{n}" for t in sorted(members) for n in members[t]]
                w(f"- **Members of types that exist ({loose}):** " + ", ".join(code(i) for i in items))
            else:
                ranked = sorted(members.items(), key=lambda kv: (-len(kv[1]), kv[0]))
                shown = ranked[:40]
                w(f"- **Members of types that exist ({loose}), by type:** " + ", ".join(f"{code(t)} {len(v)}" for t, v in shown)
                  + (f", and {sum(len(v) for _, v in ranked[40:])} in {len(ranked) - 40} more types" if len(ranked) > 40 else ""))
        w("")
    if not any_missing:
        w("Nothing is missing in the projects in scope.")
        w("")
    return "\n".join(L).rstrip() + "\n"


# --------------------------------------------------------------------------
# Main
# --------------------------------------------------------------------------

def workspace_members(repo: str) -> list:
    path = os.path.join(repo, "Cargo.toml")
    if not os.path.exists(path):
        return []
    data = load_toml(path)
    return list(data.get("workspace", {}).get("members", []))


def write_if_changed(path: str, text: str) -> bool:
    try:
        with open(path, encoding="utf-8") as f:
            if f.read() == text:
                return False
    except OSError:
        pass
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)
    return True


def main() -> int:
    here = os.path.dirname(os.path.abspath(__file__))
    repo_default = os.path.dirname(os.path.dirname(here))
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--repo", default=repo_default)
    ap.add_argument("--explain", nargs=2, metavar=("PROJECT", "FILE"), help="print the member matching of one upstream file and exit")
    ap.add_argument("--check", action="store_true", help="exit 1 if the generated documents are out of date (writes nothing)")
    args = ap.parse_args()
    repo = os.path.abspath(args.repo)
    data_dir = os.path.join(repo, "docs", "porting", "data")
    index = load_upstream(data_dir)
    apply_project_list(index, repo)
    overrides = Overrides(load_toml(os.path.join(data_dir, "path-overrides.toml")))
    waivers = Waivers(load_toml(os.path.join(data_dir, "member-waivers.toml")))

    roots = [p.get("rust") or map_project_dir(p["path"]) for p in index["projects"]]
    projects = []
    for p in index["projects"]:
        projects.append(scan_project(p, repo, overrides, waivers, roots))

    if args.explain:
        pname, fname = args.explain
        for pr in projects:
            if pr["name"] != pname:
                continue
            for fr in pr["files"]:
                if fr["path"] == fname:
                    print(f"{fr['path']} -> {fr['rust'] or fr['expected']} [{fr['status']}] {fr['note']}")
                    for tr in fr["types"]:
                        print(f"  {tr['kind']} {tr['name']}: {tr['status']}" + (f" (in {tr['where']})" if tr.get("where") else ""))
                        for mr in tr["members"]:
                            print(f"    {mr['status']:8} {mr['m'].get('signature', mr['m']['name'])}  <- {mr['how']}")
                    return 0
        print("not found", file=sys.stderr)
        return 1

    counts = {}
    for pr in projects:
        c = empty_counts()
        for fr in pr["files"]:
            add_counts(c, file_counts(fr, pr["detail"]))
        counts[pr["name"]] = c

    commit = index["upstreamCommit"]
    outputs = {}
    for pr in projects:
        outputs[os.path.join(repo, "docs", "porting", "tracking", page_name(pr["name"]))] = render_project(pr, counts[pr["name"]], commit)
    outputs[os.path.join(repo, "docs", "porting", "TRACKING.md")] = render_tracking(index, projects, counts, repo, workspace_members(repo))
    outputs[os.path.join(repo, "docs", "porting", "REMAINING.md")] = render_remaining(index, projects, counts, repo, data_dir)

    numbers = overall_numbers(projects, counts)
    summary = {
        "upstreamCommit": commit,
        "inScope": {"files": numbers["scope"]["files"], "types": numbers["scope"]["types"], "members": numbers["scope"]["members"]},
        "allProjects": {"membersPresent": numbers["present"], "members": numbers["all"][2], "membersNotApplicable": numbers["na"][2],
                        "membersOutOfScope": numbers["out"][2]},
        "projects": [],
        "unusedPathOverrides": [overrides.maps[i].get("upstream") for i in range(len(overrides.maps)) if i not in overrides.used],
        "unusedWaivers": [w for i, w in enumerate(waivers.entries) if not waivers.hits[i]],
    }
    for pr in sorted(projects, key=lambda p: p["path"]):
        c = counts[pr["name"]]
        summary["projects"].append({
            "name": pr["name"], "path": pr["path"], "rust": pr["rust"], "scope": pr["scope"], "detail": pr["detail"],
            "files": c["files"], "types": c["types"], "members": c["members"], "interfaces": c["interfaces"],
            "propertyRegistrations": c["properties"], "routedEvents": c["events"], "notApplicableFiles": c["na_files"],
            "matchKinds": dict(sorted(pr["how"].items())),
            "presentFiles": {fr["path"]: fr["rust"] for fr in sorted(pr["files"], key=lambda f: f["path"]) if fr["status"] == "present"},
            "rustOnly": [ro["path"] for ro in pr.get("rustOnly", [])],
        })
    outputs[os.path.join(data_dir, "port-status.json")] = json.dumps(summary, indent=1, ensure_ascii=False) + "\n"

    if args.check:
        stale = []
        for path, text in outputs.items():
            try:
                with open(path, encoding="utf-8") as f:
                    if f.read() != text:
                        stale.append(path)
            except OSError:
                stale.append(path)
        for s in stale:
            print("out of date:", os.path.relpath(s, repo))
        return 1 if stale else 0

    # remove pages of projects that no longer exist
    tracking_dir = os.path.join(repo, "docs", "porting", "tracking")
    if os.path.isdir(tracking_dir):
        for fn in os.listdir(tracking_dir):
            p = os.path.join(tracking_dir, fn)
            if fn.endswith(".md") and p not in outputs:
                os.remove(p)
    changed = sum(write_if_changed(p, t) for p, t in outputs.items())

    total = empty_counts()
    for pr in projects:
        if pr["scope"] == "in" and pr["detail"] == "full":
            add_counts(total, counts[pr["name"]])
    print(f"port-status: files {ratio(total['files'])}, types {ratio(total['types'])}, members {ratio(total['members'])} "
          f"({pct(*total['members'])}); {changed} of {len(outputs)} documents updated")
    print(f"port-status: of every upstream source project, in scope or not: members {numbers['present']}/{numbers['all'][2]} "
          f"({share(numbers['present'], numbers['all'][2])})")
    for w_ in summary["unusedPathOverrides"]:
        print("warning: path override matched nothing:", w_, file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
