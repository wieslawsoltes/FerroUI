#!/usr/bin/env python3
"""Name the allocation sites a start of the frame benchmarks printed, and add them up.

A start run with the feature `counters` and `FERROUI_HEAP_SITES=<phase>` prints the blocks that
phase left allocated by the frames of the call that allocated each (`heap sites`, see
`tests/FerroUI.FrameBenchmarks/tests/heap/mod.rs`):

    FERROUI_HEAP_SITES=show cargo test -p ferroui-frame-benchmarks --release --features counters \\
        --test frames start_of_controls -- --ignored --nocapture --test-threads=1 > start.txt
    python3 scripts/perf/heap-sites.py start.txt

The frames are addresses; `atos` (macOS) names them from the program the output names, which has
to be the build that ran. Two tables come out:

- by owner: every site is given to the innermost of its frames that is not of the allocator, the
  standard library or a collection (`--skip` adds prefixes), so a vector that grew is counted for
  the function that pushed;
- by path (`--paths N`): the largest sites with the owner and the frames outside it, which says
  on whose behalf the owner allocated.

A function inlined into its caller has no frame and is counted for the caller; a build of the
crates in question with line tables (`profile.release.package.<name>.debug = "line-tables-only"`)
lets `atos -i` name the inlined ones too.
"""

import argparse
import re
import subprocess
import sys

# A frame of these belongs to whoever called it.
PLUMBING = (
    "__rust", "__rg_", "__rdl_", "_rjem", "alloc::", "<alloc::", "core::", "<core::", "std::", "<std::",
    "hashbrown::", "<hashbrown::", "smallvec::", "<smallvec::", "indexmap::", "<indexmap::", "frames::heap::",
    "<frames::heap::", "_$LT$alloc", "_$LT$core", "_$LT$std",
)


def read(path):
    text = sys.stdin.read() if path == "-" else open(path, errors="replace").read()
    start = re.search(r"^\S+ heap sites:\n", text, re.M)
    if not start:
        sys.exit("no `heap sites` in the output: was it a start with the feature `counters` and "
                 "FERROUI_HEAP_SITES set?")
    image = sums = None
    sites = []
    for line in text[start.end():].splitlines():
        words = line.split()
        if not words or line.endswith("heap sites end"):
            break
        if words[0] == "image":
            image = (" ".join(words[1:-1]), words[-1])
        elif words[0] == "sum":
            sums = line[4:]
        elif words[0] == "site":
            sites.append((int(words[1]), int(words[2]), int(words[3]), words[4:]))
    if image is None:
        sys.exit("the `heap sites` of the output name no program")
    return image, sums, sites


def names(image, addresses, inlined):
    """The name of the function each address is in. With `inlined`, the functions inlined at the
    address come first, joined by ` < `."""
    program, loaded = image
    addresses = sorted(addresses)
    named = {}
    for at in range(0, len(addresses), 2000):
        part = addresses[at:at + 2000]
        command = ["atos", "-o", program, "-l", loaded] + (["-i"] if inlined else []) + part
        output = subprocess.run(command, text=True, capture_output=True, check=True).stdout
        # With `-i` an address is a group of lines, and the groups are apart by an empty line.
        groups = output.strip("\n").split("\n\n") if inlined else output.splitlines()
        if len(groups) != len(part):
            sys.exit(f"atos named {len(groups)} of {len(part)} addresses")
        for address, group in zip(part, groups):
            named[address] = " < ".join(short(line) for line in group.splitlines() if line.strip())
    return named


def short(line):
    """`name (in program) (file:line)` or `name (in program) + offset` without what follows the name
    and without the hash of a mangled name."""
    name = re.sub(r" \(in [^)]*\).*$", "", line.strip())
    return re.sub(r"::h[0-9a-f]{16}$", "", name)


def is_plumbing(name, skip):
    return name.startswith(skip) or name.startswith("0x")


def megabytes(count):
    return f"{count / (1024 * 1024):9.2f}"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("output", help="the output of the start, or - for the standard input")
    parser.add_argument("--owners", type=int, default=40, help="the owners to print (default 40)")
    parser.add_argument("--paths", type=int, default=0, help="the largest sites to print with their frames")
    parser.add_argument("--frames", type=int, default=8, help="the frames of a path to print (default 8)")
    parser.add_argument("--skip", action="append", default=[], help="a prefix of names that own nothing")
    parser.add_argument("--inlined", action="store_true", help="ask atos for the inlined functions (-i)")
    parser.add_argument("--match", help="only the sites with a frame whose name contains this")
    arguments = parser.parse_args()
    skip = PLUMBING + tuple(arguments.skip)

    image, sums, sites = read(arguments.output)
    named = names(image, {frame for site in sites for frame in site[3]}, arguments.inlined)

    paths = []
    for size, blocks, allocations, frames in sites:
        # An inlined group is several functions of one frame, the innermost first.
        path = [name for frame in frames for name in named[frame].split(" < ")]
        if arguments.match and not any(arguments.match in name for name in path):
            continue
        own = next((index for index, name in enumerate(path) if not is_plumbing(name, skip)), None)
        owner = path[own] if own is not None else "(the allocator and the library alone)"
        paths.append((size, blocks, allocations, owner, path[own + 1:] if own is not None else path))

    total = sum(path[0] for path in paths) or 1
    print(f"{sums}")
    print(f"{megabytes(total)} MB in {sum(path[1] for path in paths)} blocks of the sites read\n")

    owners = {}
    for size, blocks, allocations, owner, _ in paths:
        entry = owners.setdefault(owner, [0, 0, 0])
        entry[0] += size
        entry[1] += blocks
        entry[2] += allocations
    print("       MB  share    blocks  bytes a block  allocations  owner")
    for owner, (size, blocks, allocations) in sorted(owners.items(), key=lambda item: -item[1][0])[:arguments.owners]:
        print(f"{megabytes(size)} {100 * size / total:5.1f} % {blocks:9} {size / max(blocks, 1):14.0f} "
              f"{allocations:12}  {owner}")

    if arguments.paths:
        print("\nThe largest sites:")
        for size, blocks, allocations, owner, callers in sorted(paths, key=lambda path: -path[0])[:arguments.paths]:
            print(f"{megabytes(size)} MB, {blocks} blocks, {allocations} allocations: {owner}")
            for name in callers[:arguments.frames]:
                print(f"            {name}")


if __name__ == "__main__":
    main()
