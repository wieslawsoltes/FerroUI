#!/usr/bin/env python3
"""Where the bytes of a browser WebAssembly module are.

    python3 scripts/browser/wasm-breakdown.py <module.wasm> <link.map> [--top N]

<module.wasm> is the module as the browser loads it, linked with a name section
(`--profiling-funcs`); <link.map> is the map wasm-ld writes for the same link
(`-Wl,--Map=<file>`). See docs/porting/browser-size.md for the commands.

The map gives the input of every function: the crate whose code generation unit
holds it (a generic function is in the crate that instantiates it), or the
archive member of a native library (Skia, HarfBuzz, the Emscripten system
libraries). The module gives the size of every function after the optimisation
the link ran (wasm-opt). They are joined by function name; functions that
wasm-opt renamed or merged are listed as not attributed. Data segments are
attributed from the map (sizes before wasm-opt, which drops zero bytes and
merges segments). Prints Markdown tables. Standard library only.
"""
import collections
import re
import sys


def leb(data, i):
    result = shift = 0
    while True:
        byte = data[i]
        i += 1
        result |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            return result, i


def read_module(path):
    """Sections (id, name, size) and the functions as (name, size of the body with its length)."""
    data = open(path, "rb").read()
    i = 8
    sections = []
    while i < len(data):
        section_id = data[i]
        i += 1
        size, i = leb(data, i)
        sections.append((section_id, i, size))
        i += size
    imported_functions = 0
    names = {}
    bodies = []
    section_sizes = []
    known = {1: "type", 2: "import", 3: "function", 4: "table", 5: "memory", 6: "global", 7: "export",
             8: "start", 9: "element", 10: "code", 11: "data", 12: "data count", 13: "tag"}
    for section_id, start, size in sections:
        label = known.get(section_id, str(section_id))
        if section_id == 0:
            length, j = leb(data, start)
            label = "custom " + data[j:j + length].decode()
        section_sizes.append((label, size))
        if section_id == 2:
            count, j = leb(data, start)
            for _ in range(count):
                length, j = leb(data, j)
                j += length
                length, j = leb(data, j)
                j += length
                kind = data[j]
                j += 1
                if kind == 0:
                    _, j = leb(data, j)
                    imported_functions += 1
                elif kind == 1:
                    j += 1
                    flags, j = leb(data, j)
                    _, j = leb(data, j)
                    if flags & 1:
                        _, j = leb(data, j)
                elif kind == 2:
                    flags, j = leb(data, j)
                    _, j = leb(data, j)
                    if flags & 1:
                        _, j = leb(data, j)
                elif kind == 3:
                    j += 2
                elif kind == 4:
                    j += 1
                    _, j = leb(data, j)
        elif section_id == 10:
            count, j = leb(data, start)
            for _ in range(count):
                body_start = j
                length, j = leb(data, j)
                j += length
                bodies.append(j - body_start)
        elif section_id == 0:
            length, j = leb(data, start)
            if data[j:j + length] == b"name":
                j += length
                end = start + size
                while j < end:
                    kind = data[j]
                    j += 1
                    sub_size, j = leb(data, j)
                    sub_end = j + sub_size
                    if kind == 1:
                        count, j = leb(data, j)
                        for _ in range(count):
                            index, j = leb(data, j)
                            length, j = leb(data, j)
                            names[index] = data[j:j + length].decode("utf8", "replace")
                            j += length
                    j = sub_end
    functions = [(names.get(k + imported_functions, ""), size) for k, size in enumerate(bodies)]
    return section_sizes, functions


LINE = re.compile(r"^\s*(\S+)\s+([0-9a-f]+)\s+([0-9a-f]+)( +)(.*)$")


