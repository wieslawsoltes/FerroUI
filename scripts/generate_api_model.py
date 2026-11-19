#!/usr/bin/env python3
"""Writes the language-neutral API model of the bindings of FerroUI.

Stage one of the generators of the bindings (bindings/README.md): the model is
built once from the metadata of the framework and every language backend reads
only the model (`bindings/<language>/generate.py`). This script writes

    bindings/model/ferroui-api.json
        classes (base, documentation, whether they can be created, where they
        hold children), properties (value kind and Rust type, default,
        read-only, attached, documentation, the classes that declare them),
        enumerations (members, values, documentation), routed events (argument
        type, the property a frame reads as the value of the event, the
        dispatch styles), the functions of the C ABI (read from the generated
        header), the byte layout of a frame, and the stable numeric IDs of
        types, properties and events with their fingerprint.

    src/FerroUI.CApi/api_ids.rs
        the same IDs as tables of names compiled into ferroui-capi: the library
        interns them in this order when it starts, so the ID of a name is the
        constant of the model and a binding calls the ABI with constants instead
        of looking names up. `ferro_api_fingerprint()` returns the fingerprint,
        which a binding compares with its own at start.

Names are stored as words (`FontSize` -> ["font", "size"]): each backend
applies the casing of its language. The metadata walk is
`scripts/ferroui_metadata.py` (shared with the view builders).

How to run (from the repository root; Python 3, standard library only):

    python3 scripts/generate_api_model.py            # writes both files
    python3 scripts/generate_api_model.py --check    # exit 1 if one is stale
"""

import argparse
import json
import os
import re
import sys

import ferroui_metadata as meta

ROOT = meta.ROOT
MODEL = os.path.join(ROOT, "bindings", "model", "ferroui-api.json")
ID_TABLE = os.path.join(ROOT, "src", "FerroUI.CApi", "api_ids.rs")
HEADER = os.path.join(ROOT, "src", "FerroUI.CApi", "include", "ferroui.h")

# The version of the format of the model file; a change that breaks a backend raises it.
MODEL_FORMAT = 1

# The property a frame reads when an event fires, given to the message as its value
# (`ferro_frame_on(event, tag, value_property)`): the text of a text box, the value of a range.
EVENT_VALUES = {
    "TextChanged": "Text",
    "TextChanging": "Text",
    "ValueChanged": "Value",
    "IsCheckedChanged": "IsChecked",
    "SelectionChanged": "SelectedIndex",
}

NUMBER_TYPES = {"f64", "f32"}
INTEGER_TYPES = {"i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize"}
STRING_TYPES = {"String", "Rc<str>", "Arc<str>", "SharedString"}

# Value types that the ABI converts from text as markup does (`ferro_set_str` of "4,2" for a
# thickness, "Red" for a brush): structs and the interfaces of brushes, plain names only.
PARSED_INTERFACES = {"IBrush", "ITransform", "IImage", "IEffect"}


def words(name):
    """`FontSize` -> ["font", "size"], `IsHitTestVisible` -> ["is", "hit", "test", "visible"],
    `Grid.Row` -> ["grid", "row"], `UTF8Text` -> ["utf8", "text"]."""
    parts = []
    for segment in name.split("."):
        parts += [part for part in meta.snake(segment).split("_") if part]
    return parts


def pascal(snake_name):
    return "".join(part[:1].upper() + part[1:] for part in snake_name.split("_") if part)


def unwrap_option(type_text):
    found = re.fullmatch(r"Option<(.+)>", type_text.strip())
    return (found.group(1).strip(), True) if found else (type_text.strip(), False)


