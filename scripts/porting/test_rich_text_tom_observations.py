#!/usr/bin/env python3
"""Extraction integrity tests; synthetic rows are not native conformance evidence."""

from __future__ import annotations

import importlib.util
import copy
import json
from pathlib import Path
import unittest


SCRIPT = Path(__file__).with_name("rich-text-tom-observations.py")
SPEC = importlib.util.spec_from_file_location("rich_text_tom_observations", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
PARSER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PARSER)


def debug_string(value: str) -> str:
    escapes = {'"': '\\"', "\\": "\\\\", "\n": "\\n", "\r": "\\r", "\t": "\\t", "\0": "\\0"}
    return '"' + "".join(
        escapes.get(char, f"\\u{{{ord(char):x}}}" if ord(char) < 0x20 or char == "\u200d" else char)
        for char in value
    ) + '"'


def native_state(value: dict) -> str:
    return "SelectionState { start: %s, end: %s, flags: %s }" % (
        value["start"], value["end"], value["flags"]
    )


def options_row(row: dict) -> str:
    return (
        f'OPTIONS fixture={row["fixture"]} requested_flags={row["requested_flags"]} '
        f'before={native_state(row["before"])} before_type={row["before"]["selection_type"]} '
        f'after_set={native_state(row["after_set"])} after_set_type={row["after_set"]["selection_type"]} '
        f'set_hresult={row["set_hresult"]} typed={debug_string(row["typed"])} '
        f'after={native_state(row["after"])} after_type={row["after"]["selection_type"]} '
        f'type_hresult={row["type_hresult"]} native_story_length_after={row["native_story_length_after"]} '
        f'story_text={debug_string(row["story_text"])} story_text_utf16_length={row["story_text_utf16_length"]}'
    )


def link_snapshot_row(snapshot: dict) -> str:
    return (
        'LinkSnapshot { range: RangeState { start: %s, end: %s }, '
        'range_text: %s, range_text_utf16_length: %s, story_text: %s, '
        'story_text_utf16_length: %s, story_length: %s, selection: %s }'
    ) % (
        snapshot["range"]["start"], snapshot["range"]["end"],
        debug_string(snapshot["range_text"]), snapshot["range_text_utf16_length"],
        debug_string(snapshot["story_text"]), snapshot["story_text_utf16_length"],
        snapshot["story_length"], native_state(snapshot["selection"]),
    )


def link_row(row: dict) -> str:
    return (
        f'LINK fixture={row["fixture"]} step={row["step"]} role={row["role"]} '
        f'operation={row["operation"]} initial_start={row["initial_range"]["start"]} '
        f'initial_end={row["initial_range"]["end"]} '
        f'argument_null={str(row["argument"]["is_null"]).lower()} '
        f'argument={debug_string(row["argument"]["text"])} argument_utf16_length={row["argument"]["utf16_length"]} '
        f'before={link_snapshot_row(row["before"])} after={link_snapshot_row(row["after"])} '
        f'hresult={row["hresult"]} result_null={str(row["result"]["is_null"]).lower()} '
        f'result={debug_string(row["result"]["text"])} result_utf16_length={row["result"]["utf16_length"]}'
    )


def native_number_row(value: dict) -> str:
    return 'NativeNumber { hresult: %s, value: %s }' % (value["hresult"], value["value"])


def range_state_row(value: dict) -> str:
    return 'RangeState { start: %s, end: %s }' % (value["start"], value["end"])


def gravity_range_row(value: dict) -> str:
    result = 'GravityRangeState { range: ' + range_state_row(value["range"]) + ', '
    for name in ("gravity", "flags", "bold", "italic", "alignment"):
        result += name + ': ' + native_number_row(value[name]) + ', '
    return result + f'text: {debug_string(value["text"])}, text_utf16_length: {value["text_utf16_length"]} }}'


def gravity_snapshot_row(value: dict) -> str:
    duplicate = 'None' if value["duplicate"] is None else 'Some(' + gravity_range_row(value["duplicate"]) + ')'
    return (
        'GravitySnapshot { primary: ' + gravity_range_row(value["primary"]) + ', duplicate: ' + duplicate
        + f', story_text: {debug_string(value["story_text"])}, story_text_utf16_length: {value["story_text_utf16_length"]}'
        + f', story_length: {value["story_length"]}, selection: {native_state(value["selection"])} }}'
    )


def gravity_row(row: dict) -> str:
    output = 'None' if row["output"] is None else 'Some(%s)' % row["output"]
    return (
        f'GRAVITY fixture={row["fixture"]} step={row["step"]} role={row["role"]} target={row["target"]} '
        f'operation={row["operation"]} argument1={row["argument1"]} argument2={row["argument2"]} value={row["value"]} '
        f'text={debug_string(row["text"])} text_utf16_length={row["text_utf16_length"]} '
        f'action_before={range_state_row(row["action_before"])} action_after={range_state_row(row["action_after"])} '
        f'before={gravity_snapshot_row(row["before"])} after={gravity_snapshot_row(row["after"])} '
        f'hresult={row["hresult"]} output={output}'
    )


def native_interface_row(value: dict) -> str:
    identity = value["identity"]
    rendered = 'None' if identity is None else (
        'Some(NativeIdentity { hresult: %s, is_null: %s, same_identity: %s })'
        % (identity["hresult"], str(identity["is_null"]).lower(), str(identity["same_identity"]).lower())
    )
    return 'NativeInterface { hresult: %s, is_null: %s, identity: %s }' % (
        value["hresult"], str(value["is_null"]).lower(), rendered,
    )


def native_text_row(value: dict) -> str:
    return 'NativeText { hresult: %s, is_null: %s, text: %s, text_utf16_length: %s }' % (
        value["hresult"], str(value["is_null"]).lower(), debug_string(value["text"]), value["text_utf16_length"],
    )


def selection_gravity_range_row(value: dict) -> str:
    result = 'SelectionGravityRangeState { range: ' + range_state_row(value["range"]) + ', '
    for name in ("gravity", "flags", "selection_type", "bold", "italic", "alignment"):
        result += name + ': ' + native_number_row(value[name]) + ', '
    return result + 'text: ' + native_text_row(value["text"]) + ' }'


def selection_gravity_snapshot_row(value: dict) -> str:
    duplicate = 'None' if value["duplicate"] is None else 'Some(' + selection_gravity_range_row(value["duplicate"]) + ')'
    return (
        'SelectionGravitySnapshot { primary: ' + selection_gravity_range_row(value["primary"])
        + ', duplicate: ' + duplicate + ', story_text: ' + native_text_row(value["story_text"])
        + ', story_length: ' + native_number_row(value["story_length"])
        + ', selection: ' + native_state(value["selection"]) + ' }'
    )


def selection_gravity_interface_row(value: dict) -> str:
    return (
        f'SELECTION_GRAVITY_INTERFACE fixture={value["fixture"]} source_hresult={value["source_hresult"]} '
        f'source_null={str(value["source_null"]).lower()} range2={native_interface_row(value["range2"])} '
        f'selection2={native_interface_row(value["selection2"])}'
    )


def selection_gravity_row(row: dict) -> str:
    output = 'None' if row["output"] is None else 'Some(%s)' % row["output"]
    returned_null = 'None' if row["returned_null"] is None else 'Some(%s)' % str(row["returned_null"]).lower()
    interface = 'None' if row["duplicate_interface"] is None else 'Some(' + native_interface_row(row["duplicate_interface"]) + ')'
    return (
        f'SELECTION_GRAVITY fixture={row["fixture"]} step={row["step"]} role={row["role"]} target={row["target"]} '
        f'operation={row["operation"]} argument1={row["argument1"]} argument2={row["argument2"]} value={row["value"]} '
        f'extend={str(row["extend"]).lower()} text_null={str(row["text_null"]).lower()} '
        f'text={debug_string(row["text"])} text_utf16_length={row["text_utf16_length"]} '
        f'action_before={selection_gravity_range_row(row["action_before"])} '
        f'action_after={selection_gravity_range_row(row["action_after"])} '
        f'before={selection_gravity_snapshot_row(row["before"])} '
        f'after={selection_gravity_snapshot_row(row["after"])} '
        f'hresult={row["hresult"]} output={output} returned_null={returned_null} duplicate_interface={interface}'
    )