def read_map(path):
    """Origin of every function name, and the data inputs as (origin, size)."""
    origin_of = {}
    data_inputs = []
    section = None
    current = None
    with open(path, encoding="utf8", errors="replace") as lines:
        for line in lines:
            m = LINE.match(line.rstrip("\n"))
            if not m:
                continue
            indent, text = len(m.group(4)), m.group(5)
            size = int(m.group(3), 16)
            if indent == 1:
                # An output section ("-" in the address column) or a data segment inside one.
                if m.group(1) == "-":
                    section = text
                continue
            if indent == 9:
                current = text.rsplit(":(", 1)[0] if text.endswith(")") and ":(" in text else text
                if section == "DATA":
                    data_inputs.append((current, size, text))
            elif indent == 17 and section == "CODE" and current is not None:
                origin_of.setdefault(text, current)
    return origin_of, data_inputs


RUST_OBJECT = re.compile(r"/examples/themed_view\.(\w+)-[0-9a-f]{16}\.")


def origin_group(origin):
    """(group, detail) of an input: a crate, or a native library with its member."""
    if origin is None:
        return ("not attributed (renamed or merged by wasm-opt)", "")
    m = RUST_OBJECT.search(origin)
    if m:
        return ("rust:" + m.group(1), "")
    m = re.search(r"lib(\w+?)-[0-9a-f]{16}\.rlib\((.*)\)$", origin)
    if m:
        crate, member = m.groups()
        if crate == "skia_bindings":
            # Members of the prebuilt Skia archive: <component>.<source file>.o, with the
            # third-party libraries as lib<name>.<file>.o and the rust-skia shims as <hash>-<name>.o.
            library, _, rest = member.partition(".")
            library = re.sub(r"^lib", "", library)
            third_party = {"freetype2": "FreeType", "jpeg": "libjpeg-turbo", "jpeg12": "libjpeg-turbo",
                           "jpeg16": "libjpeg-turbo", "png": "libpng", "zlib": "zlib", "wuffs": "Wuffs"}
            if library in third_party and member.startswith("lib"):
                return ("native:skia third party: " + third_party[library], rest)
            if re.match(r"^[0-9a-f]{16}-", library):
                return ("native:skia bindings (rust-skia C++ shims)", member)
            return ("native:skia", library + "." + rest)
        if crate == "harfbuzz_sys":
            return ("native:harfbuzz", member)
        if crate == "compiler_builtins":
            return ("rust:compiler_builtins", "")
        return ("native:" + crate, member)
    m = re.search(r"/(lib[\w+.-]+?)\.a\((.*)\)$", origin)
    if m:
        return ("emscripten:" + m.group(1), m.group(2))
    if "/examples/themed_view." in origin:
        return ("rust:themed_view (example and allocator shim)", "")
    if origin.startswith("<internal>"):
        return ("linker generated", "")
    return ("other:" + origin, "")


def skia_part(member):
    """The part of Skia a member of the archive (<component>.<source file>.o) belongs to."""
    library, _, file = member.partition(".")
    if file.startswith("SkSL"):
        return "SkSL compiler and code generators"
    if library == "gpu":
        if file.startswith("GrGL"):
            return "Ganesh: GL backend"
        if re.search(r"Op$|Op\.|Ops|Renderer|Tessellat|Atlas|Triangulator", file):
            return "Ganesh: ops, path renderers, tessellation"
        return "Ganesh: context, resources, effects"
    if library == "gpu_shared":
        return "GPU shared code (text sub-runs, blending, tessellation)"
    if library == "pathops":
        return "path ops"
    if library == "skcms":
        return "skcms colour management"
    if library in ("typeface_freetype", "fontmgr_custom", "fontmgr_custom_directory"):
        return "FreeType font host and custom font manager"
    if library.startswith(("png_", "jpeg_", "wuffs")) or re.search(
            r"Codec|Bmp|Wbmp|Ico|Swizzler|Exif|Encoded|Tiff|Gainmap|Hdr|ICC|Sampler|ImageGenerator_FromEncoded|"
            r"PixmapUtils|Encoder|ColorPalette", file):
        return "image codecs and encoders"
    if "ImageFilter" in file or file.startswith("SkImageFilter"):
        return "image filters"
    if library == "skia":
        if re.search(r"PathEffect|Dash|Corner|Discrete|Trim|Gradient|ColorFilter|MaskFilter|Emboss|Shadow|Blenders|"
                     r"Patch|Poly", file):
            return "effects, gradients, colour and mask filters, shadows"
        return "other (logging, files, utilities)"
    if library == "core":
        if re.search(r"Blit|Scan|RasterPipeline|Edge|^SkDraw|Mask|Opts|Swizzle|Memset|Mipmap|AAClip|AlphaRuns|"
                     r"ConvertPixels|Sprite|Clipper|Stroke|RasterClip", file):
            return "core: raster (scan conversion, blitters, pipeline)"
        if re.search(r"Picture|Record|Drawable|BBH|RTree", file):
            return "core: picture recording and playback"
        if re.search(r"Glyph|Strike|Font|Typeface|Scaler|TextBlob|Slug|Descriptor|GlyphRun", file):
            return "core: text and glyph cache"
        if re.search(r"ImageFilter|Shader|ColorFilter|Blend|RuntimeEffect|KnownRuntimeEffects|Mesh|Vertices", file):
            return "core: shaders, blenders, runtime effects"
        return "core: canvas, paths, images, utilities"
    return library