def value_of(model, type_text):
    """The value kind of a property type: what a backend writes it with."""
    inner, nullable = unwrap_option(type_text)
    value = {"rust": " ".join(type_text.split()), "nullable": nullable}
    if inner == "bool":
        value["kind"] = "bool"
    elif inner in NUMBER_TYPES:
        value["kind"] = "number"
    elif inner in INTEGER_TYPES:
        value["kind"] = "integer"
    elif inner in STRING_TYPES:
        value["kind"] = "string"
    elif inner in model.enums:
        value["kind"] = "enum"
        value["enum"] = inner
    elif inner == "BoxedValue":
        # A content: text (shown by a text block) or an object.
        value["kind"] = "content"
    elif re.fullmatch(r"(?:Element)?Ref<\s*([A-Z]\w*)\s*>", inner):
        value["kind"] = "object"
        value["class"] = re.fullmatch(r"(?:Element)?Ref<\s*([A-Z]\w*)\s*>", inner).group(1)
    elif re.fullmatch(r"[A-Z]\w*", inner) and inner not in ("Vec", "Box"):
        value["kind"] = "parsed"
    elif re.fullmatch(r"Rc<\s*dyn\s+([A-Z]\w*)\s*>", inner) and \
            re.fullmatch(r"Rc<\s*dyn\s+([A-Z]\w*)\s*>", inner).group(1) in PARSED_INTERFACES:
        value["kind"] = "parsed"
    else:
        value["kind"] = "unsupported"
    return value


def children_of(model, name):
    """Where a class holds its children, as `ferro_add_child` places them."""
    chain = [name] + meta.ancestors(model, name)
    for base, kind in (("Popup", "child"), ("Panel", "children"), ("ItemsControl", "items"),
                       ("ContentControl", "content"), ("ContentPresenter", "content"),
                       ("Decorator", "child")):
        if base in chain:
            return kind
    return "none"