def synthetic_selection_gravity_record(default: bool = False) -> dict:
    """Invented parser inputs, not expected native selection behavior.

    In particular, mode 4 deliberately reads back as 4, and the untouched
    default deliberately reads back as 2. Extraction cannot assume the measured
    independent-range results are the actual selection's behavior.
    """
    def number(value: int, hresult: str = "0x00000000") -> dict:
        return {"hresult": hresult, "value": value}

    def text_value(text: str, is_null: bool = False) -> dict:
        return {"hresult": "0x00000000", "is_null": is_null, "text": text,
                "text_utf16_length": len(text.encode("utf-16-le")) // 2}

    def interface(same: bool = True) -> dict:
        return {"hresult": "0x00000000", "is_null": False,
                "identity": {"hresult": "0x00000000", "is_null": False, "same_identity": same}}

    def tracked(story: str, start: int, end: int, gravity: int, actual: bool = True, null_text: bool = False) -> dict:
        text = story.encode("utf-16-le")[2 * start:2 * end].decode("utf-16-le")
        return {
            "range": {"start": start, "end": end}, "gravity": number(gravity),
            "flags": number(17 if actual else -(2 ** 31), "0x00000000" if actual else "0x80004001"),
            "selection_type": number(1 if start == end else 2, "0x00000000" if actual else "0x80004001"),
            "bold": number(0), "italic": number(0), "alignment": number(0),
            "text": text_value(text, null_text),
        }

    story = "\r" if default else "a😀b\r"
    current = {
        "primary": tracked(story, 0, 0, 2 if default else 0, null_text=True), "duplicate": None,
        "story_text": text_value(story), "story_length": number(len(story.encode("utf-16-le")) // 2),
        "selection": {"start": 0, "end": 0, "flags": 17, "anchor": 0, "active": 0},
    }
    rows = []

    def select(snapshot: dict, start: int, end: int, gravity: int) -> dict:
        snapshot = copy.deepcopy(snapshot)
        snapshot["primary"] = tracked(snapshot["story_text"]["text"], start, end, gravity)
        snapshot["selection"] = {"start": start, "end": end, "flags": 17, "anchor": end, "active": start}
        return snapshot

    def add(operation: str, after: dict, *, target: str = "Primary", a: int = 0, b: int = 0,
            value: int = 0, role: str = "Target", text: str | None = None, extend: bool = False,
            output: int | None = None, hresult: str = "0x00000000") -> None:
        nonlocal current
        action_before = tracked(current["story_text"]["text"], a, b, 0, False) if target == "External" else current[target.lower()]
        action_after = tracked(after["story_text"]["text"], a, b, 0, False) if target == "External" else after[target.lower()]
        rows.append(copy.deepcopy({
            "fixture": "selection_default" if default else "selection_configured", "step": len(rows),
            "role": role, "target": target, "operation": operation, "argument1": a, "argument2": b,
            "value": value, "extend": extend, "text_null": text is None, "text": text or "",
            "text_utf16_length": len((text or "").encode("utf-16-le")) // 2,
            "action_before": action_before, "action_after": action_after, "before": current, "after": after,
            "hresult": hresult, "output": output,
            "returned_null": False if operation == "Duplicate" else None,
            "duplicate_interface": interface(False) if operation == "Duplicate" else None,
        }))
        current = copy.deepcopy(after)

    if default:
        after = copy.deepcopy(current)
        after["story_text"] = text_value("X\r")
        after["story_length"] = number(2)
        after = select(after, 1, 1, 2)
        add("TypeText", after, text="X")
        add("GetGravity", current, output=2)
    else:
        after = copy.deepcopy(current)
        after["primary"]["bold"] = number(-1)
        add("SetBold", after, role="Setup", target="External", a=0, b=1, value=-1)
        add("SetRange", select(current, 1, 3, 0), a=3, b=1)
        after = copy.deepcopy(current)
        after["primary"]["gravity"] = number(4)
        add("SetGravity", after, value=4)
        after = copy.deepcopy(current)
        after["duplicate"] = tracked(story, 1, 3, 0, False)
        add("Duplicate", after)
        after = copy.deepcopy(current)
        after["duplicate"]["gravity"] = number(2)
        add("SetGravity", after, target="Duplicate", value=2)
        add("MoveRight", select(current, 3, 3, 4), a=1, b=1, output=1)
        add("SetGravity", current, value=-1, hresult="0x80070057")
        add("TypeText", current, text=None, hresult="0x00000001")
        add("TypeText", current, text="", hresult="0x00000001")
        add("SetRange", select(current, 1, 1, 4), a=1, b=1)
        after = copy.deepcopy(current)
        after["story_text"] = text_value("aX😀b\r")
        after["story_length"] = number(6)
        after["duplicate"] = tracked("aX😀b\r", 1, 4, 2, False)
        after = select(after, 2, 2, 4)
        add("TypeText", after, text="X")
        add("GetGravity", current, target="External", a=1, b=2, output=0)
        add("EndKey", select(current, 5, 5, 2), a=5, output=1)
        add("GetGravity", current, output=2)
    name = rows[0]["fixture"]
    supplied = "" if default else "a😀b"
    return {
        "engine": {"windows_version": [10, 0, 26100], "dll_path": "Msftedit.dll", "dll_file_version": "1.2.3.4"},
        "fixtures": [{"family": "selection_gravity", "name": name, "text": supplied,
                      "supplied_utf16_length": len(supplied.encode("utf-16-le")) // 2,
                      "native_story_length": 1 if default else 5, "window_ex_style": 0,
                      "expected_commands": len(rows), "initialization": "Default" if default else "Configured"}],
        "home_end": [], "vertical": [],
        "selection_gravity_interfaces": [{"fixture": name, "source_hresult": "0x00000000", "source_null": False,
                                           "range2": interface(), "selection2": interface()}],
        "selection_gravity": rows,
        "counts": {"home_end": 0, "home_end_repeats": 0, "vertical": 0,
                   "selection_gravity": len(rows), "selection_gravity_fixtures": 1},
    }




def synthetic_selection_gravity_extension(count: int) -> dict:
    """Independent invented fixtures for validating the extended family budget.

    A repeated final getter makes each fixture 15 commands. This tests the
    declared-count protocol; it is not a prediction of paragraph typing.
    """
    result = synthetic_selection_gravity_record()
    for key in ("fixtures", "selection_gravity_interfaces", "selection_gravity"):
        result[key] = []
    for index in range(count):
        fixture = synthetic_selection_gravity_record()
        name = f"synthetic_extension_{index}"
        fixture["fixtures"][0]["name"] = name
        fixture["fixtures"][0]["expected_commands"] = 15
        fixture["selection_gravity_interfaces"][0]["fixture"] = name
        fixture["selection_gravity"].append(copy.deepcopy(fixture["selection_gravity"][-1]))
        for step, row in enumerate(fixture["selection_gravity"]):
            row["fixture"], row["step"] = name, step
        for key in ("fixtures", "selection_gravity_interfaces", "selection_gravity"):
            result[key].extend(fixture[key])
    result["counts"].update(selection_gravity=15 * count, selection_gravity_fixtures=count)
    return result


def record_log(record: dict) -> str:
    """Reconstitute the probe grammar from immutable previously captured rows."""
    engine = record["engine"]
    version = ", ".join(map(str, engine["windows_version"]))
    lines = [f'ENGINE windows=({version}) dll={json.dumps(engine["dll_path"])} file_version={engine["dll_file_version"]}']
    for fixture in record["fixtures"]:
        prefix = "" if fixture["family"] == "home_end" else fixture["family"].upper() + "_"
        line = (
            f'{prefix}FIXTURE name={fixture["name"]} text={debug_string(fixture["text"])} '
            f'supplied_utf16_length={fixture["supplied_utf16_length"]} native_story_length={fixture["native_story_length"]}'
        )
        if "window_ex_style" in fixture:
            line += f' window_ex_style={fixture["window_ex_style"]}'
        if "expected_commands" in fixture:
            line += f' expected_commands={fixture["expected_commands"]}'
        if "initialization" in fixture:
            line += f' initialization={fixture["initialization"]}'
        lines.append(line)
    for interface in record.get("selection_gravity_interfaces", []):
        lines.append(selection_gravity_interface_row(interface))
    for family in ("home_end", "vertical", "horizontal"):
        for row in record.get(family, []):
            identity = f'{family.upper()} fixture={row["fixture"]} '
            if family == "home_end":
                identity += f'supplied_utf16_length={row["supplied_utf16_length"]} native_story_length={row["native_story_length"]} '
            identity += f'unit={row["unit"]} '
            if family != "home_end":
                identity += f'count={row["count"]} '
            identity += f'extend={str(row["extend"]).lower()} operation={row["operation"]} '
            states = f'before={native_state(row["before"])} '
            if family == "horizontal":
                states += f'before_type={row["before"]["selection_type"]} '
            states += f'after={native_state(row["after"])} '
            if family == "horizontal":
                states += f'after_type={row["after"]["selection_type"]} '
            lines.append(
                identity + states + f'anchor={row["after"]["anchor"]} active={row["after"]["active"]} '
                f'delta={row["delta"]} active_displacement={row["active_displacement"]} hresult={row["hresult"]}'
            )
            if "repeat" in row:
                repeat = row["repeat"]
                lines.append(
                    f'HOME_END_REPEAT fixture={row["fixture"]} unit={row["unit"]} '
                    f'extend={str(row["extend"]).lower()} operation={row["operation"]} '
                    f'after={native_state(repeat["after"])} delta={repeat["delta"]} hresult={repeat["hresult"]}'
                )
    for row in record.get("options", []):
        lines.append(options_row(row))
    for row in record.get("link", []):
        lines.append(link_row(row))
    for row in record.get("gravity", []):
        lines.append(gravity_row(row))
    for row in record.get("selection_gravity", []):
        lines.append(selection_gravity_row(row))
    for row in record.get("range_tracking", []):
        lines.append(gravity_row(row).replace("GRAVITY ", "RANGE_TRACKING ", 1))
    for family in ("home_end", "vertical", "horizontal", "options", "link", "gravity", "selection_gravity", "range_tracking"):
        count = record["counts"].get(family, 0)
        if not count:
            continue
        if family == "home_end":
            lines.append(f'PROBE_COMPLETE commands={count} repeated_noops={record["counts"]["home_end_repeats"]}')
        elif family == "link":
            lines.append(f'LINK_PROBE_COMPLETE commands={count} fixtures={record["counts"]["link_fixtures"]}')
        elif family == "selection_gravity":
            lines.append(f'SELECTION_GRAVITY_PROBE_COMPLETE commands={count} fixtures={record["counts"]["selection_gravity_fixtures"]}')
        elif family in ("gravity", "range_tracking"):
            lines.append(f'{family.upper()}_PROBE_COMPLETE commands={count} fixtures={record["counts"][family + "_fixtures"]}')
        else:
            lines.append(f'{family.upper()}_PROBE_COMPLETE commands={count}')
    return "\n".join("SYSTEM_MSFTEDIT_" + line for line in lines)


SYNTHETIC_OPTIONS = (
    'SYSTEM_MSFTEDIT_ENGINE windows=(10, 0, 26100) dll="Msftedit.dll" file_version=1.2.3.4\n'
    'SYSTEM_MSFTEDIT_OPTIONS_FIXTURE name=example text="abcdef" supplied_utf16_length=6 native_story_length=7 window_ex_style=0\n'
    'SYSTEM_MSFTEDIT_OPTIONS fixture=example requested_flags=17 '
    'before=SelectionState { start: 1, end: 4, flags: 16 } before_type=2 '
    'after_set=SelectionState { start: 1, end: 4, flags: 17 } after_set_type=2 '
    'set_hresult=0x00000000 typed="😀" '
    'after=SelectionState { start: 3, end: 3, flags: 17 } after_type=1 '
    'type_hresult=0x00000000 native_story_length_after=6 story_text="a😀ef\\r" story_text_utf16_length=6\n'
    'SYSTEM_MSFTEDIT_OPTIONS_PROBE_COMPLETE commands=1'
)


def synthetic_link_record() -> dict:
    """Invented parser cases; no return value or hidden-field layout is evidence."""
    def snapshot(start: int, end: int, text: str, story: str) -> dict:
        return {
            "range": {"start": start, "end": end},
            "range_text": text,
            "range_text_utf16_length": len(text.encode("utf-16-le")) // 2,
            "story_text": story,
            "story_text_utf16_length": len(story.encode("utf-16-le")) // 2,
            "story_length": len(story.encode("utf-16-le")) // 2,
            "selection": {"start": 0, "end": 0, "flags": 17, "anchor": 0, "active": 0},
        }

    null_bstr = {"is_null": True, "text": "", "utf16_length": 0}
    url = '"https://例子.example/😀"'
    url_bstr = {"is_null": False, "text": url, "utf16_length": len(url.encode("utf-16-le")) // 2}
    initial = snapshot(0, 1, "a", "ab\r")
    hidden = snapshot(3, 4, "a", "\x13X\x14ab\r")
    expanded = snapshot(0, 4, "\x13X\x14a", hidden["story_text"])
    rows = [
        {
            "fixture": "example_link", "step": 0, "role": "Target", "operation": "Set",
            "initial_range": {"start": 0, "end": 1}, "argument": url_bstr,
            "result": null_bstr, "before": initial, "after": hidden, "hresult": "0x00000000",
        },
        {
            "fixture": "example_link", "step": 1, "role": "Readback", "operation": "Get",
            "initial_range": {"start": 0, "end": 1}, "argument": null_bstr,
            "result": url_bstr, "before": hidden, "after": expanded, "hresult": "0x00000000",
        },
    ]
    return {
        "engine": {"windows_version": [10, 0, 26100], "dll_path": "Msftedit.dll", "dll_file_version": "1.2.3.4"},
        "fixtures": [{
            "family": "link", "name": "example_link", "text": "ab", "supplied_utf16_length": 2,
            "native_story_length": 3, "window_ex_style": 0,
        }],
        "home_end": [], "vertical": [], "link": [copy.deepcopy(row) for row in rows],
        "counts": {"home_end": 0, "home_end_repeats": 0, "vertical": 0, "link": 2, "link_fixtures": 1},
    }


def synthetic_gravity_record() -> dict:
    """Invented parser states, explicitly not native gravity/format expectations."""
    def number(value: int, hresult: str = "0x00000000") -> dict:
        return {"hresult": hresult, "value": value}

    def text_length(text: str) -> int:
        return len(text.encode("utf-16-le")) // 2

    def tracked(start: int, end: int, gravity: int, story: str) -> dict:
        text = story.encode("utf-16-le")[2 * start:2 * end].decode("utf-16-le")
        return {
            "range": {"start": start, "end": end}, "gravity": number(gravity),
            "flags": number(-(2 ** 31), "0x80004001"), "bold": number(0),
            "italic": number(0), "alignment": number(0), "text": text,
            "text_utf16_length": text_length(text),
        }

    current = {
        "primary": tracked(0, 0, 0, "a😀b\r"), "duplicate": None,
        "story_text": "a😀b\r", "story_text_utf16_length": 5, "story_length": 5,
        "selection": {"start": 0, "end": 0, "flags": 17, "anchor": 0, "active": 0},
    }
    rows = []

    def add(operation: str, after: dict, *, target: str = "Primary", a: int = 0, b: int = 0,
            value: int = 0, text: str = "", role: str = "Target", hresult: str = "0x00000000",
            output: int | None = None) -> None:
        nonlocal current
        action_before = {"start": a, "end": b} if target == "External" else current[target.lower()]["range"]
        action_after = (
            {"start": a, "end": a + text_length(text)} if operation == "SetText" else action_before
        ) if target == "External" else after[target.lower()]["range"]
        rows.append(copy.deepcopy({
            "fixture": "example_gravity", "step": len(rows), "role": role, "target": target,
            "operation": operation, "argument1": a, "argument2": b, "value": value,
            "text": text, "text_utf16_length": text_length(text), "before": current,
            "after": after, "action_before": action_before, "action_after": action_after,
            "hresult": hresult, "output": output,
        }))
        current = copy.deepcopy(after)

    after = copy.deepcopy(current)
    after["primary"]["bold"] = number(-1)
    add("SetBold", after, target="External", a=0, b=1, value=-1, role="Setup")
    after = copy.deepcopy(current)
    after["primary"] = tracked(1, 3, 0, current["story_text"])
    after["primary"]["bold"] = number(-9_999_999)
    add("SetRange", after, a=3, b=1)
    after = copy.deepcopy(current)
    after["primary"]["gravity"] = number(4)
    add("SetGravity", after, value=4)
    after = copy.deepcopy(current)
    after["duplicate"] = copy.deepcopy(after["primary"])
    add("Duplicate", after)
    after = copy.deepcopy(current)
    after.update(story_text="aX😀b\r", story_text_utf16_length=6, story_length=6)
    for name in ("primary", "duplicate"):
        after[name] = tracked(1, 4, 4, after["story_text"])
        after[name]["bold"] = number(-9_999_999)
    add("SetText", after, target="External", a=1, b=1, text="X")
    after = copy.deepcopy(current)
    after["duplicate"]["gravity"] = number(2)
    add("SetGravity", after, target="Duplicate", value=2)
    after = copy.deepcopy(current)
    after["primary"] = tracked(0, 0, 4, after["story_text"])
    add("SetRange", after, a=0, b=0)
    add("GetGravity", current, output=4)
    add("SetGravity", current, value=-(2 ** 31), hresult="0x80070057")
    add("Collapse", current, value=-1, hresult="0x00000001")
    add("GetGravity", current, output=4)
    return {
        "engine": {"windows_version": [10, 0, 26100], "dll_path": "Msftedit.dll", "dll_file_version": "1.2.3.4"},
        "fixtures": [{
            "family": "gravity", "name": "example_gravity", "text": "a😀b", "supplied_utf16_length": 4,
            "native_story_length": 5, "window_ex_style": 0, "expected_commands": len(rows),
        }],
        "home_end": [], "vertical": [], "gravity": rows,
        "counts": {"home_end": 0, "home_end_repeats": 0, "vertical": 0, "gravity": len(rows), "gravity_fixtures": 1},
    }


def synthetic_range_tracking_record(count: int = 1) -> dict:
    """Invented six-step protocol rows; these are not endpoint expectations.

    Requested mode 0 or 2 deliberately reads back as 4. The parser must retain
    observed output rather than infer native behavior from the requested mode.
    """
    old = synthetic_gravity_record()
    template = copy.deepcopy(old["gravity"][1:5])
    current = template[-1]["after"]
    for target in ("Primary", "Duplicate"):
        template.append({
            "fixture": "synthetic_tracking", "step": len(template), "role": "Target",
            "target": target, "operation": "GetGravity", "argument1": 0, "argument2": 0,
            "value": 0, "text": "", "text_utf16_length": 0,
            "action_before": copy.deepcopy(current[target.lower()]["range"]),
            "action_after": copy.deepcopy(current[target.lower()]["range"]),
            "before": copy.deepcopy(current), "after": copy.deepcopy(current),
            "hresult": "0x00000000", "output": current[target.lower()]["gravity"]["value"],
        })
    record = {
        "engine": copy.deepcopy(old["engine"]), "fixtures": [], "home_end": [], "vertical": [],
        "range_tracking": [],
        "counts": {"home_end": 0, "home_end_repeats": 0, "vertical": 0,
                   "range_tracking": 6 * count, "range_tracking_fixtures": count},
    }
    for index in range(count):
        name = f"synthetic_tracking_{index}"
        fixture = copy.deepcopy(old["fixtures"][0])
        fixture.update(family="range_tracking", name=name, expected_commands=6)
        record["fixtures"].append(fixture)
        rows = copy.deepcopy(template)
        for step, row in enumerate(rows):
            row["fixture"], row["step"] = name, step
        rows[1]["value"] = 0 if index < 12 else 2
        record["range_tracking"].extend(rows)
    return record


class NativeObservationExtractionTests(unittest.TestCase):
    def test_every_versioned_record_roundtrips_without_changing_observations(self) -> None:
        paths = sorted((SCRIPT.parents[2] / "docs/porting/winui-rich-text").glob("native-msftedit-*.json"))
        self.assertGreaterEqual(len(paths), 4)
        for path in paths:
            with self.subTest(record=path.name):
                record = json.loads(path.read_text(encoding="utf-8"))
                expected = {key: value for key, value in record.items() if key not in ("schema_version", "scope", "provenance")}
                self.assertEqual(PARSER.extract(record_log(record)), expected)

    def test_options_preserve_each_phase_direction_type_and_changed_story(self) -> None:
        record = PARSER.extract(SYNTHETIC_OPTIONS)
        row = record["options"][0]
        self.assertEqual(record["counts"]["options"], 1)
        self.assertEqual((row["before"]["anchor"], row["before"]["active"]), (1, 4))
        self.assertEqual((row["after_set"]["anchor"], row["after_set"]["active"]), (4, 1))
        self.assertEqual(row["after"]["selection_type"], 1)
        self.assertEqual(row["after"]["active"], 3)
        self.assertEqual(row["typed"], "😀")
        self.assertEqual(row["story_text"], "a😀ef\r")
        self.assertEqual(row["native_story_length_after"], 6)
        self.assertEqual(PARSER.extract(record_log(record)), record)

    def test_options_allow_story_growth_beyond_original_fixture(self) -> None:
        log = SYNTHETIC_OPTIONS.replace(
            "start: 3, end: 3, flags: 17", "start: 9, end: 9, flags: 17"
        ).replace("native_story_length_after=6", "native_story_length_after=10").replace(
            'story_text="a😀ef\\r" story_text_utf16_length=6',
            'story_text="abcdefXYZ\\r" story_text_utf16_length=10',
        )
        self.assertEqual(PARSER.extract(log)["options"][0]["after"]["end"], 9)

    def test_options_reject_incomplete_duplicate_and_mismatched_counts(self) -> None:
        for log in (
            SYNTHETIC_OPTIONS.rsplit("\n", 1)[0],
            SYNTHETIC_OPTIONS.replace("commands=1", "commands=2"),
            SYNTHETIC_OPTIONS + "\nSYSTEM_MSFTEDIT_OPTIONS_PROBE_COMPLETE commands=1",
        ):
            with self.subTest(log=log[-90:]), self.assertRaises(ValueError):
                PARSER.extract(log)

    def test_options_reject_inconsistent_states_lengths_flags_and_fixture_identity(self) -> None:
        edits = (
            ("requested_flags=17", "requested_flags=32"),
            ("fixture=example", "fixture=missing"),
            ("start: 1, end: 4, flags: 17", "start: 1, end: 8, flags: 17"),
            ("start: 3, end: 3, flags: 17", "start: 7, end: 7, flags: 17"),
            ("start: 3, end: 3, flags: 17", "start: 4, end: 3, flags: 17"),
            ("story_text_utf16_length=6", "story_text_utf16_length=5"),
            ("native_story_length_after=6", "native_story_length_after=5"),
            ("native_story_length=7", "native_story_length=5"),
            ("window_ex_style=0", "window_ex_style=4294967296"),
            (" window_ex_style=0", ""),
        )
        for old, new in edits:
            with self.subTest(old=old, new=new), self.assertRaises(ValueError):
                PARSER.extract(SYNTHETIC_OPTIONS.replace(old, new))

    def test_an_earlier_complete_probe_does_not_hide_a_later_incomplete_family(self) -> None:
        fixture = (
            '\nSYSTEM_MSFTEDIT_HORIZONTAL_FIXTURE name=incomplete text="a" '
            'supplied_utf16_length=1 native_story_length=2 window_ex_style=0'
        )
        with self.assertRaisesRegex(ValueError, "no completed probe"):
            PARSER.extract(SYNTHETIC_OPTIONS + fixture)
        orphan = fixture.replace("HORIZONTAL_FIXTURE", "OPTIONS_FIXTURE")
        with self.assertRaisesRegex(ValueError, "no observed command"):
            PARSER.extract(SYNTHETIC_OPTIONS + orphan)

    def test_rust_debug_escaping_preserves_unicode_and_literal_backslashes(self) -> None:
        value = 'A😀e\u0301👩\u200d👦\\u{200d}\0\r\n\t"'
        self.assertEqual(PARSER.rust_string(debug_string(value)), value)
        for invalid in ('"\\u{d800}"', '"\\u{110000}"', '"\\u{}"', '"\\x20"', '"\\u200d"', '"\n"'):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                PARSER.rust_string(invalid)

    def test_link_preserves_exact_bstr_utf16_hidden_story_and_getter_range_expansion(self) -> None:
        expected = synthetic_link_record()
        record = PARSER.extract(record_log(expected))
        self.assertEqual(record, expected)
        setter, getter = record["link"]
        self.assertEqual(setter["before"]["story_length"], 3)
        self.assertEqual(setter["after"]["story_length"], 6)
        self.assertEqual(getter["before"]["range"], {"start": 3, "end": 4})
        self.assertEqual(getter["after"]["range"], {"start": 0, "end": 4})
        self.assertIn("😀", getter["result"]["text"])

    def test_link_preserves_null_and_allocated_empty_bstr_separately(self) -> None:
        for is_null in (False, True):
            record = synthetic_link_record()
            record["link"][1]["result"] = {"is_null": is_null, "text": "", "utf16_length": 0}
            with self.subTest(is_null=is_null):
                self.assertEqual(PARSER.extract(record_log(record)), record)

    def test_link_retains_target_hresult_failures_and_observes_readback(self) -> None:
        record = synthetic_link_record()
        setter, getter = record["link"]
        setter["hresult"] = "0x80070057"
        setter["after"] = copy.deepcopy(setter["before"])
        getter["before"] = copy.deepcopy(setter["after"])
        getter["after"] = copy.deepcopy(getter["before"])
        getter["hresult"] = "0x00000001"
        getter["result"] = {"is_null": True, "text": "", "utf16_length": 0}
        self.assertEqual(PARSER.extract(record_log(record)), record)

    def test_link_setup_has_explicit_successful_history_and_one_target(self) -> None:
        record = synthetic_link_record()
        record["link"][0]["role"] = "Setup"
        record["link"][1]["role"] = "Target"
        self.assertEqual(PARSER.extract(record_log(record)), record)
        record["link"][0]["hresult"] = "0x80070057"
        with self.assertRaisesRegex(ValueError, "successful SetURL"):
            PARSER.extract(record_log(record))

    def test_link_rejects_incomplete_duplicate_reordered_and_mismatched_steps(self) -> None:
        baseline = synthetic_link_record()
        invalid = []
        for steps in (
            [baseline["link"][0]],
            [baseline["link"][1], baseline["link"][0]],
            [baseline["link"][0], baseline["link"][0]],
        ):
            record = copy.deepcopy(baseline)
            record["link"] = steps
            record["counts"]["link"] = len(steps)
            invalid.append(record_log(record))
        log = record_log(baseline)
        invalid.extend((
            log.rsplit("\n", 1)[0],
            log.replace("commands=2 fixtures=1", "commands=3 fixtures=1"),
            log.replace("commands=2 fixtures=1", "commands=2 fixtures=2"),
            log + "\nSYSTEM_MSFTEDIT_LINK_PROBE_COMPLETE commands=2 fixtures=1",
            log + "\nSYSTEM_MSFTEDIT_" + link_row(baseline["link"][0]),
            log.replace("step=1 role=Readback", "step=2 role=Readback"),
        ))
        for value in invalid:
            with self.subTest(log=value[-100:]), self.assertRaises(ValueError):
                PARSER.extract(value)

    def test_link_rejects_contradictory_range_story_bstr_and_phase_metadata(self) -> None:
        baseline = synthetic_link_record()
        for row_index, path, value in (
            (0, ("initial_range", "end"), 4),
            (0, ("before", "range", "start"), 1),
            (0, ("after", "story_length"), 3),
            (0, ("after", "range_text_utf16_length"), 2),
            (0, ("after", "selection", "end"), 7),
            (0, ("argument", "is_null"), True),
            (0, ("argument", "utf16_length"), 1),
            (0, ("result", "is_null"), False),
            (1, ("argument", "is_null"), False),
            (1, ("initial_range", "end"), 2),
            (1, ("before", "range", "start"), 2),
            (1, ("before", "story_text"), "\x13Y\x14ab\r"),
        ):
            record = copy.deepcopy(baseline)
            target = record["link"][row_index]
            for name in path[:-1]:
                target = target[name]
            target[path[-1]] = value
            with self.subTest(row=row_index, path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_link_rejects_undeclared_or_late_fixtures(self) -> None:
        lines = record_log(synthetic_link_record()).splitlines()
        for reordered in (
            [lines[0], lines[2], lines[1], *lines[3:]],
            [*lines, lines[1].replace("name=example_link", "name=late")],
        ):
            with self.assertRaises(ValueError):
                PARSER.extract("\n".join(reordered))

    def test_gravity_roundtrips_raw_getters_formats_direction_and_live_duplicates(self) -> None:
        expected = synthetic_gravity_record()
        actual = PARSER.extract(record_log(expected))
        self.assertEqual(actual, expected)
        rows = actual["gravity"]
        self.assertEqual((rows[1]["argument1"], rows[1]["argument2"]), (3, 1))
        self.assertEqual(rows[1]["after"]["primary"]["bold"]["value"], -9_999_999)
        self.assertEqual(rows[1]["after"]["primary"]["flags"], {"hresult": "0x80004001", "value": -(2 ** 31)})
        self.assertEqual(rows[4]["after"]["primary"]["text"], "X😀")
        self.assertEqual(rows[5]["after"]["duplicate"]["gravity"]["value"], 2)
        self.assertEqual(rows[5]["after"]["primary"]["gravity"]["value"], 4)
        self.assertEqual(rows[8]["hresult"], "0x80070057")
        self.assertEqual(rows[9]["hresult"], "0x00000001")

    def test_gravity_keeps_extreme_invalid_inputs_and_failed_getter_outputs(self) -> None:
        for value in (-(2 ** 31), -9_999_999, -1, 5, 2 ** 31 - 1):
            record = synthetic_gravity_record()
            record["gravity"][8]["value"] = value
            record["gravity"][7]["hresult"] = "0x80004001"
            record["gravity"][7]["output"] = -(2 ** 31)
            with self.subTest(value=value):
                self.assertEqual(PARSER.extract(record_log(record)), record)
        record = synthetic_gravity_record()
        record["gravity"][6]["argument1"] = 2 ** 31 - 1
        record["gravity"][6]["argument2"] = -(2 ** 31)
        self.assertEqual(PARSER.extract(record_log(record)), record)

    def test_gravity_rejects_incomplete_duplicate_or_mismatched_completion(self) -> None:
        record = synthetic_gravity_record()
        log = record_log(record)
        invalid = [
            log.rsplit("\n", 1)[0],
            log.replace("expected_commands=11", "expected_commands=10"),
            log.replace("commands=11 fixtures=1", "commands=10 fixtures=1"),
            log.replace("commands=11 fixtures=1", "commands=11 fixtures=2"),
            log + "\nSYSTEM_MSFTEDIT_GRAVITY_PROBE_COMPLETE commands=11 fixtures=1",
            log + "\nSYSTEM_MSFTEDIT_" + gravity_row(record["gravity"][-1]),
            log.replace("step=9 role=Target", "step=10 role=Target"),
        ]
        for rows in (record["gravity"][:-1], record["gravity"] + [record["gravity"][-1]],
                     [record["gravity"][1], record["gravity"][0], *record["gravity"][2:]]):
            changed = copy.deepcopy(record)
            changed["gravity"] = rows
            changed["counts"]["gravity"] = len(rows)
            changed["fixtures"][0]["expected_commands"] = len(rows)
            invalid.append(record_log(changed))
        for value in invalid:
            with self.subTest(log=value[-90:]), self.assertRaises(ValueError):
                PARSER.extract(value)

    def test_gravity_rejects_unknown_or_late_fixture_declarations(self) -> None:
        lines = record_log(synthetic_gravity_record()).splitlines()
        invalid = (
            [lines[0], lines[2], lines[1], *lines[3:]],
            [*lines, lines[1].replace("name=example_gravity", "name=late")],
            [lines[0], lines[1].replace(" expected_commands=11", ""), *lines[2:]],
            [lines[0], lines[1].replace(" window_ex_style=0", ""), *lines[2:]],
        )
        for reordered in invalid:
            with self.assertRaises(ValueError):
                PARSER.extract("\n".join(reordered))
        with self.assertRaises(ValueError):
            PARSER.extract(SYNTHETIC_OPTIONS.replace("window_ex_style=0", "window_ex_style=0 expected_commands=1"))

    def test_gravity_rejects_contradictory_ranges_utf16_and_native_widths(self) -> None:
        baseline = synthetic_gravity_record()
        for row_index, path, value in (
            (1, ("argument1",), 2 ** 31),
            (2, ("value",), -(2 ** 31) - 1),
            (7, ("output",), 2 ** 31),
            (0, ("before", "primary", "gravity", "value"), 2 ** 31),
            (0, ("before", "primary", "range", "end"), 6),
            (0, ("before", "primary", "text_utf16_length"), 1),
            (0, ("before", "story_text_utf16_length"), 4),
            (0, ("before", "story_length"), 4),
            (0, ("before", "selection", "end"), 6),
            (0, ("before", "selection", "flags"), 2 ** 31),
            (4, ("text_utf16_length",), 2),
            (4, ("action_before", "end"), 2),
            (4, ("action_after", "end"), 7),
            (5, ("action_before", "start"), 0),
            (5, ("after", "duplicate", "range", "start"), 5),
        ):
            record = copy.deepcopy(baseline)
            target = record["gravity"][row_index]
            for name in path[:-1]:
                target = target[name]
            target[path[-1]] = value
            with self.subTest(row=row_index, path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_gravity_rejects_wrong_argument_shapes_and_unrecorded_outputs(self) -> None:
        for row_index, key, value in (
            (7, "output", None), (8, "output", 4), (8, "argument1", 1),
            (8, "text", "X"), (6, "value", 1), (9, "value", 1),
            (0, "target", "Primary"), (0, "role", "Target"), (0, "value", 2),
            (0, "hresult", "0x80070057"), (7, "role", "Setup"),
            (1, "operation", "GetUnknownGravity"),
        ):
            record = synthetic_gravity_record()
            record["gravity"][row_index][key] = value
            if key == "text":
                record["gravity"][row_index]["text_utf16_length"] = len(value.encode("utf-16-le")) // 2
            with self.subTest(row=row_index, key=key), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_gravity_rejects_inconsistent_duplicate_lifetimes_and_targets(self) -> None:
        for row_index, phase, key, value in (
            (3, "before", "duplicate", synthetic_gravity_record()["gravity"][3]["after"]["duplicate"]),
            (3, "after", "duplicate", None),
            (4, "after", "duplicate", None),
            (5, "before", "duplicate", None),
        ):
            record = synthetic_gravity_record()
            record["gravity"][row_index][phase][key] = value
            with self.subTest(row=row_index, phase=phase), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))
        record = synthetic_gravity_record()
        record["gravity"][3]["hresult"] = "0x80004001"
        with self.assertRaises(ValueError):
            PARSER.extract(record_log(record))

    def test_gravity_rejects_missing_initial_configuration_or_state_continuity(self) -> None:
        for row_index, path, value in (
            (1, ("operation",), "SetStart"),
            (2, ("operation",), "Collapse"),
            (4, ("before", "primary", "bold", "value"), -1),
            (4, ("before", "story_text"), "x😀b\r"),
            (4, ("before", "selection", "flags"), 16),
        ):
            record = synthetic_gravity_record()
            target = record["gravity"][row_index]
            for name in path[:-1]:
                target = target[name]
            target[path[-1]] = value
            with self.subTest(row=row_index, path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))
        record = synthetic_gravity_record()
        record["fixtures"][0]["text"] = "x😀b"
        with self.assertRaises(ValueError):
            PARSER.extract(record_log(record))

    def test_gravity_and_link_families_coexist_without_changing_old_schema(self) -> None:
        gravity = synthetic_gravity_record()
        link = synthetic_link_record()
        record = copy.deepcopy(gravity)
        record["fixtures"].extend(link["fixtures"])
        record["link"] = link["link"]
        record["counts"].update(link=2, link_fixtures=1)
        self.assertEqual(PARSER.extract(record_log(record)), record)
        old = PARSER.extract(SYNTHETIC_OPTIONS)
        self.assertNotIn("gravity", old)
        self.assertNotIn("gravity", old["counts"])

    def test_selection_gravity_roundtrips_actual_identity_affinity_and_duplicate_results(self) -> None:
        record = synthetic_selection_gravity_record()
        actual = PARSER.extract(record_log(record))
        self.assertEqual(actual, record)
        self.assertEqual(actual["selection_gravity"][2]["after"]["primary"]["gravity"]["value"], 4)
        self.assertEqual(actual["selection_gravity"][4]["after"]["duplicate"]["gravity"]["value"], 2)
        self.assertFalse(actual["selection_gravity"][3]["duplicate_interface"]["identity"]["same_identity"])
        self.assertEqual(actual["selection_gravity"][11]["action_after"]["text"]["text"], "X")

    def test_selection_gravity_preserves_null_empty_and_native_error_outputs(self) -> None:
        record = synthetic_selection_gravity_record()
        rows = record["selection_gravity"]
        self.assertTrue(rows[7]["text_null"])
        self.assertFalse(rows[8]["text_null"])
        self.assertEqual(rows[7]["text"], rows[8]["text"])
        rows[-1]["hresult"] = "0x80004001"
        rows[-1]["output"] = -(2 ** 31)
        rows[6]["value"] = 2 ** 31 - 1
        self.assertEqual(PARSER.extract(record_log(record)), record)
        self.assertTrue(record["selection_gravity"][0]["before"]["primary"]["text"]["is_null"])
        self.assertFalse(record["selection_gravity"][5]["after"]["primary"]["text"]["is_null"])

    def test_selection_gravity_default_fixture_has_no_synthesized_mode_or_range(self) -> None:
        record = synthetic_selection_gravity_record(default=True)
        self.assertEqual(PARSER.extract(record_log(record)), record)
        self.assertEqual(record["selection_gravity"][0]["before"]["primary"]["gravity"]["value"], 2)
        for field, value in (("initialization", "Configured"), ("text", "x")):
            changed = copy.deepcopy(record)
            changed["fixtures"][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                PARSER.extract(record_log(changed))
        record["selection_gravity"][0]["text"] = "Y"
        with self.assertRaises(ValueError):
            PARSER.extract(record_log(record))

    def test_selection_gravity_rejects_unproven_interface_identity(self) -> None:
        changes = (
            (("source_null",), True), (("source_hresult",), "0x80004002"),
            (("range2", "hresult"), "0x80004002"), (("range2", "is_null"), True),
            (("range2", "identity"), None), (("range2", "identity", "same_identity"), False),
            (("selection2", "identity", "hresult"), "0x80004002"),
            (("selection2", "identity", "is_null"), True),
        )
        for path, value in changes:
            record = synthetic_selection_gravity_record()
            target = record["selection_gravity_interfaces"][0]
            for key in path[:-1]:
                target = target[key]
            target[path[-1]] = value
            with self.subTest(path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_selection_gravity_rejects_missing_duplicate_late_and_reordered_interfaces(self) -> None:
        lines = record_log(synthetic_selection_gravity_record()).splitlines()
        interface = lines[2]
        variants = (
            [*lines[:2], *lines[3:]],
            [*lines[:3], interface, *lines[3:]],
            [lines[0], interface, lines[1], *lines[3:]],
            [*lines[:2], lines[3], interface, *lines[4:]],
            [*lines, interface],
            [*lines[:2], interface.replace("fixture=selection_configured", "fixture=unknown"), *lines[3:]],
        )
        for variant in variants:
            with self.subTest(prefix=variant[:3]), self.assertRaises(ValueError):
                PARSER.extract("\n".join(variant))

    def test_selection_gravity_rejects_incomplete_duplicate_and_unbounded_command_sets(self) -> None:
        record = synthetic_selection_gravity_record()
        log = record_log(record)
        for changed in (
            log.rsplit("\n", 1)[0], log + "\n" + log.splitlines()[-1],
            log.replace("PROBE_COMPLETE commands=14", "PROBE_COMPLETE commands=13"),
            log.replace("fixtures=1", "fixtures=2"), log + "\n" + log.splitlines()[-2],
            log.replace("expected_commands=14", "expected_commands=251"),
        ):
            with self.subTest(tail=changed[-100:]), self.assertRaises(ValueError):
                PARSER.extract(changed)
        for rows in (record["selection_gravity"][:-1], [record["selection_gravity"][1], record["selection_gravity"][0], *record["selection_gravity"][2:]]):
            changed = copy.deepcopy(record)
            changed["selection_gravity"] = rows
            changed["counts"]["selection_gravity"] = len(rows)
            with self.assertRaises(ValueError):
                PARSER.extract(record_log(changed))

    def test_selection_gravity_rejects_alias_inconsistent_snapshots_and_hidden_state_changes(self) -> None:
        for row, path, value in (
            (5, ("before", "selection", "flags"), 16),
            (5, ("action_after", "gravity", "value"), 1),
            (4, ("before", "duplicate", "gravity", "value"), 2),
            (5, ("before", "primary", "flags", "hresult"), "0x80004001"),
            (2, ("before", "primary", "range", "end"), 2 ** 31),
            (3, ("before", "story_length", "hresult"), "0x80004001"),
        ):
            record = synthetic_selection_gravity_record()
            target = record["selection_gravity"][row]
            for key in path[:-1]:
                target = target[key]
            target[path[-1]] = value
            with self.subTest(row=row, path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_selection_gravity_rejects_malformed_bstr_and_scalar_width_records(self) -> None:
        for row, path, value in (
            (10, ("text_null",), True), (10, ("text_utf16_length",), 2),
            (10, ("after", "story_text", "text_utf16_length"), 5),
            (1, ("after", "primary", "text", "is_null"), True),
            (1, ("after", "primary", "gravity", "value"), -(2 ** 31) - 1),
            (6, ("value",), 2 ** 31),
            (13, ("output",), 2 ** 31),
            (11, ("action_after", "text", "text"), "XX"),
        ):
            record = synthetic_selection_gravity_record()
            target = record["selection_gravity"][row]
            for key in path[:-1]:
                target = target[key]
            target[path[-1]] = value
            with self.subTest(row=row, path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_selection_gravity_rejects_wrong_operation_arguments_and_targets(self) -> None:
        for row, field, value in (
            (2, "text_null", False), (2, "extend", True), (2, "argument1", 1),
            (5, "argument1", 5), (5, "output", None), (5, "value", 1),
            (12, "argument2", 1), (1, "operation", "GetGravity"),
            (0, "role", "Target"), (0, "value", 1), (11, "argument1", 0),
            (6, "output", 0), (1, "target", "Duplicate"),
        ):
            record = synthetic_selection_gravity_record()
            record["selection_gravity"][row][field] = value
            with self.subTest(row=row, field=field), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_selection_gravity_rejects_duplicate_creation_without_interface_provenance(self) -> None:
        for row, field, value in (
            (3, "returned_null", True), (3, "returned_null", None),
            (3, "duplicate_interface", None), (3, "hresult", "0x80004001"),
            (4, "returned_null", False), (4, "duplicate_interface", {"hresult": "0x80004002", "is_null": True, "identity": None}),
        ):
            record = synthetic_selection_gravity_record()
            record["selection_gravity"][row][field] = value
            with self.subTest(row=row, field=field), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_selection_gravity_and_measured_range_gravity_keep_separate_families(self) -> None:
        record = synthetic_gravity_record()
        selection = synthetic_selection_gravity_record()
        record["fixtures"].extend(selection["fixtures"])
        for key in ("selection_gravity_interfaces", "selection_gravity"):
            record[key] = selection[key]
        record["counts"].update(selection_gravity=len(selection["selection_gravity"]), selection_gravity_fixtures=1)
        self.assertEqual(PARSER.extract(record_log(record)), record)
        old = PARSER.extract(record_log(synthetic_gravity_record()))
        self.assertNotIn("selection_gravity", old)
        self.assertNotIn("selection_gravity_interfaces", old)
        self.assertNotIn("selection_gravity", old["counts"])


    def test_selection_gravity_extension_preserves_all_250_historical_rows(self) -> None:
        path = SCRIPT.parents[2] / "docs/porting/winui-rich-text/native-msftedit-selection-gravity.json"
        saved_bytes = path.read_bytes()
        saved = json.loads(saved_bytes)
        original = {key: value for key, value in saved.items() if key not in ("schema_version", "scope", "provenance")}
        self.assertEqual(len(original["selection_gravity"]), 250)
        combined = copy.deepcopy(original)
        extension = synthetic_selection_gravity_extension(16)
        for key in ("fixtures", "selection_gravity_interfaces", "selection_gravity"):
            combined[key].extend(extension[key])
        combined["counts"].update(selection_gravity=490, selection_gravity_fixtures=34)
        actual = PARSER.extract(record_log(combined))
        self.assertEqual(actual, combined)
        self.assertEqual(actual["selection_gravity"][:250], original["selection_gravity"])
        self.assertEqual(actual["selection_gravity_interfaces"][:18], original["selection_gravity_interfaces"])
        self.assertEqual(path.read_bytes(), saved_bytes)

    def test_selection_gravity_family_accepts_exact_budget_with_independent_defaults(self) -> None:
        record = synthetic_selection_gravity_extension(34)
        default = synthetic_selection_gravity_record(default=True)
        for key in ("fixtures", "selection_gravity_interfaces", "selection_gravity"):
            record[key].extend(default[key])
        record["counts"].update(selection_gravity=512, selection_gravity_fixtures=35)
        self.assertEqual(PARSER.extract(record_log(record)), record)

    def test_selection_gravity_family_rejects_a_real_row_over_budget(self) -> None:
        record = synthetic_selection_gravity_extension(34)
        default = synthetic_selection_gravity_record(default=True)
        for key in ("fixtures", "selection_gravity_interfaces", "selection_gravity"):
            record[key].extend(default[key])
        # The extra row is valid and consecutive inside its own configured
        # fixture; only the total family budget makes this record invalid.
        row = copy.deepcopy(record["selection_gravity"][14])
        row["step"] = 15
        record["selection_gravity"].insert(15, row)
        record["fixtures"][0]["expected_commands"] = 16
        record["counts"].update(selection_gravity=513, selection_gravity_fixtures=35)
        with self.assertRaisesRegex(ValueError, "bounded completion counts"):
            PARSER.extract(record_log(record))

    def test_selection_gravity_extension_still_requires_each_interface_and_complete_fixture(self) -> None:
        record = synthetic_selection_gravity_extension(18)
        for change in ("interface", "duplicate_identity", "missing_row"):
            changed = copy.deepcopy(record)
            if change == "interface":
                changed["selection_gravity_interfaces"].pop()
            elif change == "duplicate_identity":
                changed["fixtures"][-1]["name"] = changed["fixtures"][0]["name"]
            else:
                changed["selection_gravity"].pop(-2)
                changed["counts"]["selection_gravity"] -= 1
            with self.subTest(change=change), self.assertRaises(ValueError):
                PARSER.extract(record_log(changed))

    def test_selection_gravity_external_paragraph_reads_preserve_raw_formats_and_utf16(self) -> None:
        record = synthetic_selection_gravity_record()
        row = record["selection_gravity"][11]
        story = row["before"]["story_text"]
        row["argument1"], row["argument2"] = 0, 6
        for phase in ("action_before", "action_after"):
            row[phase]["range"] = {"start": 0, "end": 6}
            row[phase]["text"] = copy.deepcopy(story)
            row[phase]["bold"] = {"hresult": "0x00000000", "value": -9_999_999}
            row[phase]["italic"] = {"hresult": "0x80004001", "value": -(2 ** 31)}
            row[phase]["alignment"] = {"hresult": "0x00000000", "value": 2}
        self.assertEqual(PARSER.extract(record_log(record)), record)
        self.assertEqual(row["action_after"]["text"]["text"], "aX😀b\r")
        self.assertEqual(row["before"], row["after"])
        row["action_after"]["text"]["text_utf16_length"] = 5
        with self.assertRaises(ValueError):
            PARSER.extract(record_log(record))


    def test_range_tracking_roundtrips_90_raw_commands_without_predicting_mode_or_endpoints(self) -> None:
        record = synthetic_range_tracking_record(15)
        actual = PARSER.extract(record_log(record))
        self.assertEqual(actual, record)
        self.assertEqual(actual["counts"]["range_tracking"], 90)
        self.assertEqual(actual["counts"]["range_tracking_fixtures"], 15)
        self.assertEqual([actual["range_tracking"][6 * index + 1]["value"] for index in range(15)], [0] * 12 + [2] * 3)
        self.assertEqual(actual["range_tracking"][1]["after"]["primary"]["gravity"]["value"], 4)
        self.assertEqual(actual["range_tracking"][5]["output"], 4)

    def test_range_tracking_preserves_historical_families_and_490_selection_protocol(self) -> None:
        path = SCRIPT.parents[2] / "docs/porting/winui-rich-text/native-msftedit-selection-gravity.json"
        raw = path.read_bytes()
        saved = json.loads(raw)
        original = {key: value for key, value in saved.items() if key not in ("schema_version", "scope", "provenance")}
        combined = copy.deepcopy(original)
        selection_extension = synthetic_selection_gravity_extension(16)
        for key in ("fixtures", "selection_gravity_interfaces", "selection_gravity"):
            combined[key].extend(selection_extension[key])
        combined["counts"].update(selection_gravity=490, selection_gravity_fixtures=34)
        before_tracking = copy.deepcopy(combined)
        tracking = synthetic_range_tracking_record(15)
        combined["fixtures"].extend(tracking["fixtures"])
        combined["range_tracking"] = tracking["range_tracking"]
        combined["counts"].update(range_tracking=90, range_tracking_fixtures=15)
        actual = PARSER.extract(record_log(combined))
        self.assertEqual(actual, combined)
        for family in ("home_end", "vertical", "horizontal", "options", "link", "gravity", "selection_gravity", "selection_gravity_interfaces"):
            self.assertEqual(actual[family], before_tracking[family], family)
        self.assertEqual((len(actual["link"]), len(actual["gravity"]), len(actual["selection_gravity"])), (137, 1577, 490))
        self.assertNotIn("range_tracking", PARSER.extract(record_log(original)))
        self.assertEqual(path.read_bytes(), raw)

    def test_range_tracking_keeps_failed_edit_and_getter_hresult_outputs(self) -> None:
        record = synthetic_range_tracking_record()
        rows = record["range_tracking"]
        unchanged = copy.deepcopy(rows[3]["before"])
        rows[3]["hresult"] = "0x80070057"
        rows[3]["after"] = copy.deepcopy(unchanged)
        rows[3]["action_after"] = copy.deepcopy(rows[3]["action_before"])
        for row in rows[4:]:
            row["before"] = copy.deepcopy(unchanged)
            row["after"] = copy.deepcopy(unchanged)
            row["action_before"] = copy.deepcopy(unchanged[row["target"].lower()]["range"])
            row["action_after"] = copy.deepcopy(row["action_before"])
        rows[5]["hresult"], rows[5]["output"] = "0x80004001", -(2 ** 31)
        rows[1]["hresult"] = "0x00000001"
        self.assertEqual(PARSER.extract(record_log(record)), record)

    def test_range_tracking_rejects_incomplete_duplicate_and_wrong_completion_counts(self) -> None:
        log = record_log(synthetic_range_tracking_record())
        for value in (
            log.rsplit("\n", 1)[0], log + "\n" + log.splitlines()[-1],
            log.replace("PROBE_COMPLETE commands=6", "PROBE_COMPLETE commands=5"),
            log.replace("fixtures=1", "fixtures=2"),
            log + "\n" + log.splitlines()[1], log + "\n" + log.splitlines()[-2],
            log.replace("expected_commands=6", "expected_commands=7"),
        ):
            with self.subTest(tail=value[-90:]), self.assertRaises(ValueError):
                PARSER.extract(value)

    def test_range_tracking_rejects_unknown_early_and_duplicate_fixture_identity(self) -> None:
        record = synthetic_range_tracking_record(2)
        log = record_log(record)
        lines = log.splitlines()
        for value in (
            "\n".join([lines[0], lines[3], lines[1], lines[2], *lines[4:]]),
            log.replace("fixture=synthetic_tracking_0", "fixture=unknown"),
            log.replace("name=synthetic_tracking_1", "name=synthetic_tracking_0"),
        ):
            with self.assertRaises(ValueError):
                PARSER.extract(value)

    def test_range_tracking_rejects_changed_six_step_protocol_and_requested_modes(self) -> None:
        for index, field, value in (
            (0, "operation", "GetGravity"), (1, "value", 1),
            (1, "role", "Setup"), (2, "target", "External"),
            (3, "target", "Primary"), (4, "target", "Duplicate"),
            (5, "target", "Primary"), (4, "step", 5),
        ):
            record = synthetic_range_tracking_record()
            record["range_tracking"][index][field] = value
            with self.subTest(index=index, field=field), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_range_tracking_rejects_duplicate_lifetime_and_nonfresh_initial_state(self) -> None:
        for index, phase, target, value in (
            (2, "after", "duplicate", None),
            (1, "after", "duplicate", "present"),
            (0, "before", "duplicate", "present"),
            (4, "after", "duplicate", None),
        ):
            record = synthetic_range_tracking_record()
            row = record["range_tracking"][index]
            row[phase][target] = copy.deepcopy(row[phase]["primary"]) if value == "present" else value
            with self.subTest(index=index, phase=phase), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))
        record = synthetic_range_tracking_record()
        row = record["range_tracking"][0]
        row["before"]["primary"]["range"] = {"start": 1, "end": 1}
        row["action_before"] = {"start": 1, "end": 1}
        with self.assertRaises(ValueError):
            PARSER.extract(record_log(record))

    def test_range_tracking_rejects_range_action_extent_and_native_width_mismatches(self) -> None:
        for index, path, value in (
            (0, ("action_after", "end"), 4),
            (3, ("argument1",), 0),
            (3, ("action_after", "end"), 7),
            (3, ("after", "duplicate", "range", "end"), 7),
            (0, ("before", "story_length"), 2 ** 31),
            (1, ("after", "primary", "gravity", "value"), 2 ** 31),
            (5, ("output",), 2 ** 31),
        ):
            record = synthetic_range_tracking_record()
            target = record["range_tracking"][index]
            for key in path[:-1]:
                target = target[key]
            target[path[-1]] = value
            with self.subTest(index=index, path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))
        record = synthetic_range_tracking_record()
        row = record["range_tracking"][0]
        row["before"]["primary"].update(text="X", text_utf16_length=1)
        with self.assertRaisesRegex(ValueError, "endpoint extent"):
            PARSER.extract(record_log(record))

    def test_range_tracking_rejects_malformed_utf16_inputs_stories_and_output_shapes(self) -> None:
        for index, path, value in (
            (3, ("text",), "😀"), (3, ("text_utf16_length",), 2),
            (3, ("after", "story_text_utf16_length"), 5),
            (0, ("after", "primary", "text_utf16_length"), 1),
            (4, ("text",), "X"), (4, ("output",), None),
            (3, ("output",), 0), (4, ("value",), 1),
        ):
            record = synthetic_range_tracking_record()
            target = record["range_tracking"][index]
            for key in path[:-1]:
                target = target[key]
            target[path[-1]] = value
            with self.subTest(index=index, path=path), self.assertRaises(ValueError):
                PARSER.extract(record_log(record))

    def test_range_tracking_rejects_unobserved_state_between_steps(self) -> None:
        for target, field in (("primary", "bold"), ("duplicate", "gravity")):
            record = synthetic_range_tracking_record()
            record["range_tracking"][4]["before"][target][field]["value"] = -123
            with self.subTest(target=target, field=field), self.assertRaisesRegex(ValueError, "between consecutive"):
                PARSER.extract(record_log(record))

    def test_range_tracking_rejects_overbudget_or_empty_complete_families(self) -> None:
        for count in (0, 16):
            record = synthetic_range_tracking_record(count)
            log = record_log(record)
            if count == 0:
                log += "\nSYSTEM_MSFTEDIT_RANGE_TRACKING_PROBE_COMPLETE commands=0 fixtures=0"
            with self.subTest(count=count), self.assertRaisesRegex(ValueError, "bounded completion"):
                PARSER.extract(log)


if __name__ == "__main__":
    unittest.main()
