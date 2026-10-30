#!/usr/bin/env python3
"""Tests of the reader of the case tables and of their check against a record of the oracle."""

from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import native_cases  # noqa: E402
import rust_literals  # noqa: E402
from rust_literals import Call, Path as RustPath, Struct  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]


class RustLiteralTests(unittest.TestCase):
    def test_constants_of_literals_structs_calls_and_paths(self) -> None:
        source = r'''
            //! A table.
            use super::{command, NativeCase};
            /* a block comment with a const X: u8 = 1; inside */
            pub(super) const NAME: &str = "a\"b\\c\r\u{200d}é";
            pub(super) const NUMBERS: &[(i32, u32)] = &[(-1, 0x8000_0000), (2_147_483_647, 0u32)];
            #[rustfmt::skip]
            pub(super) const CASES: &[NativeCase] = &[
                NativeCase { name: "x", target: NativeTarget::External(0, 4), mode: Primary, value: Some(-3), none: None, flag: true, },
                command(1, false, (2, 3), Some("t"),),
            ];
            const fn helper(value: i32) -> i32 { value }
        '''
        values = rust_literals.constants(source)
        self.assertEqual(values["NAME"], 'a"b\\c\r‍é')
        self.assertEqual(values["NUMBERS"], [[-1, 0x80000000], [2147483647, 0]])
        case, call = values["CASES"]
        self.assertEqual(case, Struct("NativeCase", {
            "name": "x",
            "target": Call("External", [0, 4]),
            "mode": RustPath("Primary"),
            "value": Call("Some", [-3]),
            "none": RustPath("None"),
            "flag": True,
        }))
        self.assertEqual(call, Call("command", [1, False, [2, 3], Call("Some", ["t"])]))
        self.assertNotIn("X", values)

    def test_every_case_table_reads_and_holds_what_its_check_compares(self) -> None:
        empty = {"fixtures": []}
        for table in native_cases.TABLES:
            with self.subTest(table=table.file):
                source = (ROOT / native_cases.CASES_DIRECTORY / table.file).read_text(encoding="utf-8")
                values = rust_literals.constants(source)
                for name in table.project(empty):
                    self.assertIsInstance(values.get(name), list, name)
                    self.assertTrue(values[name], name)


LINK_TABLE = '''
pub(super) const CASES: &[NativeLinkCase] = &[
NativeLinkCase {
fixture: "link_plain_get_caret_start", step: 0, role: "Target",
initial_text: "alpha", operation: "Get",
argument: None, result: Some("x"), hresult: 0x00000001,
before: NativeLinkSnapshot { range: (0, 0), story: "alpha\\r", story_utf16_length: 6, text: "", text_utf16_length: 0, selection: [0, 0, 17, 0, 0] },
after: NativeLinkSnapshot { range: (0, 0), story: "alpha\\r", story_utf16_length: 6, text: "", text_utf16_length: 0, selection: [0, 0, 17, 0, 0] },
},
];
'''


def link_snapshot() -> dict:
    return {
        "range": {"start": 0, "end": 0},
        "range_text": "",
        "range_text_utf16_length": 0,
        "story_text": "alpha\r",
        "story_text_utf16_length": 6,
        "story_length": 6,
        "selection": {"start": 0, "end": 0, "flags": 17, "anchor": 0, "active": 0},
    }


LINK_RECORD = {
    "fixtures": [{"family": "link", "name": "link_plain_get_caret_start", "text": "alpha"}],
    "link": [{
        "fixture": "link_plain_get_caret_start",
        "step": 0,
        "role": "Target",
        "operation": "Get",
        "argument": {"is_null": True, "text": "", "utf16_length": 0},
        "result": {"is_null": False, "text": "x", "utf16_length": 1},
        "before": link_snapshot(),
        "after": link_snapshot(),
        "hresult": "0x00000001",
    }],
}


class CheckTests(unittest.TestCase):
    def check_link_table(self, record: dict) -> list[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / native_cases.CASES_DIRECTORY).mkdir(parents=True)
            (root / native_cases.CASES_DIRECTORY / native_cases.TABLES[0].file).write_text(LINK_TABLE, encoding="utf-8")
            return native_cases.check_table(native_cases.TABLES[0], record, root)

    def test_a_table_that_holds_the_observation_agrees(self) -> None:
        self.assertEqual(self.check_link_table(LINK_RECORD), [])

    def test_each_changed_observation_is_reported_with_its_place(self) -> None:
        record = copy.deepcopy(LINK_RECORD)
        record["link"][0]["hresult"] = "0x80070057"
        record["link"][0]["after"]["selection"]["flags"] = 81
        self.assertEqual(self.check_link_table(record), [
            "CASES[0].hresult: the record has 2147942487, the table 1",
            "CASES[0].after.selection[2]: the record has 81, the table 17",
        ])

    def test_a_null_and_an_empty_string_differ(self) -> None:
        record = copy.deepcopy(LINK_RECORD)
        record["link"][0]["result"] = {"is_null": True, "text": "", "utf16_length": 0}
        self.assertEqual(self.check_link_table(record), ["CASES[0].result: the record has None, the table 'x'"])

    def test_missing_and_added_observations_are_reported(self) -> None:
        record = copy.deepcopy(LINK_RECORD)
        record["link"].append(copy.deepcopy(record["link"][0]))
        self.assertEqual(self.check_link_table(record), ["CASES: the record has 2 entries, the table 1"])

    def test_the_tables_agree_with_the_record_they_were_taken_from(self) -> None:
        # #243's record of 2026-10 (Msftedit 6.2.26100.33648, x64); every table on main was taken
        # from it or from an earlier record whose observations it repeats.
        directory = os.environ.get("RICH_TEXT_ORACLE_RECORDS")
        path = Path(directory or ".") / "native-msftedit-e5a0778a-x64.json"
        if not directory or not path.exists():
            self.skipTest("native-msftedit-e5a0778a-x64.json is not in RICH_TEXT_ORACLE_RECORDS")
        record = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(native_cases.check(record, ROOT), {table.file: [] for table in native_cases.TABLES})


if __name__ == "__main__":
    unittest.main()