def fnv1a(text):
    value = 0xCBF29CE484222325
    for byte in text.encode("utf-8"):
        value = ((value ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    # A signed 64-bit integer: the type of `ferro_api_fingerprint`.
    return value - (1 << 64) if value >= (1 << 63) else value


# ---- the C ABI, read from the generated header ------------------------------------------------

C_TYPES = {
    "void": "void",
    "bool": "bool",
    "int32_t": "i32",
    "int64_t": "i64",
    "uint64_t": "u64",
    "double": "f64",
    "FerroStatus": "status",
    "FerroHandle": "handle",
    "const char*": "utf8",
    "char*": "utf8_buffer",
    "const uint16_t*": "utf16",
    "const uint8_t*": "bytes",
    "const void*": "pointer",
    "void*": "pointer",
    "int64_t*": "out_i64",
    "double*": "out_f64",
    "bool*": "out_bool",
    "FerroHandle*": "out_handle",
}


def c_type(text):
    text = " ".join(text.replace("*", " * ").split()).replace(" *", "*")
    return C_TYPES.get(text, text)


def read_header():
    """The functions, callback types and status codes of `include/ferroui.h`."""
    header = meta.read(HEADER)
    functions, callbacks, statuses = [], {}, []
    doc = []
    lines = header.split("\n")
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.startswith("// "):
            doc.append(line[3:])
            index += 1
            continue
        if line == "//":
            index += 1
            continue
        status = re.match(r"#define (FERRO_[A-Z0-9_]+) (-?\d+)$", line)
        if status and status.group(1) != "FERRO_ABI_VERSION":
            statuses.append({"name": status.group(1), "value": int(status.group(2)), "doc": " ".join(doc)})
        callback = re.match(r"typedef (.+?) \(\*(\w+)\)\((.*)\);$", line)
        if callback:
            params = [p.strip() for p in callback.group(3).split(",") if p.strip() and p.strip() != "void"]
            callbacks[callback.group(2)] = {
                "returns": c_type(callback.group(1)),
                "params": [param_of(p) for p in params],
                "doc": " ".join(doc),
            }
        function = re.match(r"^([A-Za-z_][\w ]*?\**)\s*\b(ferro_\w+)\((.*)$", line)
        if function and not line.startswith("typedef") and not line.startswith("#"):
            text = line
            while not text.rstrip().endswith(";"):
                index += 1
                text += " " + lines[index].strip()
            found = re.match(r"^(.+?)\b(ferro_\w+)\((.*)\);$", text)
            params = [p.strip() for p in found.group(3).split(",") if p.strip() and p.strip() != "void"]
            functions.append({
                "name": found.group(2),
                "returns": c_type(found.group(1).strip()),
                "params": [param_of(p) for p in params],
                "doc": " ".join(doc),
            })
        if not line.startswith("//"):
            doc = []
        index += 1
    return functions, callbacks, statuses


def param_of(text):
    found = re.match(r"^(.*?)(\w+)$", text.strip())
    kind = found.group(1).strip()
    return {"name": found.group(2), "type": c_type(kind) if kind not in ("FerroEventFn", "FerroStartFn", "FerroPostFn",
            "FerroEventDataFn", "FerroReleaseFn", "FerroPostDataFn", "FerroCreateFn", "FerroReleaseDataFn",
            "FerroLayoutFn", "FerroRenderFn", "FerroNotifyFn", "FerroPointerFn", "FerroKeyFn") else "callback:" + kind}


# ---- the frame layout ------------------------------------------------------------------------

# The byte layout of a frame (docs/bindings-design.md, section 4): records of 8-byte words,
# little-endian. A language that writes memory fills a buffer with these records and makes one
# `ferro_frame_commit_bytes` call per update; a language that cannot (Osprey) calls the scalar
# writer (`ferro_frame_open`, `ferro_frame_str`, ...), which writes the same records.
FRAME_LAYOUT = {
    "byte_order": "little-endian",
    "alignment": 8,
    "record_header": [
        {"field": "op", "type": "u32"},
        {"field": "size", "type": "u32", "doc": "the size of the whole record in bytes, a multiple of 8"},
    ],
    "records": [
        {"op": 1, "name": "open", "writer": "ferro_frame_open",
         "fields": [{"field": "type", "type": "i64"}, {"field": "key", "type": "i64"}],
         "doc": "Opens an element of the class `type` (a type ID). `key` tells siblings apart: 0 keeps"
                " the element by its position among the unkeyed siblings."},
        {"op": 2, "name": "close", "writer": "ferro_frame_close", "fields": [],
         "doc": "Closes the element opened last."},
        {"op": 3, "name": "text", "writer": "ferro_frame_str",
         "fields": [{"field": "property", "type": "i64"}, {"field": "length", "type": "i64"},
                    {"field": "bytes", "type": "u8[length]", "doc": "UTF-8, padded with zeros to 8 bytes"}],
         "doc": "Sets a property of the open element from text (as `ferro_set_str`)."},
        {"op": 4, "name": "number", "writer": "ferro_frame_f64",
         "fields": [{"field": "property", "type": "i64"}, {"field": "value", "type": "f64"}],
         "doc": "Sets a property from a number (as `ferro_set_f64`)."},
        {"op": 5, "name": "integer", "writer": "ferro_frame_i64",
         "fields": [{"field": "property", "type": "i64"}, {"field": "value", "type": "i64"}],
         "doc": "Sets a property from an integer or an enumeration member (as `ferro_set_i64`)."},
        {"op": 6, "name": "bool", "writer": "ferro_frame_bool",
         "fields": [{"field": "property", "type": "i64"}, {"field": "value", "type": "i64", "doc": "0 or 1"}],
         "doc": "Sets a property from a boolean (as `ferro_set_bool`)."},
        {"op": 7, "name": "on", "writer": "ferro_frame_on",
         "fields": [{"field": "event", "type": "i64"}, {"field": "tag", "type": "i64"},
                    {"field": "value_property", "type": "i64", "doc": "0 for none"}],
         "doc": "Binds an event of the open element to the message tag `tag`: the dispatcher"
                " (`ferro_frame_dispatcher`) is called with it, and with the value of `value_property`"
                " read when the event fires."},
        {"op": 8, "name": "text16", "writer": "ferro_frame_str16",
         "fields": [{"field": "property", "type": "i64"}, {"field": "units", "type": "i64"},
                    {"field": "code_units", "type": "u16[units]", "doc": "UTF-16, padded with zeros to 8 bytes"}],
         "doc": "Sets a property from UTF-16 text, for languages whose strings are UTF-16."},
        {"op": 9, "name": "key_text", "writer": "ferro_frame_key_str",
         "fields": [{"field": "length", "type": "i64"},
                    {"field": "bytes", "type": "u8[length]", "doc": "UTF-8, padded with zeros to 8 bytes"}],
         "doc": "Gives the open element a text key in place of its integer key."},
        {"op": 10, "name": "memo", "writer": "ferro_frame_memo",
         "fields": [{"field": "version", "type": "i64"}, {"field": "tag_base", "type": "i64"}],
         "doc": "Makes the open element a memo element: while its key and `version` stay, a commit"
                " does not look into its subtree, and the next frame can reuse it whole. The tags of its"
                " events are `tag_base` plus offsets the version fixes."},
        {"op": 11, "name": "reuse", "writer": "ferro_frame_reuse",
         "fields": [{"field": "key", "type": "i64"}, {"field": "version", "type": "i64"},
                    {"field": "tag_base", "type": "i64"}],
         "doc": "Stands for the memo element with the integer key `key` and the version `version` of the"
                " last frame under the root, with its tags moved to start at `tag_base`, in place of an"
                " element. `ferro_frame_reuse` writes it when the root has that element."},
        {"op": 12, "name": "bind", "writer": "ferro_frame_bind",
         "fields": [{"field": "property", "type": "i64"}, {"field": "slot", "type": "i64"}],
         "doc": "Binds a property of the open element to a slot of the signal table: a write of the slot"
                " updates the property at the next apply, with no new frame."},
    ],
}

# The signal table shared through memory (docs/c-abi.md, section 14): what a binding that can
# read and write memory needs to use it with loads and stores; the others call
# `ferro_signal_set_*` and `ferro_signal_get_*`.
SIGNAL_LAYOUT = {
    "table": "ferro_signal_table",
    "layout_version": 1,
    "kinds": [
        {"value": 1, "name": "i64"}, {"value": 2, "name": "f64", "doc": "the bits of the number"},
        {"value": 3, "name": "bool", "doc": "0 or 1"}, {"value": 4, "name": "handle"},
        {"value": 5, "name": "text", "doc": "the text is in the library; the value is its length in bytes"},
    ],
    "fields": [
        {"field": "layout", "type": "u32"}, {"field": "capacity", "type": "u32", "doc": "a multiple of 64"},
        {"field": "values", "type": "u64[capacity]", "writer": "the language (bound slots), the library (watched)"},
        {"field": "versions", "type": "u64[capacity]", "writer": "the library"},
        {"field": "dirty", "type": "u64[capacity / 64]", "writer": "the language, an atomic OR with release order"},
        {"field": "changed", "type": "u64[capacity / 64]", "writer": "the library sets, the language clears"},
        {"field": "kinds", "type": "u32[capacity]", "writer": "the library"},
    ],
    "apply": "ferro_signals_apply, the start of every frame commit and the end of every dispatch",
}

# Controls written in another language (docs/c-abi.md, section 15): what a backend needs to
# present a class registered through the ABI the way its language writes a class.
CONTROL_LAYOUT = {
    "register": "ferro_class_register",
    "bases": [
        {"value": 1, "name": "Control"}, {"value": 2, "name": "Panel"},
        {"value": 3, "name": "Decorator"}, {"value": 4, "name": "ContentControl"},
    ],
    "callbacks": [
        {"setter": "ferro_class_on_create", "type": "FerroCreateFn", "member": "created"},
        {"setter": "ferro_class_on_measure", "type": "FerroLayoutFn", "member": "measure_override"},
        {"setter": "ferro_class_on_arrange", "type": "FerroLayoutFn", "member": "arrange_override"},
        {"setter": "ferro_class_on_render", "type": "FerroRenderFn", "member": "render"},
        {"setter": "ferro_class_on_property_changed", "type": "FerroNotifyFn", "member": "on_property_changed"},
        {"setter": "ferro_class_on_attached", "type": "FerroNotifyFn", "member": "on_attached_to_visual_tree"},
        {"setter": "ferro_class_on_detached", "type": "FerroNotifyFn", "member": "on_detached_from_visual_tree"},
        {"setter": "ferro_class_on_apply_template", "type": "FerroNotifyFn", "member": "on_apply_template"},
        {"setter": "ferro_class_on_pointer", "type": "FerroPointerFn", "member": "on_pointer_*"},
        {"setter": "ferro_class_on_key", "type": "FerroKeyFn", "member": "on_key_*"},
    ],
    "property": {
        "register": "ferro_property_register",
        "kinds": "the kinds of signals: 1 i64, 2 f64, 3 bool, 5 text",
        "flags": [
            {"value": 1, "name": "affects_measure"}, {"value": 2, "name": "affects_arrange"},
            {"value": 4, "name": "affects_render"}, {"value": 8, "name": "inherits"}, {"value": 16, "name": "attached"},
        ],
    },
    "data": {"instance": "ferro_object_set_data", "class": "ferro_class_set_data"},
    "status": "a callback returns 0 (or 1: handled) on success, a negative status on a failure",
}


# ---- the model --------------------------------------------------------------------------------

def build():
    model = meta.Model()
    for directory, crate in meta.CRATES:
        meta.read_paths(model, directory, crate)
    for directory, crate in meta.CRATES:
        meta.read_classes(model, directory, crate)

    classes = sorted(name for name in model.bases if name not in model.ambiguous)

    # Properties by the name the ABI finds them by: the registered name, `Owner.Name` for an
    # attached property.
    by_name = {}
    used_enums = set()

    def declare(owner, kind, snake, value_type, holder, read_only):
        details = model.property_details.get((owner, snake), {})
        name = details.get("name") or pascal(snake)
        attached = kind == "AttachedProperty"
        abi_name = f"{owner}.{name}" if attached else name
        value = value_of(model, value_type)
        if value["kind"] == "enum":
            used_enums.add(value["enum"])
        entry = by_name.setdefault(abi_name, {"name": abi_name, "words": words(name if not attached else abi_name),
                                              "declarations": []})
        entry["declarations"].append({
            "owner": owner,
            "kind": {"StyledProperty": "styled", "DirectProperty": "direct", "AttachedProperty": "attached"}[kind],
            "value": value,
            "default": details.get("default"),
            "read_only": read_only,
            "doc": details.get("doc", ""),
        })
        return abi_name

    own_properties = {}
    for owner in sorted(model.properties):
        for kind, snake, value_type, target, holder in model.properties[owner]:
            own_properties.setdefault(owner, []).append(declare(owner, kind, snake, value_type, holder, False))
    for owner in sorted(model.read_only):
        for kind, snake, value_type, holder in model.read_only[owner]:
            own_properties.setdefault(owner, []).append(declare(owner, kind, snake, value_type, holder, True))

    events = {}
    own_events = {}
    for owner in sorted(model.events):
        for snake, args in model.events[owner]:
            details = model.event_details.get((owner, snake), {})
            name = details.get("name") or pascal(snake)
            entry = events.setdefault(name, {"name": name, "words": words(name), "declarations": [],
                                             "value_property": EVENT_VALUES.get(name),
                                             "dispatch": ["token", "user_data", "frame_tag"]})
            entry["declarations"].append({
                "owner": owner,
                "args": " ".join(args.split()),
                "raisable": args.strip() == "RoutedEventArgs",
                "doc": details.get("doc", ""),
            })
            own_events.setdefault(owner, []).append(name)

    type_names = classes
    property_names = sorted(by_name)
    event_names = sorted(events)
    fingerprint = fnv1a("\n".join(["types"] + type_names + ["properties"] + property_names + ["events"] + event_names))

    type_ids = {name: index + 1 for index, name in enumerate(type_names)}
    property_ids = {name: index + 1 for index, name in enumerate(property_names)}
    event_ids = {name: index + 1 for index, name in enumerate(event_names)}

    out_classes = []
    for name in classes:
        is_control = name == "Control" or "Control" in meta.ancestors(model, name)
        out_classes.append({
            "id": type_ids[name],
            "name": name,
            "words": words(name),
            "base": model.bases.get(name) if model.bases.get(name) in type_ids else None,
            "rust_path": meta.class_path(model, name),
            "creatable": name in model.creatable,
            "control": is_control,
            "children": children_of(model, name),
            "properties": sorted(set(own_properties.get(name, []))),
            "events": sorted(set(own_events.get(name, []))),
            "doc": model.class_docs.get(name, ""),
        })

    out_properties = []
    for name in property_names:
        entry = by_name[name]
        entry["declarations"].sort(key=lambda d: d["owner"])
        out_properties.append({"id": property_ids[name], **entry})

    out_events = []
    for name in event_names:
        entry = events[name]
        entry["declarations"].sort(key=lambda d: d["owner"])
        out_events.append({"id": event_ids[name], **entry})

    out_enums = []
    for name in sorted(used_enums):
        found = model.enums[name]
        out_enums.append({
            "name": name,
            "words": words(name),
            "rust_path": model.paths.get(name) if name not in model.ambiguous else None,
            "members": [{"name": member, "words": words(member), "value": value, "doc": doc}
                        for member, value, doc in found["members"]],
            "doc": found["doc"],
        })

    functions, callbacks, statuses = read_header()
    abi_version = int(re.search(r"#define FERRO_ABI_VERSION (\d+)", meta.read(HEADER)).group(1))

    document = {
        "format": MODEL_FORMAT,
        "generated_by": "scripts/generate_api_model.py",
        "abi_version": abi_version,
        "fingerprint": fingerprint,
        "naming": "Names are the names of the framework; `words` are their words in lower case, for"
                  " each backend to apply the casing of its language.",
        "abi": {"functions": functions, "callbacks": callbacks, "statuses": statuses},
        "frame": FRAME_LAYOUT,
        "signals": SIGNAL_LAYOUT,
        "controls": CONTROL_LAYOUT,
        "classes": out_classes,
        "properties": out_properties,
        "events": out_events,
        "enums": out_enums,
    }
    return document, (type_names, property_names, event_names, fingerprint)


def id_table(type_names, property_names, event_names, fingerprint):
    def table(name, names, doc):
        lines = [f"/// {doc}", f"pub(crate) static {name}: [&str; {len(names)}] = ["]
        lines += [f"    {json.dumps(n)}," for n in names]
        lines.append("];")
        return lines
    lines = [
        "// @generated by scripts/generate_api_model.py from the sources of the framework: do not edit.",
        "//! The stable IDs of the API model (`bindings/model/ferroui-api.json`): the library interns",
        "//! these names in this order when it starts, so the ID of a name is its index plus one, the",
        "//! constant a generated binding calls the ABI with (`ids.rs`).",
        "",
        "/// The fingerprint of the tables below ([`ferro_api_fingerprint`](crate::ferro_api_fingerprint)):",
        "/// a binding generated from the same model has the same one.",
        f"pub(crate) const API_FINGERPRINT: i64 = {fingerprint};",
        "",
    ]
    lines += table("TYPE_NAMES", type_names, "The classes, by type ID.")
    lines.append("")
    lines += table("PROPERTY_NAMES", property_names, "The properties, by property ID (`Owner.Name` for an attached one).")
    lines.append("")
    lines += table("EVENT_NAMES", event_names, "The routed events, by event ID.")
    lines.append("")
    return "\n".join(lines)


def write_or_check(path, text, check):
    current = meta.read(path) if os.path.exists(path) else None
    if current == text:
        return False
    if check:
        print(f"stale: {os.path.relpath(path, ROOT)}")
        return True
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print(f"wrote {os.path.relpath(path, ROOT)}")
    return False


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--check", action="store_true")
    options = parser.parse_args()
    document, tables = build()
    text = json.dumps(document, indent=1, ensure_ascii=False) + "\n"
    stale = write_or_check(MODEL, text, options.check)
    stale |= write_or_check(ID_TABLE, id_table(*tables), options.check)
    if stale:
        sys.exit(1)


if __name__ == "__main__":
    main()
