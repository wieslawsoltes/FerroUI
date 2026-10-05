#!/usr/bin/env python3
"""What a WebAssembly module of a browser application is made of.

    python3 scripts/browser/wasm-size-report.py <module.wasm> [--assets <directory>...] [--top <n>]

Prints, as Markdown tables:

- the size of each section of the module, raw and with gzip -9;
- with --assets, the files below the given directories whose content is in the data section
  (embedded fonts, markup, pictures), by file extension;
- when the module has a name section, the code by origin (Rust crate, Skia with FreeType and the
  image codecs, HarfBuzz, the C and C++ runtime) and by symbol family (the function path with its
  generic arguments erased, so that the instances of one generic function are counted together).
  The gzip column of those tables compresses the function bodies of a row on their own: it shows
  how well the row compresses, and the rows do not add up to the gzip size of the code section.

A module built by scripts/build-browser.sh has no name section. Link one with the names kept, for
example for the themed_view example:

    cargo rustc --profile browser --target wasm32-unknown-emscripten -p ferroui-browser \\
        --example themed_view -- -C link-arg=--profiling-funcs

Function names are demangled with llvm-cxxfilt of the Emscripten SDK when it is on PATH.
"""
import collections
import gzip
import os
import re
import shutil
import subprocess
import sys

SECTION_NAMES = {0: "custom", 1: "type", 2: "import", 3: "function", 4: "table", 5: "memory", 6: "global",
                 7: "export", 8: "start", 9: "element", 10: "code", 11: "data", 12: "data count", 13: "tag"}


def uleb(data, at):
    value = shift = 0
    while True:
        byte = data[at]
        at += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            return value, at


def sleb(data, at):
    value = shift = 0
    while True:
        byte = data[at]
        at += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            if byte & 0x40:
                value -= 1 << shift
            return value, at


def sections(data):
    """(id, name, start of the content, length of the content) of every section."""
    if data[:4] != b"\0asm":
        sys.exit("not a WebAssembly module")
    at = 8
    while at < len(data):
        section_id = data[at]
        size, at = uleb(data, at + 1)
        name = SECTION_NAMES.get(section_id, str(section_id))
        if section_id == 0:
            length, name_at = uleb(data, at)
            name = "custom " + data[name_at:name_at + length].decode("utf-8", "replace")
        yield section_id, name, at, size
        at += size


def imported_function_count(data, start):
    count, at = uleb(data, start)
    functions = 0
    for _ in range(count):
        for _ in range(2):  # module and field names
            length, at = uleb(data, at)
            at += length
        kind = data[at]
        at += 1
        if kind == 0:  # function: type index
            _, at = uleb(data, at)
            functions += 1
        elif kind == 1:  # table: reference type, limits
            at += 1
            flags, at = uleb(data, at)
            _, at = uleb(data, at)
            if flags & 1:
                _, at = uleb(data, at)
        elif kind == 2:  # memory: limits
            flags, at = uleb(data, at)
            _, at = uleb(data, at)
            if flags & 1:
                _, at = uleb(data, at)
        elif kind == 3:  # global: value type, mutability
            at += 2
        elif kind == 4:  # tag: attribute, type index
            at += 1
            _, at = uleb(data, at)
    return functions


def function_bodies(data, start):
    count, at = uleb(data, start)
    bodies = []
    for _ in range(count):
        size, body = uleb(data, at)
        bodies.append((at, body + size - at))
        at = body + size
    return bodies


def function_names(data, start, size):
    length, at = uleb(data, start)
    at += length
    names = {}
    while at < start + size:
        subsection = data[at]
        length, at = uleb(data, at + 1)
        if subsection == 1:
            count, entry = uleb(data, at)
            for _ in range(count):
                index, entry = uleb(data, entry)
                name_length, entry = uleb(data, entry)
                names[index] = data[entry:entry + name_length].decode("utf-8", "replace")
                entry += name_length
        at += length
    return names