ESCAPES = [("$LT$", "<"), ("$GT$", ">"), ("$u20$", " "), ("$RF$", "&"), ("$C$", ","), ("$BP$", "*"),
           ("$u7b$", "{"), ("$u7d$", "}"), ("$u5b$", "["), ("$u5d$", "]"), ("$u27$", "'"), ("$SP$", "@"),
           ("$u3b$", ";"), ("$u21$", "!"), ("$u22$", "\"")]


def rust_name(name):
    name = re.sub(r" \(\.llvm\.\d+\)$", "", name)
    name = re.sub(r"::h[0-9a-f]{16}$", "", name)
    if name.startswith("_$"):
        name = name[1:]
    for escape, replacement in ESCAPES:
        name = name.replace(escape, replacement)
    return name.replace("..", "::")


STD_CRATES = ("core", "alloc", "std", "hashbrown")


def rust_family(crate, name):
    """A family of a Rust function inside its crate."""
    n = rust_name(name)
    # The type a trait implementation is for, or the path of the function.
    m = re.match(r"^<(.+?) as (.+?)>::(.*)$", n)
    subject = m.group(1) if m else n
    subject = subject.lstrip("&*").replace("mut ", "").replace("dyn ", "")
    first = subject.split("::")[0].split("<")[0]
    if first in STD_CRATES or n.startswith("core::") or n.startswith("alloc::") or n.startswith("std::"):
        for family in ("drop_in_place", "fmt", "RawVec", "raw_vec", "Vec", "Rc", "RefCell", "hashbrown", "HashMap",
                       "sort", "slice", "iter", "FnOnce", "Fn", "Option", "Result", "Box", "String", "str", "panicking",
                       "BTreeMap", "btree", "Any", "Cell", "Clone", "PartialEq", "Ord", "Hash", "Iterator", "Debug"):
            if re.search(r"\b" + family + r"\b", n):
                return "std generic: " + family
        return "std generic: other"
    parts = subject.split("::")
    if parts and parts[0] == crate:
        parts = parts[1:]
    if crate == "ferroui_markup_xaml_loader" and len(parts) >= 2 and parts[0] in ("runtime", "compiler_extensions"):
        return "::".join(parts[:2]).split("<")[0]
    return parts[0].split("<")[0] if parts else "(root)"


def strip_generics(text):
    """The text without its generic arguments (balanced <...>), keeping qualified paths <X as Y>."""
    out, depth = [], 0
    for ch in text:
        if ch == "<":
            depth += 1
        elif ch == ">":
            depth -= 1
        elif depth == 0:
            out.append(ch)
    return "".join(out)


