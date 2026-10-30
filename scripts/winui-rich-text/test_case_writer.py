#!/usr/bin/env python3
"""Tests of the writer of the case tables (case_writer.py)."""

from __future__ import annotations

import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import case_writer  # noqa: E402
import native_cases  # noqa: E402
from test_native_cases import LINK_RECORD, LINK_TABLE  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
LINK = native_cases.TABLES[0]
RECORD = {
    **LINK_RECORD,
    "provenance": {"commit": "a" * 40, "probe_git_blob": "b" * 40, "run_url": "https://example.invalid/runs/1"},
    "engine": {"dll_file_version": "6.2.1.2", "windows_version": "10.0.26100"},
}
TABLE = "// An old header.\n// Source: a file that is not in the repository.\n" + LINK_TABLE.replace(
    "pub(super) const CASES",
    'pub(super) const SOURCE_SHA256: &str =\n    "00";\npub(super) const MEASURED_COMMIT: &str = "c";\n'
    'pub(super) const PROBE_GIT_BLOB: &str = "d";\npub(super) const ENGINE_VERSION: &str = "e";\npub(super) const CASES',
)


def write(record: dict, table: str = TABLE) -> str:
    return case_writer.write_table(LINK, record, "sha", "record.json", table)


def body(source: str) -> str:
    return source[case_writer.leading_comment_end(source):]


class WriterTests(unittest.TestCase):
    def test_a_table_that_agrees_keeps_its_body_and_gets_the_provenance_of_the_record(self) -> None:
        result = write(RECORD)
        self.assertIn("// Record: record.json (SHA-256 sha) of https://example.invalid/runs/1,", result)
        self.assertIn("// Msftedit 6.2.1.2 on Windows 10.0.26100, never Rust expectations.", result)
        self.assertNotIn("not in the repository", result)
        expected = body(TABLE).replace('"00"', '"sha"').replace('"c"', '"' + "a" * 40 + '"')
        expected = expected.replace('"d"', '"' + "b" * 40 + '"').replace('"e"', '"6.2.1.2"')
        self.assertEqual(body(result), expected)
        self.assertEqual(write(RECORD, result), result)

    def test_only_the_literals_that_differ_are_rewritten(self) -> None:
        record = copy.deepcopy(RECORD)
        record["link"][0]["hresult"] = "0x80070057"
        record["link"][0]["after"]["selection"]["flags"] = 81
        record["link"][0]["after"]["story_text"] = 'al"pha\r'
        result = write(record)
        self.assertIn("hresult: 0x80070057,", result)
        self.assertIn('after: NativeLinkSnapshot { range: (0, 0), story: "al\\"pha\\r", story_utf16_length: 6, text: "", text_utf16_length: 0, selection: [0, 0, 81, 0, 0] },', result)
        self.assertIn('before: NativeLinkSnapshot { range: (0, 0), story: "alpha\\r",', result)

    def test_options_become_none_or_some(self) -> None:
        record = copy.deepcopy(RECORD)
        record["link"][0]["argument"] = {"is_null": False, "text": "arg", "utf16_length": 3}
        record["link"][0]["result"] = {"is_null": True, "text": "", "utf16_length": 0}
        result = write(record)
        self.assertIn('argument: Some("arg"), result: None,', result)

    def test_added_entries_follow_the_layout_of_the_last_one_and_removed_entries_go(self) -> None:
        record = copy.deepcopy(RECORD)
        second = copy.deepcopy(record["link"][0])
        second["step"] = 1
        record["link"].append(second)
        result = write(record)
        self.assertEqual(result.count("NativeLinkCase {\nfixture:"), 2)
        self.assertIn('fixture: "link_plain_get_caret_start", step: 1, role: "Target",', result)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / native_cases.CASES_DIRECTORY).mkdir(parents=True)
            (root / native_cases.CASES_DIRECTORY / LINK.file).write_text(result, encoding="utf-8")
            self.assertEqual(native_cases.check_table(LINK, record, root), [])
        record["link"] = []
        emptied = write(record, result)
        self.assertNotIn("NativeLinkCase {", emptied)
        self.assertIn("pub(super) const CASES: &[NativeLinkCase] = &[", emptied)

    def test_rewriting_the_tables_from_the_record_they_were_taken_from_changes_only_their_headers(self) -> None:
        directory = os.environ.get("RICH_TEXT_ORACLE_RECORDS")
        path = Path(directory or ".") / "native-msftedit-e5a0778a-x64.json"
        if not directory or not path.exists():
            self.skipTest("native-msftedit-e5a0778a-x64.json is not in RICH_TEXT_ORACLE_RECORDS")
        record = json.loads(path.read_text(encoding="utf-8"))
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        for table in native_cases.TABLES:
            with self.subTest(table=table.file):
                source = (ROOT / native_cases.CASES_DIRECTORY / table.file).read_text(encoding="utf-8")
                result = case_writer.write_table(table, record, digest, path.name, source)
                if not table.provenance:
                    self.assertEqual(body(result), body(source))
                else:
                    self.assertEqual(len(body(result).splitlines()), len(body(source).splitlines()))
                self.assertEqual(case_writer.write_table(table, record, digest, path.name, result), result)


if __name__ == "__main__":
    unittest.main()