LEGACY_ESCAPES = (("$LT$", "<"), ("$GT$", ">"), ("$u20$", " "), ("$C$", ","), ("$RF$", "&"), ("$BP$", "*"),
                  ("$u7b$", "{"), ("$u7d$", "}"), ("$u5b$", "["), ("$u5d$", "]"), ("$u27$", "'"), ("$LP$", "("),
                  ("$RP$", ")"), ("$SP$", "@"), ("$u3b$", ";"), ("$u2b$", "+"), ("$u22$", '"'))


def readable(names):
    """('rust' or 'c', readable name) for each name of the name section."""
    cxxfilt = shutil.which("llvm-cxxfilt")
    if cxxfilt:
        names = subprocess.run([cxxfilt], input="\n".join(names), capture_output=True, text=True).stdout.split("\n")
    result = []
    for name in names:
        name = re.sub(r" \(\.llvm\.\d+\)$", "", name)
        if re.search(r"::h[0-9a-f]{16}(_\d+)?$", name):
            name = re.sub(r"::h[0-9a-f]{16}(_\d+)?$", "", name)
            if name.startswith("_$"):
                name = name[1:]
            for escape, character in LEGACY_ESCAPES:
                name = name.replace(escape, character)
            result.append(("rust", name.replace("..", "::")))
        else:
            result.append(("c", name))
    return result


SKIA = re.compile(r"^(\(anonymous namespace\)::)?(Sk|Gr|sk[a-z_]*::|SkSL|skgpu|skif|skcms|skia|sktext|skvx|portable|hsw|"
                  r"FT_|TT_|tt_|af_|cff_|cf2_|ps_|t1_|sfnt|ft_|FTC|Ins_|Direct_Move|Round_|Compute_|Project|Dual_Project|"
                  r"Read_CVT|Write_CVT|Move_CVT|SetSuperRound|Current_Ratio|Normalize|Get_Short|Init_Context|Free_Context)")
HARFBUZZ = re.compile(r"^(hb_|_hb_|OT::|AAT::|CFF::|graph::|hb::|\w+_shaper|(arabic|indic|khmer|myanmar|use|hangul|thai|"
                      r"hebrew|syllabic)_|setup_syllables|reorder|collect_features|override_features|data_create|"
                      r"data_destroy|decompose|compose|preprocess_text|postprocess_glyphs|initial_reordering|"
                      r"final_reordering|find_syllables|record_|insert_dotted)")
CODECS = re.compile(r"^(jpeg|jinit|jcopy|jzero|jround|jdiv|jsimd|jpeg_|encode_mcu|decode_mcu|png_|inflate|deflate|"
                    r"crc32|adler32|zcalloc|zcfree|_tr_|fill_window|longest_match|wuffs)")
RUNTIME = re.compile(r"^(emscripten|__|_emscripten|std::|operator |dl|malloc|free|realloc|calloc|mem|str|sbrk|abort|"
                     r"_Unwind|__cxa|__cxx|stackSave|stackRestore|stackAlloc|setThrew|fflush|printf|vfprintf|"
                     r"fmt_|pop_arg|pad|out|qsort|wcrtomb|frexp|getenv|pthread)")
GENERIC_PARAMETERS = {"T", "S", "F", "A", "M", "I", "V", "K", "dyn", "mut"}


def origin(kind, name):
    if kind == "rust":
        if name.startswith("core::ptr::drop_in_place<"):
            inner = re.match(r"[(&*\[]*(?:mut |const |dyn )?<?([A-Za-z_]\w*)", name[len("core::ptr::drop_in_place<"):])
            return f"Rust drop glue of {inner.group(1) if inner else '?'} types"
        crate = re.match(r"<?([A-Za-z_]\w*)", name)
        crate = crate.group(1) if crate else "?"
        if crate in GENERIC_PARAMETERS:  # <T as crate::Trait>::method
            trait = re.search(r" as ([A-Za-z_]\w*)::", name)
            crate = trait.group(1) if trait else crate
        return f"Rust {crate}"
    if SKIA.match(name):
        return "C++ Skia (with FreeType)"
    if HARFBUZZ.match(name):
        return "C++ HarfBuzz"
    if CODECS.match(name):
        return "C image codecs and zlib"
    if RUNTIME.match(name):
        return "C/C++ runtime (libc, libc++, Emscripten)"
    return "C/C++ other"