def head_path(name):
    """The path of the item a function belongs to, without generic arguments: for <X as Y>::m the
    path of X, for a closure the function it is in. Needs v0 symbol names (-Csymbol-mangling-version=v0)."""
    name = re.sub(r" \(\.llvm\.\d+\)$", "", name).strip()
    while name.startswith("<"):
        depth = 0
        for i, ch in enumerate(name):
            depth += ch == "<"
            depth -= ch == ">"
            if depth == 0:
                break
        inner, rest = name[1:i], name[i + 1:]
        # The self type of a qualified path: up to the top-level " as ".
        depth = 0
        cut = len(inner)
        for j, ch in enumerate(inner):
            depth += ch == "<"
            depth -= ch == ">"
            if depth == 0 and inner.startswith(" as ", j):
                cut = j
                break
        self_type = inner[:cut].lstrip("&*").replace("mut ", "").replace("dyn ", "")
        name = self_type + rest if not self_type.startswith("(") else rest.lstrip(":")
        if not name:
            return ""
    return strip_generics(name)


# Families of the framework code, by the path a function belongs to (first match wins).
FAMILIES = [
    ("markup metadata tables (markup_types: register_value_types and closures)", r"::markup_types::"),
    ("class registration (register_types)", r"::register_types::|::register_types$"),
    ("value type registry (ValueTypes, value_type)", r"::data::core::value_type"),
    ("property store (values, bindings, observers)", r"ferroui_base::property_store::"),
    ("property definitions and metadata", r"ferroui_base::(styled_property|direct_property|ferro_property|ferro_property_metadata|ferro_property_registry|attached_property|property_metadata)"),
    ("class model (vtables, interfaces, casts)", r"build_vtable|__register_interfaces|ferroui_base::type_system::|::object_casts::"),
    ("XAML loader: run-time type system", r"ferroui_markup_xaml_loader::runtime::type_system"),
    ("XAML loader: interpreter", r"ferroui_markup_xaml_loader::runtime::interpreter"),
    ("XAML loader: framework nodes", r"ferroui_markup_xaml_loader::runtime"),
    ("XAML front end: transformers and compiler (loader crate)", r"ferroui_markup_xaml_loader::"),
    ("XAML front end: XamlX core", r"^xamlx::"),
    ("markup extensions and converters (ferroui_markup_xaml)", r"^ferroui_markup_xaml::"),
    ("animation", r"::animation::"),
    ("styling and themes", r"::styling::|ferroui_themes_"),
    ("layout, controls and templates", r"^ferroui_controls::"),
    ("text formatting and Unicode", r"::text_formatting::|::text_processing::"),
    ("media, rendering and composition", r"ferroui_base::(media|rendering)::|^ferroui_skia::|^ferroui_harfbuzz::|^ferroui_opengl::"),
    ("input and interactivity", r"ferroui_base::(input|interactivity)::"),
    ("reactive (observables, disposables)", r"ferroui_base::reactive::"),
    ("rest of ferroui_base", r"^ferroui_base::"),
    ("browser platform and example", r"^ferroui_browser::|^themed_view::|^wasm_bindgen::"),
    ("std drop glue", r"^core::ptr::drop_in_place"),
    ("std formatting and panics", r"^core::(fmt|panicking)|^std::panicking|^alloc::fmt"),
    ("std collections, Rc, iterators and the rest", r"^(core|alloc|std|hashbrown|panic_unwind|compiler_builtins)::|"
     r"^(\[|&|for fn|__|fmodf?$|cbrtf?$|roundf?$|fmaf?$|type_id$)|^(str|bool|char|[iuf](8|16|32|64|128|size)|T)(::|$)"),
]


def family_of(name):
    if "$LT$" in name or ".." in name:
        name = rust_name(name)
    head = head_path(name)
    for label, pattern in FAMILIES:
        if re.search(pattern, head):
            return label
    return "other crates: " + (head.split("::")[0] if head else "?")


def mb(value):
    return f"{value / 1e6:.2f}"


