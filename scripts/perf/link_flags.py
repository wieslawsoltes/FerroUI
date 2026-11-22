#!/usr/bin/env python3
"""The options of a link line from the output of `rustc --print=link-args`, and how two differ (#1001).

Objects, libraries, search paths and frameworks are left out, and the path in an option is replaced
by `<path>`, so that what remains is how the linker was asked to link.

    link_flags.py <output of rustc> [<output of rustc for another target>]
    link_flags.py --self-test
"""
import re
import shlex
import sys

WITH_VALUE = {"-arch", "-target", "-isysroot", "--target"}
LEFT_OUT = {"-o", "-L", "-framework", "-l", "-nodefaultlibs"}


def link_line(text):
    """The longest line of the output that runs a linker with options."""
    lines = [line for line in text.splitlines() if ' "-' in line or " -" in line]
    if not lines:
        raise ValueError("no link line in this output")
    return max(lines, key=len)


def without_paths(option):
    return re.sub(r"(?:[A-Za-z]:)?/[^,=\s]+", "<path>", option)


def options(text):
    """The sorted options of the link line in `text`."""
    tokens = shlex.split(link_line(text))
    # `KEY=value` before the linker is its environment; the linker is the first other word.
    while tokens and re.match(r"[A-Za-z_][A-Za-z0-9_]*=", tokens[0]):
        tokens.pop(0)
    tokens = tokens[1:]
    found = set()
    index = 0
    while index < len(tokens):
        token = tokens[index]
        index += 1
        if not token.startswith("-"):
            continue
        if token in LEFT_OUT:
            index += 1
            continue
        if token.startswith(("-L", "-l")) and not token.startswith("-lto"):
            continue
        if token in WITH_VALUE and index < len(tokens):
            found.add(f"{token} {without_paths(tokens[index])}")
            index += 1
            continue
        found.add(without_paths(token))
    return sorted(found)


def self_test():
    line = ('LC_ALL="C" PATH="/a:/b" "cc" "-arch" "arm64" "/tmp/x.o" "-L" "/lib" "-lfoo" "-framework" "Metal" '
            '"-o" "/out/app" "-Wl,-dead_strip" "-Wl,-exported_symbols_list,/tmp/list" "-nodefaultlibs"')
    expected = ["-Wl,-dead_strip", "-Wl,-exported_symbols_list,<path>", "-arch arm64"]
    assert options("noise\n" + line + "\n") == expected, options(line)
    print("self-test passed")


def main():
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    first = options(open(sys.argv[1], errors="replace").read())
    if len(sys.argv) == 2:
        print("\n".join(first))
        return
    second = options(open(sys.argv[2], errors="replace").read())
    print("in both:")
    for option in sorted(set(first) & set(second)):
        print(f"  {option}")
    print(f"only in {sys.argv[1]}:")
    for option in sorted(set(first) - set(second)):
        print(f"  {option}")
    print(f"only in {sys.argv[2]}:")
    for option in sorted(set(second) - set(first)):
        print(f"  {option}")


if __name__ == "__main__":
    main()
