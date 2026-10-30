"""Compare the case tables of the editor's tests with a record of the system Msftedit.

The case tables (`src/FerroUI.Controls/rich_text/*_native_cases.rs`) are literal observations of
the system Msftedit taken from records of the oracle (`src/tools/RichTextOracle`). The records are
not committed (docs/porting/winui-rich-text.md, section 8, decision D11): a table names the record
it was taken from and that record's hash, and this module checks a table against a fresh record,
the output of `rich-text-tom-observations.py extract` on a new run of the oracle.

For each table, `project_*` derives from the record the values the table holds, in the form
`rust_literals` reads the table in, and `check` reports every entry that differs. Only the fields
a table replays are compared; a record holds more.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path as FilePath
from typing import Callable

import rust_literals
from rust_literals import Call, Path, Struct


CASES_DIRECTORY = FilePath("src/FerroUI.Controls/rich_text")


def hresult(value: str) -> int:
    return int(value, 16)


def plain(value):
    """A parsed Rust value as plain, comparable Python data."""
    if isinstance(value, list):
        return [plain(item) for item in value]
    if isinstance(value, Struct):
        return {"struct": value.name, **{key: plain(item) for key, item in value.fields.items()}}
    if isinstance(value, Call):
        return {"call": value.name, "args": [plain(item) for item in value.args]}
    if isinstance(value, Path):
        return {"path": value.name}
    return value


NONE = Path("None")


def fixtures_of(record: dict, family: str) -> dict:
    return {fixture["name"]: fixture for fixture in record["fixtures"] if fixture["family"] == family}


def selection_list(selection: dict) -> list:
    return [selection["start"], selection["end"], selection["flags"], selection["anchor"], selection["active"]]


# Links: rich_text_link_native_cases.rs (CASES).

def link_snapshot(snapshot: dict) -> Struct:
    return Struct("NativeLinkSnapshot", {
        "range": [snapshot["range"]["start"], snapshot["range"]["end"]],
        "story": snapshot["story_text"],
        "story_utf16_length": snapshot["story_text_utf16_length"],
        "text": snapshot["range_text"],
        "text_utf16_length": snapshot["range_text_utf16_length"],
        "selection": selection_list(snapshot["selection"]),
    })


def project_links(record: dict) -> dict:
    fixtures = fixtures_of(record, "link")
    cases = []
    for row in record.get("link", []):
        cases.append(Struct("NativeLinkCase", {
            "fixture": row["fixture"],
            "step": row["step"],
            "role": row["role"],
            "initial_text": fixtures[row["fixture"]]["text"],
            "operation": row["operation"],
            "argument": None if row["argument"]["is_null"] else row["argument"]["text"],
            "result": None if row["result"]["is_null"] else row["result"]["text"],
            "hresult": hresult(row["hresult"]),
            "before": link_snapshot(row["before"]),
            "after": link_snapshot(row["after"]),
        }))
    return {"CASES": cases}


# Passive ranges across SetText: rich_edit_text_range_tracking_native_cases.rs. ORIGINAL holds the
# gravity fixtures of the form SetRange, SetGravity, Duplicate, SetText; ADDITIONAL the fixtures of
# the range tracking family. One case per fixture, from its SetText step.

TRACKING_PROTOCOL = ["SetRange", "SetGravity", "Duplicate", "SetText"]


def rows_by_fixture(rows: list[dict]) -> dict[str, list[dict]]:
    result: dict[str, list[dict]] = {}
    for row in rows:
        result.setdefault(row["fixture"], []).append(row)
    return result


def range_triple(snapshot: dict, which: str) -> list:
    value = snapshot[which]
    return [value["range"]["start"], value["range"]["end"], value["gravity"]["value"]]


def tracking_case(name: str, rows: list[dict]) -> Struct:
    set_range, set_gravity, _, set_text = rows[:4]
    return Struct("NativeCase", {
        "name": name,
        "requested_range": [set_range["argument1"], set_range["argument2"]],
        "requested_gravity": set_gravity["value"],
        "primary_before": range_triple(set_text["before"], "primary"),
        "duplicate_before": range_triple(set_text["before"], "duplicate"),
        "edit": [set_text["argument1"], set_text["argument2"]],
        "text": set_text["text"],
        "text_length": set_text["text_utf16_length"],
        "hresult": hresult(set_text["hresult"]),
        "primary_after": range_triple(set_text["after"], "primary"),
        "duplicate_after": range_triple(set_text["after"], "duplicate"),
        "action_after": [set_text["action_after"]["start"], set_text["action_after"]["end"]],
        "story": set_text["after"]["story_text"],
    })


def project_range_tracking(record: dict) -> dict:
    original = [
        tracking_case(name, rows)
        for name, rows in rows_by_fixture(record.get("gravity", [])).items()
        if [row["operation"] for row in rows[:4]] == TRACKING_PROTOCOL
    ]
    additional = [
        tracking_case(name, rows)
        for name, rows in rows_by_fixture(record.get("range_tracking", [])).items()
    ]
    return {"ORIGINAL": original, "ADDITIONAL": additional}


# Gravity, formatting affinity and the retention of ranges:
# rich_edit_text_range_gravity_native_cases.rs.

def primary_format(snapshot: dict) -> list:
    primary = snapshot["primary"]
    return [primary["bold"]["value"], primary["italic"]["value"], primary["alignment"]["value"]]


def first(rows: list[dict], operation: str, role: str = "Target") -> dict:
    return next(row for row in rows if row["operation"] == operation and row["role"] == role)


def affinity_case(name: str, text: str, rows: list[dict]) -> Struct:
    split = next(
        (row["argument2"] for row in rows if row["role"] == "Setup" and row["operation"] == "SetBold" and row["value"] == -1),
        0,
    )
    set_text = first(rows, "SetText")
    return Struct("AffinityCase", {
        "name": name,
        "text": text,
        "split": split,
        "position": first(rows, "SetRange")["argument1"],
        "requested": first(rows, "SetGravity")["value"],
        "gravity": first(rows, "GetGravity")["output"],
        "before": primary_format(set_text["before"]),
        "after": primary_format(set_text["after"]),
        "story": set_text["after"]["story_text"],
    })


def retention_operation(row: dict):
    operation = row["operation"]
    if operation == "SetRange":
        return Call("SetRange", [row["argument1"], row["argument2"]])
    if operation == "SetGravity":
        return Call("SetGravity", [row["value"]])
    if operation in ("SetStart", "SetEnd"):
        return Call(operation, [row["argument1"]])
    if operation == "Collapse":
        return Call("Collapse", [row["value"] != 0])
    if operation == "SetText":
        return Call("SetText", [row["text"]])
    return Path(operation)


def retention_case(name: str, rows: list[dict]) -> Struct:
    commands = []
    for row in rows:
        duplicate = row["after"]["duplicate"]
        commands.append(Struct("RetentionCommand", {
            "step": row["step"],
            "duplicate_target": row["target"] == "Duplicate",
            "operation": retention_operation(row),
            "hresult": hresult(row["hresult"]),
            "primary": range_triple(row["after"], "primary"),
            "duplicate": None if duplicate is None else range_triple(row["after"], "duplicate"),
        }))
    return Struct("RetentionCase", {"name": name, "commands": commands})


def invalid_case(name: str, rows: list[dict]) -> Struct:
    set_range = first(rows, "SetRange")
    rejected = [row for row in rows if row["operation"] == "SetGravity"][1]
    return Struct("InvalidCase", {
        "name": name,
        "start": set_range["argument1"],
        "end": set_range["argument2"],
        "value": rejected["value"],
        "hresult": hresult(rejected["hresult"]),
        "expected": range_triple(rejected["after"], "primary"),
    })


def project_range_gravity(record: dict) -> dict:
    fixtures = fixtures_of(record, "gravity")
    rows = rows_by_fixture(record.get("gravity", []))
    return {
        "AFFINITY": [
            affinity_case(name, fixtures[name]["text"], fixture_rows)
            for name, fixture_rows in rows.items() if "_affinity_" in name
        ],
        "RETENTION": [
            retention_case(name, fixture_rows)
            for name, fixture_rows in rows.items() if "_retention_" in name
        ],
        "INVALID": [
            invalid_case(name, fixture_rows)
            for name, fixture_rows in rows.items() if name.startswith("gravity_invalid_")
        ],
    }


# Selection options and TypeText: rich_edit_text_selection_options_native_cases.rs.

def options_state(state: dict) -> list:
    return [state["anchor"], state["active"], state["flags"], state["selection_type"]]


def project_selection_options(record: dict) -> dict:
    fixtures = list(fixtures_of(record, "options").values())
    index = {fixture["name"]: position for position, fixture in enumerate(fixtures)}
    return {
        "NATIVE_FIXTURES": [[fixture["name"], fixture["text"]] for fixture in fixtures],
        "NATIVE_CASES": [
            Call("native", [
                index[row["fixture"]],
                options_state(row["before"]),
                row["requested_flags"],
                options_state(row["after_set"]),
                row["typed"],
                options_state(row["after"]),
                row["story_text"],
            ])
            for row in record.get("options", [])
        ],
    }


# The live selection: rich_edit_text_selection_gravity_native_cases.rs holds the selection gravity
# fixtures, rich_edit_text_paragraph_typing_native_cases.rs the paragraph TypeText fixtures (the
# names `selection_paragraph_typing_*`) of the same family.

PARAGRAPH_TYPING = "selection_paragraph_typing_"


def selection_state(snapshot: dict) -> Call:
    primary = snapshot["primary"]
    duplicate = snapshot["duplicate"]
    return Call("state", [
        snapshot["selection"]["anchor"],
        snapshot["selection"]["active"],
        primary["gravity"]["value"],
        primary["flags"]["value"],
        primary["selection_type"]["value"],
        primary["bold"]["value"],
        primary["italic"]["value"],
        primary["alignment"]["value"],
        None if duplicate is None else duplicate["gravity"]["value"],
        snapshot["story_text"]["text"],
    ])


def selection_target(row: dict):
    if row["target"] == "External":
        return Call("External", [row["argument1"], row["argument2"]])
    return Path(row["target"])


def selection_operation(row: dict):
    operation = row["operation"]
    if operation == "SetGravity":
        return Call(operation, [row["value"]])
    if operation in ("SetBold", "SetItalic", "SetAlignment"):
        return Call(operation, [row["value"]])
    if operation == "SetRange":
        return Call(operation, [row["argument1"], row["argument2"]])
    if operation == "TypeText":
        return Call(operation, [None if row["text_null"] else row["text"]])
    if operation in ("MoveLeft", "MoveRight", "MoveUp", "MoveDown"):
        return Call(operation, [row["argument1"], row["argument2"], row["extend"]])
    if operation == "EndKey":
        return Call(operation, [row["argument1"], row["extend"]])
    return Path(operation)


def selection_fixture(fixture: dict, rows: list[dict]) -> Struct:
    commands = []
    for row in rows:
        action = row["action_after"]
        commands.append(Call("command", [
            row["step"],
            row["role"] == "Setup",
            selection_target(row),
            selection_operation(row),
            hresult(row["hresult"]),
            row["output"],
            selection_state(row["before"]),
            selection_state(row["after"]),
            [action["bold"]["value"], action["italic"]["value"], action["alignment"]["value"]],
        ]))
    return Struct("NativeFixture", {
        "name": fixture["name"],
        "text": fixture["text"],
        "supplied_length": fixture["supplied_utf16_length"],
        "story_length": fixture["native_story_length"],
        "commands": commands,
    })


def selection_fixtures(record: dict, paragraph_typing: bool) -> list[Struct]:
    fixtures = fixtures_of(record, "selection_gravity")
    return [
        selection_fixture(fixtures[name], rows)
        for name, rows in rows_by_fixture(record.get("selection_gravity", [])).items()
        if name.startswith(PARAGRAPH_TYPING) == paragraph_typing
    ]


def project_selection_gravity(record: dict) -> dict:
    return {"FIXTURES": selection_fixtures(record, paragraph_typing=False)}


def project_paragraph_typing(record: dict) -> dict:
    return {"FIXTURES": selection_fixtures(record, paragraph_typing=True)}


@dataclass(frozen=True)
class Table:
    file: str
    project: Callable[[dict], dict]
    # Constants of the table that state where it came from, not what was observed.
    provenance: tuple[str, ...] = ()


TABLES = (
    Table(
        "rich_text_link_native_cases.rs",
        project_links,
        ("SOURCE_SHA256", "MEASURED_COMMIT", "PROBE_GIT_BLOB", "ENGINE_VERSION"),
    ),
    Table("rich_edit_text_range_tracking_native_cases.rs", project_range_tracking),
    Table("rich_edit_text_range_gravity_native_cases.rs", project_range_gravity),
    Table("rich_edit_text_selection_options_native_cases.rs", project_selection_options),
    Table("rich_edit_text_selection_gravity_native_cases.rs", project_selection_gravity),
    Table("rich_edit_text_paragraph_typing_native_cases.rs", project_paragraph_typing),
)


def normalize_table(table_constants: dict) -> dict:
    """Option values in the table are `Some(x)` or `None`; compare them as values."""
    def walk(value):
        if isinstance(value, list):
            return [walk(item) for item in value]
        if isinstance(value, Struct):
            return Struct(value.name, {key: walk(item) for key, item in value.fields.items()})
        if isinstance(value, Call):
            if value.name == "Some" and len(value.args) == 1:
                return walk(value.args[0])
            return Call(value.name, [walk(item) for item in value.args])
        if value == NONE:
            return None
        return value
    return {key: walk(value) for key, value in table_constants.items()}


def describe_difference(path: str, expected, actual, out: list[str], limit: int) -> None:
    if len(out) >= limit:
        return
    expected_plain, actual_plain = plain(expected), plain(actual)
    if expected_plain == actual_plain:
        return
    if isinstance(expected_plain, list) and isinstance(actual_plain, list) and len(expected) == len(actual):
        for index, (left, right) in enumerate(zip(expected, actual)):
            describe_difference(f"{path}[{index}]", left, right, out, limit)
        return
    if isinstance(expected, Struct) and isinstance(actual, Struct) and expected.fields.keys() == actual.fields.keys():
        for key in expected.fields:
            describe_difference(f"{path}.{key}", expected.fields[key], actual.fields[key], out, limit)
        return
    if isinstance(expected, Call) and isinstance(actual, Call) and expected.name == actual.name and len(expected.args) == len(actual.args):
        for index, (left, right) in enumerate(zip(expected.args, actual.args)):
            describe_difference(f"{path}({index})", left, right, out, limit)
        return
    if isinstance(expected_plain, list) and isinstance(actual_plain, list):
        out.append(f"{path}: the record has {len(expected_plain)} entries, the table {len(actual_plain)}")
        return
    out.append(f"{path}: the record has {expected_plain!r}, the table {actual_plain!r}")


def check_table(table: Table, record: dict, root: FilePath, limit: int = 20) -> list[str]:
    source = (root / CASES_DIRECTORY / table.file).read_text(encoding="utf-8")
    actual = normalize_table(rust_literals.constants(source))
    expected = table.project(record)
    differences: list[str] = []
    for name, value in expected.items():
        if name not in actual:
            differences.append(f"{name}: not in the table")
            continue
        describe_difference(name, value, actual[name], differences, limit)
    return differences


def check(record: dict, root: FilePath, limit: int = 20) -> dict[str, list[str]]:
    return {table.file: check_table(table, record, root, limit) for table in TABLES}