def main():
    args = sys.argv[1:]
    top = 40
    if "--top" in args:
        top = int(args[args.index("--top") + 1])
        del args[args.index("--top"):args.index("--top") + 2]
    module_path, map_path = args
    sections, functions = read_module(module_path)
    origin_of, data_inputs = read_map(map_path)

    total = sum(size for _, size in sections)
    print("### Sections\n")
    print("| Section | Bytes | MB | Share |")
    print("|---|---:|---:|---:|")
    for label, size in sections:
        print(f"| {label} | {size:,} | {mb(size)} | {100 * size / total:.1f}% |")
    code_total = sum(size for _, size in functions)

    groups = collections.Counter()
    group_counts = collections.Counter()
    skia_parts = collections.Counter()
    rust_families = collections.defaultdict(collections.Counter)
    std_in_crates = collections.Counter()
    std_families = collections.Counter()
    attributed = []
    for name, size in functions:
        origin = origin_of.get(name)
        if origin is None:
            origin = origin_of.get(re.sub(r" \(\.llvm\.\d+\)$", "", name))
        group, member = origin_group(origin)
        groups[group] += size
        group_counts[group] += 1
        attributed.append((size, name, group))
        if group == "native:skia":
            skia_parts[skia_part(member)] += size
        if group.startswith("rust:"):
            crate = group[5:]
            family = rust_family(crate, name)
            rust_families[crate][family] += size
            if family.startswith("std generic"):
                std_in_crates[crate] += size
                std_families[family] += size

    print("\n### Code by crate or library\n")
    print(f"Code section: {code_total:,} bytes in {len(functions):,} functions (body sizes after wasm-opt).\n")
    print("| Crate or library | Functions | Bytes | MB | Share of code |")
    print("|---|---:|---:|---:|---:|")
    for group, size in groups.most_common():
        print(f"| {group} | {group_counts[group]:,} | {size:,} | {mb(size)} | {100 * size / code_total:.1f}% |")

    print("\n### Skia by part (code; third-party libraries and shims are listed above)\n")
    skia_total = sum(skia_parts.values())
    print("| Part | Bytes | MB | Share of Skia |")
    print("|---|---:|---:|---:|")
    for part, size in skia_parts.most_common():
        print(f"| {part} | {size:,} | {mb(size)} | {100 * size / max(skia_total, 1):.1f}% |")

    print("\n### Rust crates by module (code, largest families)\n")
    for crate, families in sorted(rust_families.items(), key=lambda item: -sum(item[1].values())):
        crate_total = sum(families.values())
        if crate_total < 200_000:
            continue
        print(f"\n`{crate}`: {crate_total:,} bytes\n")
        print("| Module or family | Bytes | MB | Share of crate |")
        print("|---|---:|---:|---:|")
        for family, size in families.most_common(15):
            print(f"| {family} | {size:,} | {mb(size)} | {100 * size / crate_total:.1f}% |")

    print("\n### Generic functions of core, alloc, std and hashbrown instantiated in the crates\n")
    print("| Instantiated in | Bytes | MB |")
    print("|---|---:|---:|")
    for crate, size in std_in_crates.most_common():
        print(f"| {crate} | {size:,} | {mb(size)} |")
    print("\n| Family | Bytes | MB |")
    print("|---|---:|---:|")
    for family, size in std_families.most_common(20):
        print(f"| {family} | {size:,} | {mb(size)} |")

    # The crate in the path of every Rust function: with fat LTO all Rust code comes from one object.
    path_crates = collections.Counter()
    path_counts = collections.Counter()
    for size, name, group in attributed:
        if group.startswith("rust:") or (group.startswith("not attributed") and ("::" in name or "$LT$" in name)):
            head = head_path(rust_name(name) if ("$LT$" in name or ".." in name) else name)
            crate = re.split(r"::|<", head)[0] if head else "?"
            if not re.match(r"^[a-z_][a-z0-9_]*$", crate):
                crate = "core (primitive types)"
            if group.startswith("not attributed"):
                crate += " (not attributed)"
            path_crates[crate] += size
            path_counts[crate] += 1
    print("\n### Rust code by the crate in the function path\n")
    print("A generic function counts for the crate that defines it, whatever crate instantiates it.\n")
    print("| Crate | Functions | Bytes | MB |")
    print("|---|---:|---:|---:|")
    for crate, size in path_crates.most_common(25):
        print(f"| {crate} | {path_counts[crate]:,} | {size:,} | {mb(size)} |")

    if any("{closure#" in name or "::<" in name for name, _ in functions[:20000]):
        families = collections.Counter()
        family_counts = collections.Counter()
        hot = collections.Counter()
        hot_counts = collections.Counter()
        for size, name, group in attributed:
            if not group.startswith("rust:"):
                continue
            label = family_of(name)
            families[label] += size
            family_counts[label] += 1
            # Generic hot spots: the item a function belongs to, with how many instances it has.
            key = re.sub(r"::(::)+", "::", head_path(rust_name(name) if "$LT$" in name else name)).rstrip(":")
            if "<" in re.sub(r" \(\.llvm\.\d+\)$", "", name).split(" as ")[0] or "::<" in name:
                hot[key] += size
                hot_counts[key] += 1
        rust_total = sum(families.values())
        print("\n### Rust code by family (all crates)\n")
        print(f"Rust code: {rust_total:,} bytes. A generic function counts where its path is, whatever crate instantiates it.\n")
        print("| Family | Functions | Bytes | MB | Share of Rust code |")
        print("|---|---:|---:|---:|---:|")
        for label, size in families.most_common():
            print(f"| {label} | {family_counts[label]:,} | {size:,} | {mb(size)} | {100 * size / rust_total:.1f}% |")
        print("\n### Monomorphization hot spots (generic items by total size of their instances)\n")
        print("| Item | Instances | Bytes | MB |")
        print("|---|---:|---:|---:|")
        for key, size in hot.most_common(top):
            print(f"| `{key[:120]}` | {hot_counts[key]:,} | {size:,} | {mb(size)} |")

    print(f"\n### Largest {top} functions\n")
    print("| Bytes | Function | Crate or library |")
    print("|---:|---|---|")
    for size, name, group in sorted(attributed, reverse=True)[:top]:
        shown = rust_name(name) if group.startswith("rust:") else name
        shown = shown.replace("|", "\\|")
        if len(shown) > 140:
            shown = shown[:137] + "..."
        print(f"| {size:,} | `{shown}` | {group} |")

    print("\n### Data by crate or library (from the link map, before wasm-opt)\n")
    data_groups = collections.Counter()
    data_detail = collections.Counter()
    for origin, size, text in data_inputs:
        group, member = origin_group(origin)
        data_groups[group] += size
        if group == "native:skia":
            data_detail[("native:skia", skia_part(member))] += size
    data_total = sum(data_groups.values())
    print(f"Data inputs: {data_total:,} bytes.\n")
    print("| Crate or library | Bytes | MB | Share of data |")
    print("|---|---:|---:|---:|")
    for group, size in data_groups.most_common(25):
        print(f"| {group} | {size:,} | {mb(size)} | {100 * size / data_total:.1f}% |")
    print(f"\nLargest {top // 2} data inputs:\n")
    print("| Bytes | Input | Crate or library |")
    print("|---:|---|---|")
    for origin, size, text in sorted(data_inputs, key=lambda item: -item[1])[:top // 2]:
        group, member = origin_group(origin)
        symbol = text.rsplit(":(", 1)[1][:-1] if ":(" in text else text
        print(f"| {size:,} | `{symbol[:100]}` | {group}{' ' + member if member else ''} |")
    if data_detail:
        print("\n| Skia part (data) | Bytes | MB |")
        print("|---|---:|---:|")
        for (_, part), size in data_detail.most_common():
            print(f"| {part} | {size:,} | {mb(size)} |")


if __name__ == "__main__":
    main()
