#!/usr/bin/env python3
"""Preserve system MSFTEDIT probe observations with their CI/engine provenance.

This extracts the Windows-only probe in rich_text_tom_oracle_tests.rs. It does
not compare the observations with FerroUI or declare WinUI engine conformance.
Unknown probe rows, incomplete runs and contradictory setup are rejected.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


PREFIX = "SYSTEM_MSFTEDIT_"
# The original 250 commands plus a bounded paragraph-TypeText extension.
# Per-fixture limits and exact declared completion counts remain independent.
MAX_SELECTION_GRAVITY_COMMANDS = 512
MAX_RANGE_TRACKING_COMMANDS = 90
DEBUG_STRING = r'"(?:\\.|[^"\\])*"'
STATE = r"SelectionState \{ start: (-?\d+), end: (-?\d+), flags: (\d+) \}"
ENGINE = re.compile(
    r'ENGINE windows=\((\d+), (\d+), (\d+)\) dll=("(?:\\.|[^"\\])*") '
    r"file_version=(\d+\.\d+\.\d+\.\d+)"
)
FIXTURE = re.compile(
    r'(VERTICAL_|HORIZONTAL_|OPTIONS_|LINK_|GRAVITY_)?FIXTURE name=(\S+) text=("(?:\\.|[^"\\])*") '
    r"(?:supplied_)?utf16_length=(\d+) native_story_length=(\d+)"
    r"(?: window_ex_style=(\d+))?(?: expected_commands=(\d+))?"
)
HOME_END = re.compile(
    r"HOME_END fixture=(\S+) supplied_utf16_length=(\d+) native_story_length=(\d+) "
    r"unit=(\w+) extend=(true|false) operation=(Home|End) before="
    + STATE + r" after=" + STATE
    + r" anchor=(-?\d+) active=(-?\d+) delta=(-?\d+) active_displacement=(-?\d+) "
    r"hresult=(0x[0-9A-F]{8})"
)
REPEAT = re.compile(
    r"HOME_END_REPEAT fixture=(\S+) unit=(\w+) extend=(true|false) "
    r"operation=(Home|End) after=" + STATE
    + r" delta=(-?\d+) hresult=(0x[0-9A-F]{8})"
)
VERTICAL = re.compile(
    r"VERTICAL fixture=(\S+) unit=(\w+) count=(-?\d+) extend=(true|false) "
    r"operation=(Up|Down) before=" + STATE + r" after=" + STATE
    + r" anchor=(-?\d+) active=(-?\d+) delta=(-?\d+) active_displacement=(-?\d+) "
    r"hresult=(0x[0-9A-F]{8})"
)
HORIZONTAL = re.compile(
    r"HORIZONTAL fixture=(\S+) unit=(\w+) count=(-?\d+) extend=(true|false) "
    r"operation=(Left|Right) before=" + STATE + r" before_type=(-?\d+) after="
    + STATE + r" after_type=(-?\d+) anchor=(-?\d+) active=(-?\d+) "
    r"delta=(-?\d+) active_displacement=(-?\d+) hresult=(0x[0-9A-F]{8})"
)
OPTIONS = re.compile(
    r"OPTIONS fixture=(\S+) requested_flags=(\d+) before=" + STATE
    + r" before_type=(-?\d+) after_set=" + STATE
    + r" after_set_type=(-?\d+) set_hresult=(0x[0-9A-F]{8}) "
    r'typed=("(?:\\.|[^"\\])*") after=' + STATE
    + r" after_type=(-?\d+) type_hresult=(0x[0-9A-F]{8}) "
    r"native_story_length_after=(\d+) "
    r'story_text=("(?:\\.|[^"\\])*") story_text_utf16_length=(\d+)'
)


def link_snapshot_pattern(prefix: str) -> str:
    return (
        rf"LinkSnapshot \{{ range: RangeState \{{ start: (?P<{prefix}_start>-?\d+), "
        rf"end: (?P<{prefix}_end>-?\d+) \}}, "
        rf"range_text: (?P<{prefix}_range_text>{DEBUG_STRING}), "
        rf"range_text_utf16_length: (?P<{prefix}_range_text_utf16_length>\d+), "
        rf"story_text: (?P<{prefix}_story_text>{DEBUG_STRING}), "
        rf"story_text_utf16_length: (?P<{prefix}_story_text_utf16_length>\d+), "
        rf"story_length: (?P<{prefix}_story_length>\d+), "
        rf"selection: SelectionState \{{ start: (?P<{prefix}_selection_start>-?\d+), "
        rf"end: (?P<{prefix}_selection_end>-?\d+), flags: (?P<{prefix}_flags>\d+) \}} \}}"
    )


LINK = re.compile(
    r"LINK fixture=(?P<fixture>\S+) step=(?P<step>\d+) "
    r"role=(?P<role>Setup|Target|Readback) operation=(?P<operation>Get|Set) "
    r"initial_start=(?P<initial_start>-?\d+) initial_end=(?P<initial_end>-?\d+) "
    r"argument_null=(?P<argument_null>true|false) "
    rf"argument=(?P<argument>{DEBUG_STRING}) argument_utf16_length=(?P<argument_length>\d+) "
    r"before=" + link_snapshot_pattern("before") + r" after=" + link_snapshot_pattern("after")
    + r" hresult=(?P<hresult>0x[0-9A-F]{8}) result_null=(?P<result_null>true|false) "
    rf"result=(?P<result>{DEBUG_STRING}) result_utf16_length=(?P<result_length>\d+)"
)


def native_number_pattern(prefix: str) -> str:
    return (
        rf"NativeNumber \{{ hresult: (?P<{prefix}_hresult>0x[0-9A-F]{{8}}), "
        rf"value: (?P<{prefix}_value>-?\d+) \}}"
    )


def gravity_range_pattern(prefix: str) -> str:
    result = (
        rf"GravityRangeState \{{ range: RangeState \{{ start: (?P<{prefix}_start>-?\d+), "
        rf"end: (?P<{prefix}_end>-?\d+) \}}, "
    )
    for name in ("gravity", "flags", "bold", "italic", "alignment"):
        result += name + ": " + native_number_pattern(prefix + "_" + name) + ", "
    return result + (
        rf"text: (?P<{prefix}_text>{DEBUG_STRING}), "
        rf"text_utf16_length: (?P<{prefix}_text_utf16_length>\d+) \}}"
    )


def gravity_snapshot_pattern(prefix: str) -> str:
    return (
        r"GravitySnapshot \{ primary: " + gravity_range_pattern(prefix + "_primary")
        + rf", duplicate: (?:(?P<{prefix}_duplicate_none>None)|Some\("
        + gravity_range_pattern(prefix + "_duplicate") + r"\)), "
        + rf"story_text: (?P<{prefix}_story_text>{DEBUG_STRING}), "
        rf"story_text_utf16_length: (?P<{prefix}_story_text_utf16_length>\d+), "
        rf"story_length: (?P<{prefix}_story_length>\d+), "
        rf"selection: SelectionState \{{ start: (?P<{prefix}_selection_start>-?\d+), "
        rf"end: (?P<{prefix}_selection_end>-?\d+), flags: (?P<{prefix}_flags>\d+) \}} \}}"
    )


GRAVITY = re.compile(
    r"GRAVITY fixture=(?P<fixture>\S+) step=(?P<step>\d+) role=(?P<role>Setup|Target) "
    r"target=(?P<target>Primary|Duplicate|External) "
    r"operation=(?P<operation>GetGravity|SetGravity|SetRange|SetStart|SetEnd|Collapse|Duplicate|SetText|SetBold|SetItalic|SetAlignment) "
    r"argument1=(?P<argument1>-?\d+) argument2=(?P<argument2>-?\d+) value=(?P<value>-?\d+) "
    rf"text=(?P<text>{DEBUG_STRING}) text_utf16_length=(?P<text_utf16_length>\d+) "
    r"action_before=RangeState \{ start: (?P<action_before_start>-?\d+), end: (?P<action_before_end>-?\d+) \} "
    r"action_after=RangeState \{ start: (?P<action_after_start>-?\d+), end: (?P<action_after_end>-?\d+) \} "
    r"before=" + gravity_snapshot_pattern("before") + r" after=" + gravity_snapshot_pattern("after")
    + r" hresult=(?P<hresult>0x[0-9A-F]{8}) output=(?:None|Some\((?P<output>-?\d+)\))"
)


# Stage A has its own identity and completion contract while retaining the
# exact previously reviewed Gravity snapshot/command representation.
RANGE_TRACKING = re.compile(GRAVITY.pattern.replace("GRAVITY fixture=", "RANGE_TRACKING fixture=", 1))
RANGE_TRACKING_FIXTURE = re.compile(
    rf'RANGE_TRACKING_FIXTURE name=(\S+) text=({DEBUG_STRING}) '
    r"supplied_utf16_length=(\d+) native_story_length=(\d+) "
    r"window_ex_style=(\d+) expected_commands=(\d+)"
)


def rust_string(encoded: str) -> str:
    """Decode a Rust Debug string, including non-printing Unicode scalars.

    Rust writes a ZWJ as a braced Unicode escape rather than JSON's fixed-width
    form. Walk escapes in order so a literal backslash followed by 'u' stays
    literal, and reject malformed/unknown escapes rather than losing text.
    """
    if not (encoded.startswith('"') and encoded.endswith('"')):
        raise ValueError("fixture string is not quoted")
    escapes = {'"': '"', "\\": "\\", "n": "\n", "r": "\r", "t": "\t", "0": "\0"}
    result = []
    index = 1
    end = len(encoded) - 1
    while index < end:
        char = encoded[index]
        index += 1
        if char != "\\":
            if char == '"' or ord(char) < 0x20 or 0xD800 <= ord(char) <= 0xDFFF:
                raise ValueError("invalid literal in fixture string")
            result.append(char)
            continue
        if index >= end:
            raise ValueError("unterminated fixture escape")
        escape = encoded[index]
        index += 1
        if escape in escapes:
            result.append(escapes[escape])
        elif escape == "u" and index < end and encoded[index] == "{":
            closing = encoded.find("}", index + 1, end)
            digits = encoded[index + 1:closing] if closing >= 0 else ""
            if not re.fullmatch(r"[0-9a-fA-F]{1,6}", digits):
                raise ValueError("invalid fixture Unicode escape")
            value = int(digits, 16)
            if value > 0x10FFFF or 0xD800 <= value <= 0xDFFF:
                raise ValueError("fixture escape is not a Unicode scalar")
            result.append(chr(value))
            index = closing + 1
        else:
            raise ValueError("unsupported fixture string escape")
    return "".join(result)


def state(values: tuple[str, ...]) -> dict:
    start, end, flags = map(int, values)
    if not 0 <= start <= end:
        raise ValueError("invalid native normalized range")
    return {
        "start": start,
        "end": end,
        "flags": flags,
        "anchor": end if flags & 1 else start,
        "active": start if flags & 1 else end,
    }


def movement(values: tuple[str, ...], state_start: int) -> dict:
    before = state(values[state_start:state_start + 3])
    after = state(values[state_start + 3:state_start + 6])
    anchor, active, delta, displacement = map(int, values[state_start + 6:state_start + 10])
    if (anchor, active) != (after["anchor"], after["active"]):
        raise ValueError("logged directed endpoints disagree with native flags")
    if displacement != after["active"] - before["active"]:
        raise ValueError("logged displacement disagrees with native endpoints")
    return {
        "before": before,
        "after": after,
        "delta": delta,
        "active_displacement": displacement,
        "hresult": values[state_start + 10],
    }


def link_snapshot(values: dict[str, str], prefix: str) -> dict:
    def field(name: str) -> str:
        return values[prefix + "_" + name]

    start, end, story_length = (int(field(name)) for name in ("start", "end", "story_length"))
    if not 0 <= start <= end <= story_length:
        raise ValueError("Link range exceeds its native story")
    selected = state(tuple(field(name) for name in ("selection_start", "selection_end", "flags")))
    if selected["end"] > story_length:
        raise ValueError("Link snapshot selection exceeds its native story")
    result = {"range": {"start": start, "end": end}}
    for name in ("range_text", "story_text"):
        text = rust_string(field(name))
        length = int(field(name + "_utf16_length"))
        if len(text.encode("utf-16-le")) // 2 != length:
            raise ValueError("Link text disagrees with its explicit UTF-16 length")
        result[name] = text
        result[name + "_utf16_length"] = length
    if result["story_text_utf16_length"] > story_length:
        raise ValueError("Link story text exceeds its native story length")
    return {**result, "story_length": story_length, "selection": selected}


def link_command(match: re.Match) -> dict:
    values = match.groupdict()
    result = {
        "fixture": values["fixture"],
        "step": int(values["step"]),
        "role": values["role"],
        "operation": values["operation"],
        "initial_range": {"start": int(values["initial_start"]), "end": int(values["initial_end"])},
    }
    for name in ("argument", "result"):
        text = rust_string(values[name])
        length = int(values[name + "_length"])
        is_null = values[name + "_null"] == "true"
        if len(text.encode("utf-16-le")) // 2 != length or (is_null and text):
            raise ValueError("Link BSTR disagrees with its null/UTF-16 metadata")
        result[name] = {"is_null": is_null, "text": text, "utf16_length": length}
    if result["operation"] == "Get" and not result["argument"]["is_null"]:
        raise ValueError("GetURL has no input BSTR")
    if result["operation"] == "Set" and not result["result"]["is_null"]:
        raise ValueError("SetURL has no output BSTR")
    result["before"] = link_snapshot(values, "before")
    result["after"] = link_snapshot(values, "after")
    result["hresult"] = values["hresult"]
    return result


def validate_link_steps(fixture: dict, rows: list[dict]) -> None:
    if [row["step"] for row in rows] != list(range(len(rows))):
        raise ValueError("Link steps must be unique and consecutive from zero")
    roles = [row["role"] for row in rows]
    if roles.count("Target") != 1:
        raise ValueError("Link fixture needs exactly one target command")
    target = roles.index("Target")
    expected = ["Setup"] * target + ["Target"]
    if rows[target]["operation"] == "Set":
        expected.append("Readback")
    if roles != expected:
        raise ValueError("Link setup/target/readback order is incomplete")
    for index, row in enumerate(rows):
        initial = row["initial_range"]
        if not 0 <= initial["start"] <= initial["end"] <= fixture["native_story_length"]:
            raise ValueError("Link initial range exceeds its exact fixture")
        if row["role"] == "Setup":
            if row["operation"] != "Set" or row["argument"]["is_null"] or int(row["hresult"], 16) >= 0x80000000:
                raise ValueError("Link setup must be a successful SetURL")
        if index == 0:
            if row["before"]["range"] != initial or row["before"]["story_length"] != fixture["native_story_length"]:
                raise ValueError("first Link command disagrees with its initial fixture")
            if not row["before"]["story_text"].startswith(fixture["text"]):
                raise ValueError("initial Link story does not contain its exact fixture text")
        else:
            previous = rows[index - 1]
            for key in ("story_text", "story_text_utf16_length", "story_length", "selection"):
                if previous["after"][key] != row["before"][key]:
                    raise ValueError("Link story/selection changed between consecutive commands")
        if row["role"] == "Readback":
            previous = rows[index - 1]
            if row["operation"] != "Get" or initial != previous["initial_range"] or row["before"] != previous["after"]:
                raise ValueError("Link readback must observe the same target range")


def signed_long(value: str) -> int:
    result = int(value)
    if not -(2 ** 31) <= result < 2 ** 31:
        raise ValueError("gravity value does not fit the native signed LONG ABI")
    return result


def gravity_range(values: dict[str, str], prefix: str, story_length: int) -> dict:
    start, end = (signed_long(values[prefix + "_" + name]) for name in ("start", "end"))
    if not 0 <= start <= end <= story_length:
        raise ValueError("gravity range exceeds its exact native story")
    result = {"range": {"start": start, "end": end}}
    for name in ("gravity", "flags", "bold", "italic", "alignment"):
        result[name] = {
            "hresult": values[prefix + "_" + name + "_hresult"],
            "value": signed_long(values[prefix + "_" + name + "_value"]),
        }
    text = rust_string(values[prefix + "_text"])
    length = int(values[prefix + "_text_utf16_length"])
    if len(text.encode("utf-16-le")) // 2 != length:
        raise ValueError("gravity range text disagrees with its UTF-16 length")
    return {**result, "text": text, "text_utf16_length": length}


def gravity_snapshot(values: dict[str, str], prefix: str) -> dict:
    story_length = signed_long(values[prefix + "_story_length"])
    text = rust_string(values[prefix + "_story_text"])
    length = int(values[prefix + "_story_text_utf16_length"])
    if len(text.encode("utf-16-le")) // 2 != length or length > story_length:
        raise ValueError("gravity story text disagrees with its UTF-16/native lengths")
    selected = state(tuple(values[prefix + "_" + name] for name in ("selection_start", "selection_end", "flags")))
    if selected["end"] > story_length or selected["flags"] >= 2 ** 31:
        raise ValueError("gravity selection exceeds its native story or signed flags ABI")
    return {
        "primary": gravity_range(values, prefix + "_primary", story_length),
        "duplicate": None if values[prefix + "_duplicate_none"] else gravity_range(values, prefix + "_duplicate", story_length),
        "story_text": text,
        "story_text_utf16_length": length,
        "story_length": story_length,
        "selection": selected,
    }


def gravity_command(match: re.Match) -> dict:
    values = match.groupdict()
    result = {name: values[name] for name in ("fixture", "role", "target", "operation", "hresult")}
    result["step"] = int(values["step"])
    for name in ("argument1", "argument2", "value"):
        result[name] = signed_long(values[name])
    result["text"] = rust_string(values["text"])
    result["text_utf16_length"] = int(values["text_utf16_length"])
    if len(result["text"].encode("utf-16-le")) // 2 != result["text_utf16_length"]:
        raise ValueError("gravity command text disagrees with its UTF-16 length")
    result["output"] = None if values["output"] is None else signed_long(values["output"])
    for phase in ("before", "after"):
        result[phase] = gravity_snapshot(values, phase)
        action = {name: signed_long(values["action_" + phase + "_" + name]) for name in ("start", "end")}
        if not 0 <= action["start"] <= action["end"] <= result[phase]["story_length"]:
            raise ValueError("gravity action range exceeds its exact story")
        result["action_" + phase] = action
        if result["target"] != "External":
            target = result[phase][result["target"].lower()]
            if target is None or target["range"] != action:
                raise ValueError("gravity action range disagrees with its tracked target")
    operation, target = result["operation"], result["target"]
    a, b, value = (result[name] for name in ("argument1", "argument2", "value"))
    if (operation == "GetGravity") != (result["output"] is not None):
        raise ValueError("only GetGravity has an integer output")
    if operation != "SetText" and result["text"]:
        raise ValueError("only gravity SetText has text input")
    if target == "External":
        if operation not in ("SetText", "SetBold", "SetItalic", "SetAlignment"):
            raise ValueError("unexpected operation on an external gravity edit range")
        if result["action_before"] != {"start": a, "end": b}:
            raise ValueError("external gravity range disagrees with its exact creation arguments")
    elif operation not in ("SetRange", "SetStart", "SetEnd") and (a or b):
        raise ValueError("unexpected range arguments to gravity command")
    elif (operation == "SetStart" and b) or (operation == "SetEnd" and a):
        raise ValueError("unexpected second endpoint argument")
    if operation not in ("SetGravity", "Collapse", "SetBold", "SetItalic", "SetAlignment") and value:
        raise ValueError("unexpected integer value argument to gravity command")
    if operation == "Collapse" and value not in (-1, 0):
        raise ValueError("gravity probe Collapse requires tomTrue or tomFalse")
    if operation in ("SetBold", "SetItalic", "SetAlignment"):
        allowed = (-1, 0) if operation != "SetAlignment" else (0, 2)
        if target != "External" or result["role"] != "Setup" or value not in allowed:
            raise ValueError("gravity format commands must be declared external setup")
    if result["role"] == "Setup":
        if operation not in ("SetBold", "SetItalic", "SetAlignment") or int(result["hresult"], 16) >= 0x80000000:
            raise ValueError("gravity fixture setup must successfully establish formatting")
    if operation == "Duplicate":
        if target != "Primary" or result["before"]["duplicate"] is not None:
            raise ValueError("gravity duplicate must be created once from its primary")
        success = int(result["hresult"], 16) < 0x80000000
        if success != (result["after"]["duplicate"] is not None):
            raise ValueError("gravity duplicate result disagrees with its HRESULT")
    elif (result["before"]["duplicate"] is None) != (result["after"]["duplicate"] is None):
        raise ValueError("gravity command unexpectedly creates or destroys its duplicate")
    return result


def validate_gravity_steps(fixture: dict, rows: list[dict]) -> None:
    if len(rows) != fixture["expected_commands"] or [row["step"] for row in rows] != list(range(len(rows))):
        raise ValueError("gravity steps do not match their declared consecutive command count")
    targets = [index for index, row in enumerate(rows) if row["role"] == "Target"]
    if len(targets) < 2 or targets != list(range(targets[0], len(rows))):
        raise ValueError("gravity setup must precede the complete target sequence")
    prefix = rows[targets[0]:targets[0] + 2]
    if [(row["target"], row["operation"]) for row in prefix] != [("Primary", "SetRange"), ("Primary", "SetGravity")]:
        raise ValueError("gravity target sequence must establish its directed range and gravity")
    if rows[-1]["operation"] != "GetGravity":
        raise ValueError("gravity fixture must finish with an explicit getter readback")
    first = rows[0]["before"]
    if first["primary"]["range"] != {"start": 0, "end": 0} or first["duplicate"] is not None:
        raise ValueError("gravity fixture must begin with a fresh independent caret range")
    if first["story_length"] != fixture["native_story_length"] or not first["story_text"].startswith(fixture["text"]):
        raise ValueError("gravity initial story disagrees with its exact fixture")
    for previous, current in zip(rows, rows[1:]):
        if previous["after"] != current["before"]:
            raise ValueError("gravity state changed between consecutive observed commands")


def range_tracking_command(match: re.Match) -> dict:
    result = gravity_command(match)
    for phase in ("before", "after"):
        for target in ("primary", "duplicate"):
            state = result[phase][target]
            if state is not None and state["text_utf16_length"] > state["range"]["end"] - state["range"]["start"]:
                raise ValueError("range tracking text exceeds its observed endpoint extent")
    return result


def validate_range_tracking_steps(fixture: dict, rows: list[dict]) -> None:
    validate_gravity_steps(fixture, rows)
    expected = [
        ("Target", "Primary", "SetRange"),
        ("Target", "Primary", "SetGravity"),
        ("Target", "Primary", "Duplicate"),
        ("Target", "External", "SetText"),
        ("Target", "Primary", "GetGravity"),
        ("Target", "Duplicate", "GetGravity"),
    ]
    if [(row["role"], row["target"], row["operation"]) for row in rows] != expected:
        raise ValueError("range tracking requires its exact six-command target sequence")
    if rows[1]["value"] not in (0, 2):
        raise ValueError("range tracking requests only UIBehavior or Forward")


def native_interface_pattern(prefix: str) -> str:
    return (
        rf"NativeInterface \{{ hresult: (?P<{prefix}_hresult>0x[0-9A-F]{{8}}), "
        rf"is_null: (?P<{prefix}_is_null>true|false), identity: "
        rf"(?:(?P<{prefix}_identity_none>None)|Some\(NativeIdentity \{{ "
        rf"hresult: (?P<{prefix}_identity_hresult>0x[0-9A-F]{{8}}), "
        rf"is_null: (?P<{prefix}_identity_is_null>true|false), "
        rf"same_identity: (?P<{prefix}_same_identity>true|false) \}}\)) \}}"
    )


def native_text_pattern(prefix: str) -> str:
    return (
        rf"NativeText \{{ hresult: (?P<{prefix}_hresult>0x[0-9A-F]{{8}}), "
        rf"is_null: (?P<{prefix}_is_null>true|false), text: (?P<{prefix}_text>{DEBUG_STRING}), "
        rf"text_utf16_length: (?P<{prefix}_text_utf16_length>\d+) \}}"
    )


def selection_gravity_range_pattern(prefix: str) -> str:
    result = (
        rf"SelectionGravityRangeState \{{ range: RangeState \{{ start: (?P<{prefix}_start>-?\d+), "
        rf"end: (?P<{prefix}_end>-?\d+) \}}, "
    )
    for name in ("gravity", "flags", "selection_type", "bold", "italic", "alignment"):
        result += name + ": " + native_number_pattern(prefix + "_" + name) + ", "
    return result + "text: " + native_text_pattern(prefix + "_text") + r" \}"


def selection_gravity_snapshot_pattern(prefix: str) -> str:
    return (
        r"SelectionGravitySnapshot \{ primary: " + selection_gravity_range_pattern(prefix + "_primary")
        + rf", duplicate: (?:(?P<{prefix}_duplicate_none>None)|Some\("
        + selection_gravity_range_pattern(prefix + "_duplicate") + r"\)), story_text: "
        + native_text_pattern(prefix + "_story_text") + ", story_length: "
        + native_number_pattern(prefix + "_story_length")
        + rf", selection: SelectionState \{{ start: (?P<{prefix}_selection_start>-?\d+), "
        rf"end: (?P<{prefix}_selection_end>-?\d+), flags: (?P<{prefix}_selection_flags>\d+) \}} \}}"
    )


SELECTION_GRAVITY_FIXTURE = re.compile(
    rf"SELECTION_GRAVITY_FIXTURE name=(\S+) text=({DEBUG_STRING}) supplied_utf16_length=(\d+) "
    r"native_story_length=(\d+) window_ex_style=(\d+) expected_commands=(\d+) initialization=(Configured|Default)"
)
SELECTION_GRAVITY_INTERFACE = re.compile(
    r"SELECTION_GRAVITY_INTERFACE fixture=(?P<fixture>\S+) source_hresult=(?P<source_hresult>0x[0-9A-F]{8}) "
    r"source_null=(?P<source_null>true|false) range2=" + native_interface_pattern("range2")
    + " selection2=" + native_interface_pattern("selection2")
)
SELECTION_GRAVITY = re.compile(
    r"SELECTION_GRAVITY fixture=(?P<fixture>\S+) step=(?P<step>\d+) role=(?P<role>Setup|Target) "
    r"target=(?P<target>Primary|Duplicate|External) "
    r"operation=(?P<operation>GetGravity|SetGravity|SetRange|Duplicate|TypeText|MoveLeft|MoveRight|MoveUp|MoveDown|EndKey|SetBold|SetItalic|SetAlignment) "
    r"argument1=(?P<argument1>-?\d+) argument2=(?P<argument2>-?\d+) value=(?P<value>-?\d+) "
    r"extend=(?P<extend>true|false) text_null=(?P<text_null>true|false) "
    rf"text=(?P<text>{DEBUG_STRING}) text_utf16_length=(?P<text_utf16_length>\d+) "
    r"action_before=" + selection_gravity_range_pattern("action_before")
    + r" action_after=" + selection_gravity_range_pattern("action_after")
    + r" before=" + selection_gravity_snapshot_pattern("before")
    + r" after=" + selection_gravity_snapshot_pattern("after")
    + r" hresult=(?P<hresult>0x[0-9A-F]{8}) output=(?:None|Some\((?P<output>-?\d+)\)) "
    r"returned_null=(?:None|Some\((?P<returned_null>true|false)\)) "
    r"duplicate_interface=(?:(?P<duplicate_interface_none>None)|Some\("
    + native_interface_pattern("duplicate_interface") + r"\))"
)


def hresult_succeeded(value: str) -> bool:
    return int(value, 16) < 0x80000000


def native_interface(values: dict[str, str], prefix: str) -> dict:
    result = {"hresult": values[prefix + "_hresult"], "is_null": values[prefix + "_is_null"] == "true", "identity": None}
    if not values[prefix + "_identity_none"]:
        result["identity"] = {
            "hresult": values[prefix + "_identity_hresult"],
            "is_null": values[prefix + "_identity_is_null"] == "true",
            "same_identity": values[prefix + "_same_identity"] == "true",
        }
    if (hresult_succeeded(result["hresult"]) and not result["is_null"]) != (result["identity"] is not None):
        raise ValueError("selection interface identity query contradicts its HRESULT/null result")
    identity = result["identity"]
    if identity and identity["same_identity"] and (identity["is_null"] or not hresult_succeeded(identity["hresult"])):
        raise ValueError("a failed/null canonical identity cannot prove selection identity")
    return result


def selection_gravity_interface(match: re.Match) -> dict:
    values = match.groupdict()
    result = {
        "fixture": values["fixture"], "source_hresult": values["source_hresult"],
        "source_null": values["source_null"] == "true",
        "range2": native_interface(values, "range2"), "selection2": native_interface(values, "selection2"),
    }
    if result["source_null"] or not hresult_succeeded(result["source_hresult"]):
        raise ValueError("selection probe has no source canonical IUnknown")
    for name in ("range2", "selection2"):
        interface = result[name]
        if not hresult_succeeded(interface["hresult"]) or interface["is_null"] or not interface["identity"] or not interface["identity"]["same_identity"]:
            raise ValueError("extended interface has not established the actual selection identity")
    return result


def native_text(values: dict[str, str], prefix: str) -> dict:
    text = rust_string(values[prefix + "_text"])
    length = int(values[prefix + "_text_utf16_length"])
    is_null = values[prefix + "_is_null"] == "true"
    if len(text.encode("utf-16-le")) // 2 != length or (is_null and text):
        raise ValueError("selection gravity BSTR null/text/UTF-16 length disagree")
    return {"hresult": values[prefix + "_hresult"], "is_null": is_null, "text": text, "text_utf16_length": length}


def selection_gravity_range(values: dict[str, str], prefix: str, story_length: int) -> dict:
    start, end = (signed_long(values[prefix + "_" + name]) for name in ("start", "end"))
    if not 0 <= start <= end <= story_length:
        raise ValueError("selection gravity range exceeds its native story")
    result = {"range": {"start": start, "end": end}}
    for name in ("gravity", "flags", "selection_type", "bold", "italic", "alignment"):
        result[name] = {"hresult": values[prefix + "_" + name + "_hresult"], "value": signed_long(values[prefix + "_" + name + "_value"])}
    result["text"] = native_text(values, prefix + "_text")
    if hresult_succeeded(result["text"]["hresult"]) and result["text"]["text_utf16_length"] > end - start:
        raise ValueError("selection gravity text exceeds its observed range extent")
    return result


def selection_gravity_snapshot(values: dict[str, str], prefix: str) -> dict:
    story_length = {
        "hresult": values[prefix + "_story_length_hresult"],
        "value": signed_long(values[prefix + "_story_length_value"]),
    }
    if not hresult_succeeded(story_length["hresult"]) or story_length["value"] < 0:
        raise ValueError("selection probe lacks a valid story length for snapshot capture")
    length = story_length["value"]
    story_text = native_text(values, prefix + "_story_text")
    if hresult_succeeded(story_text["hresult"]) and story_text["text_utf16_length"] > length:
        raise ValueError("selection gravity story text exceeds its native length")
    selected = state(tuple(values[prefix + "_selection_" + name] for name in ("start", "end", "flags")))
    primary = selection_gravity_range(values, prefix + "_primary", length)
    if selected["end"] > length or selected["flags"] >= 2 ** 31:
        raise ValueError("selection gravity actual selection exceeds native bounds")
    if primary["range"] != {"start": selected["start"], "end": selected["end"]} or not hresult_succeeded(primary["flags"]["hresult"]) or primary["flags"]["value"] != selected["flags"]:
        raise ValueError("extended selection snapshot disagrees with the actual selection")
    return {
        "primary": primary,
        "duplicate": None if values[prefix + "_duplicate_none"] else selection_gravity_range(values, prefix + "_duplicate", length),
        "story_text": story_text, "story_length": story_length, "selection": selected,
    }


def selection_gravity_command(match: re.Match) -> dict:
    values = match.groupdict()
    result = {name: values[name] for name in ("fixture", "role", "target", "operation", "hresult")}
    result["step"] = int(values["step"])
    for name in ("argument1", "argument2", "value"):
        result[name] = signed_long(values[name])
    for name in ("extend", "text_null"):
        result[name] = values[name] == "true"
    result["text"] = rust_string(values["text"])
    result["text_utf16_length"] = int(values["text_utf16_length"])
    if len(result["text"].encode("utf-16-le")) // 2 != result["text_utf16_length"] or (result["text_null"] and result["text"]):
        raise ValueError("selection gravity input BSTR null/text/UTF-16 length disagree")
    result["output"] = None if values["output"] is None else signed_long(values["output"])
    result["returned_null"] = None if values["returned_null"] is None else values["returned_null"] == "true"
    result["duplicate_interface"] = None if values["duplicate_interface_none"] else native_interface(values, "duplicate_interface")
    for phase in ("before", "after"):
        result[phase] = selection_gravity_snapshot(values, phase)
        action = selection_gravity_range(values, "action_" + phase, result[phase]["story_length"]["value"])
        result["action_" + phase] = action
        if result["target"] != "External" and result[phase][result["target"].lower()] != action:
            raise ValueError("selection gravity action state disagrees with its tracked target")
    operation, target = result["operation"], result["target"]
    a, b, value = (result[name] for name in ("argument1", "argument2", "value"))
    movements = {"MoveLeft", "MoveRight", "MoveUp", "MoveDown", "EndKey"}
    if (operation in movements | {"GetGravity"}) != (result["output"] is not None):
        raise ValueError("selection gravity getter/movement output presence is incorrect")
    if operation != "TypeText" and (not result["text_null"] or result["text"]):
        raise ValueError("only selection TypeText has a BSTR argument")
    if operation not in movements and result["extend"]:
        raise ValueError("only selection movement has an extend argument")
    if target == "External":
        if operation not in ("GetGravity", "SetBold", "SetItalic", "SetAlignment") or result["action_before"]["range"] != {"start": a, "end": b}:
            raise ValueError("selection gravity external inspection/setup arguments disagree")
    elif operation not in movements | {"SetRange"} and (a or b):
        raise ValueError("selection gravity command has unexpected endpoint arguments")
    if operation in movements:
        if target != "Primary" or value or a != (1 if operation in ("MoveLeft", "MoveRight") else 5) or (operation == "EndKey" and b):
            raise ValueError("selection gravity movement signature/target disagrees with probe")
    if operation == "TypeText" and target != "Primary":
        raise ValueError("TypeText must act on the actual selection")
    if operation not in ("SetGravity", "SetBold", "SetItalic", "SetAlignment") and value:
        raise ValueError("selection gravity operation has an unexpected value argument")
    formats = {"SetBold", "SetItalic", "SetAlignment"}
    if operation in formats:
        if target != "External" or result["role"] != "Setup" or value not in ((0, 2) if operation == "SetAlignment" else (-1, 0)):
            raise ValueError("selection gravity formatting must be explicit external setup")
    if result["role"] == "Setup" and (operation not in formats or not hresult_succeeded(result["hresult"])):
        raise ValueError("selection gravity setup did not establish its declared formatting")
    if operation == "Duplicate":
        if target != "Primary" or result["before"]["duplicate"] is not None or result["returned_null"] is None:
            raise ValueError("selection gravity duplicate must record one actual selection clone result")
        queried = hresult_succeeded(result["hresult"]) and not result["returned_null"]
        interface = result["duplicate_interface"]
        if queried != (interface is not None):
            raise ValueError("selection duplicate interface query disagrees with creation HRESULT/null")
        available = queried and hresult_succeeded(interface["hresult"]) and not interface["is_null"]
        if available != (result["after"]["duplicate"] is not None):
            raise ValueError("selection duplicate lifetime disagrees with its interface result")
    elif result["returned_null"] is not None or result["duplicate_interface"] is not None or ((result["before"]["duplicate"] is None) != (result["after"]["duplicate"] is None)):
        raise ValueError("selection gravity command unexpectedly changes duplicate lifetime")
    return result


def validate_selection_gravity_steps(fixture: dict, rows: list[dict]) -> None:
    if len(rows) != fixture["expected_commands"] or [row["step"] for row in rows] != list(range(len(rows))):
        raise ValueError("selection gravity steps disagree with their consecutive command count")
    targets = [index for index, row in enumerate(rows) if row["role"] == "Target"]
    if not targets or targets != list(range(targets[0], len(rows))):
        raise ValueError("selection gravity setup must precede its target sequence")
    if fixture["initialization"] == "Configured":
        prefix = rows[targets[0]:targets[0] + 2]
        if [(row["target"], row["operation"]) for row in prefix] != [("Primary", "SetRange"), ("Primary", "SetGravity")]:
            raise ValueError("configured selection gravity needs its exact range/mode prefix")
        if prefix[1]["value"] not in range(5):
            raise ValueError("configured selection gravity must start with a declared enum mode")
    elif fixture["text"] or len(rows) != 2 or targets != [0, 1] or [(row["target"], row["operation"]) for row in rows] != [("Primary", "TypeText"), ("Primary", "GetGravity")] or rows[0]["text_null"] or rows[0]["text"] != "X":
        raise ValueError("default selection gravity fixture must preserve untouched empty-story initialization")
    if rows[-1]["operation"] != "GetGravity" or rows[-1]["target"] != "Primary":
        raise ValueError("selection gravity fixture must finish by reading the actual selection gravity")
    first = rows[0]["before"]
    if first["duplicate"] is not None or first["story_length"]["value"] != fixture["native_story_length"] or not first["story_text"]["text"].startswith(fixture["text"]):
        raise ValueError("selection gravity initial story or duplicate disagrees with its fresh fixture")
    for previous, current in zip(rows, rows[1:]):
        if previous["after"] != current["before"]:
            raise ValueError("selection gravity state changed between consecutive commands")


def extract(log: str) -> dict:
    engine = None
    fixtures = []
    home_end = []
    vertical = []
    horizontal = []
    options = []
    link = []
    gravity = []
    range_tracking = []
    selection_gravity = []
    selection_gravity_interfaces = []
    completed = {}
    for line_number, line in enumerate(log.splitlines(), 1):
        marker = line.find(PREFIX)
        if marker < 0:
            continue
        row = line[marker + len(PREFIX):].rstrip()
        match = ENGINE.fullmatch(row)
        if match:
            current = {
                "windows_version": list(map(int, match.group(1, 2, 3))),
                "dll_path": json.loads(match[4]),
                "dll_file_version": match[5],
            }
            if engine is not None and engine != current:
                raise ValueError("engine changed within one probe record")
            engine = current
            continue
        match = RANGE_TRACKING_FIXTURE.fullmatch(row)
        if match:
            if "range_tracking" in completed:
                raise ValueError("range tracking fixture follows completion")
            text = rust_string(match[2])
            supplied, story, style, commands = map(int, match.group(3, 4, 5, 6))
            if len(text.encode("utf-16-le")) // 2 != supplied or not supplied <= story < 2 ** 31 or style > 0xFFFFFFFF or commands != 6:
                raise ValueError("range tracking fixture has inconsistent native lengths/style/six-command count")
            fixtures.append({"family": "range_tracking", "name": match[1], "text": text,
                             "supplied_utf16_length": supplied, "native_story_length": story,
                             "window_ex_style": style, "expected_commands": commands})
            continue
        match = RANGE_TRACKING.fullmatch(row)
        if match:
            if "range_tracking" in completed:
                raise ValueError("range tracking command follows completion")
            if not any(fixture["family"] == "range_tracking" and fixture["name"] == match["fixture"] for fixture in fixtures):
                raise ValueError("range tracking command precedes its exact fixture")
            range_tracking.append(range_tracking_command(match))
            continue
        match = re.fullmatch(r"RANGE_TRACKING_PROBE_COMPLETE commands=(\d+) fixtures=(\d+)", row)
        if match:
            if "range_tracking" in completed:
                raise ValueError("duplicate range tracking completion")
            completed["range_tracking"] = (int(match[1]), int(match[2]))
            continue
        match = SELECTION_GRAVITY_FIXTURE.fullmatch(row)
        if match:
            if "selection_gravity" in completed:
                raise ValueError("selection gravity fixture follows completion")
            text = rust_string(match[2])
            supplied, story, style, commands = map(int, match.group(3, 4, 5, 6))
            if len(text.encode("utf-16-le")) // 2 != supplied or not supplied <= story < 2 ** 31 or style > 0xFFFFFFFF or not 2 <= commands <= 250:
                raise ValueError("selection gravity fixture has inconsistent native lengths/style/count")
            fixtures.append({"family": "selection_gravity", "name": match[1], "text": text,
                             "supplied_utf16_length": supplied, "native_story_length": story,
                             "window_ex_style": style, "expected_commands": commands, "initialization": match[7]})
            continue
        match = SELECTION_GRAVITY_INTERFACE.fullmatch(row)
        if match:
            if "selection_gravity" in completed or any(value["fixture"] == match["fixture"] for value in selection_gravity_interfaces):
                raise ValueError("duplicate or late selection interface identity")
            if not any(fixture["family"] == "selection_gravity" and fixture["name"] == match["fixture"] for fixture in fixtures):
                raise ValueError("selection interface identity has no preceding fixture")
            selection_gravity_interfaces.append(selection_gravity_interface(match))
            continue
        match = SELECTION_GRAVITY.fullmatch(row)
        if match:
            if "selection_gravity" in completed:
                raise ValueError("selection gravity command follows completion")
            if not any(value["fixture"] == match["fixture"] for value in selection_gravity_interfaces):
                raise ValueError("selection gravity command precedes its actual-selection identity proof")
            selection_gravity.append(selection_gravity_command(match))
            continue
        match = re.fullmatch(r"SELECTION_GRAVITY_PROBE_COMPLETE commands=(\d+) fixtures=(\d+)", row)
        if match:
            if "selection_gravity" in completed:
                raise ValueError("duplicate selection gravity completion")
            completed["selection_gravity"] = (int(match[1]), int(match[2]))
            continue
        match = FIXTURE.fullmatch(row)
        if match:
            text = rust_string(match[3])
            if len(text.encode("utf-16-le")) // 2 != int(match[4]):
                raise ValueError("fixture text disagrees with supplied UTF-16 length")
            fixture = {
                "family": {
                    None: "home_end",
                    "VERTICAL_": "vertical",
                    "HORIZONTAL_": "horizontal",
                    "OPTIONS_": "options",
                    "LINK_": "link",
                    "GRAVITY_": "gravity",
                }[match[1]],
                "name": match[2],
                "text": text,
                "supplied_utf16_length": int(match[4]),
                "native_story_length": int(match[5]),
            }
            if fixture["native_story_length"] < fixture["supplied_utf16_length"]:
                raise ValueError("native story is shorter than its supplied text")
            if fixture["family"] in ("horizontal", "options", "link", "gravity"):
                if match[6] is None or int(match[6]) > 0xFFFFFFFF:
                    raise ValueError("horizontal/options/Link/gravity fixture needs its native u32 window style")
                fixture["window_ex_style"] = int(match[6])
            elif match[6] is not None:
                raise ValueError("unexpected window style outside horizontal/options/Link/gravity fixture")
            if fixture["family"] == "gravity":
                if match[7] is None or int(match[7]) < 2:
                    raise ValueError("gravity fixture needs its declared command count")
                fixture["expected_commands"] = int(match[7])
            elif match[7] is not None:
                raise ValueError("unexpected per-fixture command count outside gravity")
            if fixture["family"] in ("link", "gravity") and fixture["family"] in completed:
                raise ValueError("Link/gravity fixture follows its completion marker")
            fixtures.append(fixture)
            continue
        match = HOME_END.fullmatch(row)
        if match:
            values = match.groups()
            home_end.append({
                "fixture": values[0],
                "supplied_utf16_length": int(values[1]),
                "native_story_length": int(values[2]),
                "unit": values[3],
                "extend": values[4] == "true",
                "operation": values[5],
                **movement(values, 6),
            })
            continue
        match = REPEAT.fullmatch(row)
        if match:
            values = match.groups()
            if not home_end:
                raise ValueError("repeat precedes initial Home/End command")
            previous = home_end[-1]
            identity = (values[0], values[1], values[2] == "true", values[3])
            if identity != tuple(previous[k] for k in ("fixture", "unit", "extend", "operation")):
                raise ValueError("repeat does not match its initial Home/End command")
            if "repeat" in previous:
                raise ValueError("duplicate Home/End repeat")
            previous["repeat"] = {
                "after": state(values[4:7]),
                "delta": int(values[7]),
                "hresult": values[8],
            }
            continue
        match = VERTICAL.fullmatch(row)
        if match:
            values = match.groups()
            vertical.append({
                "fixture": values[0],
                "unit": values[1],
                "count": int(values[2]),
                "extend": values[3] == "true",
                "operation": values[4],
                **movement(values, 5),
            })
            continue
        match = HORIZONTAL.fullmatch(row)
        if match:
            values = match.groups()
            # Reuse exact directed-state/displacement validation while keeping
            # the two native selection types adjacent to their own states.
            moved = movement(values[:8] + values[9:12] + values[13:], 5)
            moved["before"]["selection_type"] = int(values[8])
            moved["after"]["selection_type"] = int(values[12])
            horizontal.append({
                "fixture": values[0],
                "unit": values[1],
                "count": int(values[2]),
                "extend": values[3] == "true",
                "operation": values[4],
                **moved,
            })
            continue
        match = OPTIONS.fullmatch(row)
        if match:
            values = match.groups()
            before, after_set, after = (state(values[start:start + 3]) for start in (2, 6, 12))
            before["selection_type"] = int(values[5])
            after_set["selection_type"] = int(values[9])
            after["selection_type"] = int(values[15])
            flags = int(values[1])
            if flags > 31:
                raise ValueError("options probe requests undocumented flags")
            text = rust_string(values[18])
            text_length = int(values[19])
            native_length = int(values[17])
            if len(text.encode("utf-16-le")) // 2 != text_length or text_length > native_length:
                raise ValueError("typed story disagrees with its native UTF-16 length")
            if after["end"] > native_length:
                raise ValueError("typing result exceeds its changed native story")
            options.append({
                "fixture": values[0],
                "requested_flags": flags,
                "before": before,
                "after_set": after_set,
                "set_hresult": values[10],
                "typed": rust_string(values[11]),
                "after": after,
                "type_hresult": values[16],
                "native_story_length_after": native_length,
                "story_text": text,
                "story_text_utf16_length": text_length,
            })
            continue
        match = LINK.fullmatch(row)
        if match:
            if "link" in completed:
                raise ValueError("Link command follows its completion marker")
            if not any(fixture["family"] == "link" and fixture["name"] == match["fixture"] for fixture in fixtures):
                raise ValueError("Link command precedes its exact fixture")
            link.append(link_command(match))
            continue
        match = GRAVITY.fullmatch(row)
        if match:
            if "gravity" in completed:
                raise ValueError("gravity command follows its completion marker")
            if not any(fixture["family"] == "gravity" and fixture["name"] == match["fixture"] for fixture in fixtures):
                raise ValueError("gravity command precedes its exact fixture")
            gravity.append(gravity_command(match))
            continue
        match = re.fullmatch(r"PROBE_COMPLETE commands=(\d+) repeated_noops=(\d+)", row)
        if match:
            if "home_end" in completed:
                raise ValueError("duplicate Home/End completion")
            completed["home_end"] = (int(match[1]), int(match[2]))
            continue
        match = re.fullmatch(r"VERTICAL_PROBE_COMPLETE commands=(\d+)", row)
        if match:
            if "vertical" in completed:
                raise ValueError("duplicate vertical completion")
            completed["vertical"] = int(match[1])
            continue
        match = re.fullmatch(r"HORIZONTAL_PROBE_COMPLETE commands=(\d+)", row)
        if match:
            if "horizontal" in completed:
                raise ValueError("duplicate horizontal completion")
            completed["horizontal"] = int(match[1])
            continue
        match = re.fullmatch(r"OPTIONS_PROBE_COMPLETE commands=(\d+)", row)
        if match:
            if "options" in completed:
                raise ValueError("duplicate options completion")
            completed["options"] = int(match[1])
            continue
        match = re.fullmatch(r"LINK_PROBE_COMPLETE commands=(\d+) fixtures=(\d+)", row)
        if match:
            if "link" in completed:
                raise ValueError("duplicate Link completion")
            completed["link"] = (int(match[1]), int(match[2]))
            continue
        match = re.fullmatch(r"GRAVITY_PROBE_COMPLETE commands=(\d+) fixtures=(\d+)", row)
        if match:
            if "gravity" in completed:
                raise ValueError("duplicate gravity completion")
            completed["gravity"] = (int(match[1]), int(match[2]))
            continue
        raise ValueError(f"unrecognized probe row on log line {line_number}: {row}")

    if engine is None or not completed:
        raise ValueError("no completed native probe with engine identity")
    if home_end or "home_end" in completed:
        counts = (len(home_end), sum("repeat" in row for row in home_end))
        if counts != completed.get("home_end") or counts[0] == 0:
            raise ValueError("Home/End observations do not match the completion counts")
    if vertical or "vertical" in completed:
        if len(vertical) != completed.get("vertical") or not vertical:
            raise ValueError("vertical observations do not match the completion count")
    if horizontal or "horizontal" in completed:
        if len(horizontal) != completed.get("horizontal") or not horizontal:
            raise ValueError("horizontal observations do not match the completion count")
    if options or "options" in completed:
        if len(options) != completed.get("options") or not options:
            raise ValueError("options observations do not match the completion count")
    if link or "link" in completed:
        counts = (len(link), sum(fixture["family"] == "link" for fixture in fixtures))
        if counts != completed.get("link") or not link:
            raise ValueError("Link observations do not match the completion counts")
    if gravity or "gravity" in completed:
        counts = (len(gravity), sum(fixture["family"] == "gravity" for fixture in fixtures))
        if counts != completed.get("gravity") or not gravity:
            raise ValueError("gravity observations do not match the completion counts")
    if range_tracking or "range_tracking" in completed:
        counts = (len(range_tracking), sum(fixture["family"] == "range_tracking" for fixture in fixtures))
        if counts != completed.get("range_tracking") or not 0 < counts[0] <= MAX_RANGE_TRACKING_COMMANDS:
            raise ValueError("range tracking observations disagree with bounded completion counts")
    if selection_gravity or "selection_gravity" in completed:
        counts = (len(selection_gravity), sum(fixture["family"] == "selection_gravity" for fixture in fixtures))
        if counts != completed.get("selection_gravity") or not 0 < counts[0] <= MAX_SELECTION_GRAVITY_COMMANDS or len(selection_gravity_interfaces) != counts[1]:
            raise ValueError("selection gravity observations/interfaces disagree with bounded completion counts")
    by_fixture = {}
    for fixture in fixtures:
        identity = (fixture["family"], fixture["name"])
        if identity in by_fixture:
            raise ValueError("duplicate fixture identity")
        if fixture["family"] not in completed:
            raise ValueError("declared fixture family has no completed probe")
        by_fixture[identity] = fixture
    used_fixtures = set()
    for family, rows in (
        ("home_end", home_end), ("vertical", vertical), ("horizontal", horizontal),
        ("options", options),
    ):
        for row in rows:
            fixture = by_fixture.get((family, row["fixture"]))
            if fixture is None:
                raise ValueError("command has no exact text fixture")
            used_fixtures.add((family, row["fixture"]))
            for key in ("supplied_utf16_length", "native_story_length"):
                if key in row and row[key] != fixture[key]:
                    raise ValueError("command lengths disagree with its exact text fixture")
            states = [row["before"], row["after_set"] if family == "options" else row["after"]]
            if "repeat" in row:
                states.append(row["repeat"]["after"])
            if any(value["end"] > fixture["native_story_length"] for value in states):
                raise ValueError("command range exceeds its native story")
    link_rows = {}
    for row in link:
        identity = ("link", row["fixture"])
        if identity not in by_fixture:
            raise ValueError("Link command has no exact text fixture")
        used_fixtures.add(identity)
        link_rows.setdefault(identity, []).append(row)
    for identity, rows in link_rows.items():
        validate_link_steps(by_fixture[identity], rows)
    gravity_rows = {}
    for row in gravity:
        identity = ("gravity", row["fixture"])
        if identity not in by_fixture:
            raise ValueError("gravity command has no exact text fixture")
        used_fixtures.add(identity)
        gravity_rows.setdefault(identity, []).append(row)
    for identity, rows in gravity_rows.items():
        validate_gravity_steps(by_fixture[identity], rows)
    range_tracking_rows = {}
    for row in range_tracking:
        identity = ("range_tracking", row["fixture"])
        if identity not in by_fixture:
            raise ValueError("range tracking command has no exact fixture")
        used_fixtures.add(identity)
        range_tracking_rows.setdefault(identity, []).append(row)
    for identity, rows in range_tracking_rows.items():
        validate_range_tracking_steps(by_fixture[identity], rows)
    selection_gravity_rows = {}
    for row in selection_gravity:
        identity = ("selection_gravity", row["fixture"])
        if identity not in by_fixture:
            raise ValueError("selection gravity command has no exact fixture")
        used_fixtures.add(identity)
        selection_gravity_rows.setdefault(identity, []).append(row)
    for identity, rows in selection_gravity_rows.items():
        validate_selection_gravity_steps(by_fixture[identity], rows)
    if used_fixtures != by_fixture.keys():
        raise ValueError("declared fixture has no observed command")
    result = {
        "engine": engine,
        "fixtures": fixtures,
        "home_end": home_end,
        "vertical": vertical,
        "counts": {
            "home_end": len(home_end),
            "home_end_repeats": sum("repeat" in row for row in home_end),
            "vertical": len(vertical),
        },
    }
    # Keep regeneration of the earlier versioned records byte-identical.
    # The optional horizontal family is present only after an executed probe.
    if horizontal:
        result["horizontal"] = horizontal
        result["counts"]["horizontal"] = len(horizontal)
    if options:
        result["options"] = options
        result["counts"]["options"] = len(options)
    if link:
        result["link"] = link
        result["counts"]["link"] = len(link)
        result["counts"]["link_fixtures"] = len(link_rows)
    if gravity:
        result["gravity"] = gravity
        result["counts"]["gravity"] = len(gravity)
        result["counts"]["gravity_fixtures"] = len(gravity_rows)
    if range_tracking:
        result["range_tracking"] = range_tracking
        result["counts"]["range_tracking"] = len(range_tracking)
        result["counts"]["range_tracking_fixtures"] = len(range_tracking_rows)
    if selection_gravity:
        result["selection_gravity_interfaces"] = selection_gravity_interfaces
        result["selection_gravity"] = selection_gravity
        result["counts"]["selection_gravity"] = len(selection_gravity)
        result["counts"]["selection_gravity_fixtures"] = len(selection_gravity_rows)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log", required=True, type=Path)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--probe-blob", required=True)
    parser.add_argument("--run-url", required=True)
    parser.add_argument("--job-url", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    for value in (args.commit, args.probe_blob):
        if not re.fullmatch(r"[0-9a-f]{40}", value):
            parser.error("commit and probe blob must be full lowercase Git object IDs")
    observations = extract(args.log.read_text(encoding="utf-8"))
    result = {
        "schema_version": 1,
        "scope": "System MSFTEDIT characterization; not the external WinUI text engine.",
        "provenance": {
            "commit": args.commit,
            "probe_path": "src/Windows/FerroUI.Win32/rich_text_tom_oracle_tests.rs",
            "probe_git_blob": args.probe_blob,
            "run_url": args.run_url,
            "job_url": args.job_url,
            "command": "cargo test --locked -p ferroui-win32 --no-default-features --lib rich_text_tom_oracle -- --nocapture --test-threads=1",
        },
        **observations,
    }
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"output": str(args.output), "counts": observations["counts"]}))


if __name__ == "__main__":
    main()