def family(name):
    """The name with the generic arguments erased."""
    out, depth = [], 0
    for character in name:
        if character == "<":
            depth += 1
            if depth == 1:
                out.append("<_>")
        elif character == ">":
            depth = max(depth - 1, 0)
        elif depth == 0:
            out.append(character)
    return "".join(out)


def gzip_size(data):
    return len(gzip.compress(data, 9))


def mb(size):
    return f"{size / 1e6:.2f}"


def main():
    args = sys.argv[1:]
    if not args or args[0].startswith("-"):
        sys.exit(__doc__)
    path = args[0]
    top = int(args[args.index("--top") + 1]) if "--top" in args else 30
    asset_dirs = []
    if "--assets" in args:
        for value in args[args.index("--assets") + 1:]:
            if value.startswith("--"):
                break
            asset_dirs.append(value)

    data = open(path, "rb").read()
    found = list(sections(data))
    print(f"`{os.path.basename(path)}`: {mb(len(data))} MB, {mb(gzip_size(data))} MB with gzip -9\n")
    print("| Section | MB | MB gzip |\n|---|---:|---:|")
    for _, name, start, size in found:
        if size >= 1000:
            print(f"| {name} | {mb(size)} | {mb(gzip_size(data[start:start + size]))} |")

    by_id = {section_id: (start, size) for section_id, _, start, size in found if section_id != 0}
    if asset_dirs and 11 in by_id:
        start, size = by_id[11]
        segments = data[start:start + size]
        raw, packed = collections.Counter(), collections.Counter()
        for directory in asset_dirs:
            for parent, _, files in os.walk(directory):
                for file in files:
                    content = open(os.path.join(parent, file), "rb").read()
                    if len(content) < 512:
                        continue
                    # wasm-opt splits data segments at long runs of zeros, so the file is looked for by
                    # its first and last bytes.
                    head = segments.find(content[:256])
                    tail = segments.find(content[-256:], max(head, 0))
                    if head >= 0 and head <= tail < head + len(content) + 4096:
                        extension = os.path.splitext(file)[1].lower() or file
                        raw[extension] += len(content)
                        packed[extension] += gzip_size(content)
        print("\n| Embedded files | MB | MB gzip (file by file) |\n|---|---:|---:|")
        for extension, size in raw.most_common():
            print(f"| `{extension}` | {mb(size)} | {mb(packed[extension])} |")

    names_section = next(((start, size) for _, name, start, size in found if name == "custom name"), None)
    if names_section is None or 10 not in by_id:
        print("\nThe module has no name section: no attribution of the code.")
        return
    imported = imported_function_count(data, by_id[2][0]) if 2 in by_id else 0
    names = function_names(data, *names_section)
    bodies = function_bodies(data, by_id[10][0])
    readable_names = readable([names.get(imported + i, f"function {imported + i}") for i in range(len(bodies))])

    groups = {"origin": collections.defaultdict(list), "family": collections.defaultdict(list)}
    for (start, size), (kind, name) in zip(bodies, readable_names):
        groups["origin"][origin(kind, name)].append((start, size))
        groups["family"][family(name)].append((start, size))
    total = sum(size for _, size in bodies)
    for title, key in (("Code by origin", "origin"), ("Code by symbol family", "family")):
        rows = sorted(groups[key].items(), key=lambda item: -sum(size for _, size in item[1]))[:top]
        print(f"\n| {title} | Functions | MB | % of code | MB gzip (alone) |\n|---|---:|---:|---:|---:|")
        for label, items in rows:
            size = sum(s for _, s in items)
            packed = gzip_size(b"".join(data[s:s + n] for s, n in items))
            label = label if len(label) <= 140 else label[:137] + "..."
            print(f"| `{label}` | {len(items)} | {mb(size)} | {100 * size / total:.1f} | {mb(packed)} |")


if __name__ == "__main__":
    main()
