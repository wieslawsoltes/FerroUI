"""Tests of set_baseline.py: python3 -m unittest discover -s scripts/api-gate -p 'test_*.py'."""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import set_baseline  # noqa: E402

GATE_TEXT = '''import os

# The release the gate compares with, and the commit of its sources: the tag v0.5.0 (2026-11-14).
# More text that stays.
BASELINE_VERSION = "0.5.0"
BASELINE_COMMIT = "d503df2befa756e681fcdc6250e9552dc7388bea"
'''


class RewriteTests(unittest.TestCase):
    def test_the_three_places_move_together(self):
        commit = "a" * 40
        text = set_baseline.rewrite(GATE_TEXT, "0.6.0", commit, "2026-12-01")
        self.assertIn('BASELINE_VERSION = "0.6.0"', text)
        self.assertIn(f'BASELINE_COMMIT = "{commit}"', text)
        self.assertIn("the tag v0.6.0 (2026-12-01).", text)
        self.assertIn("# More text that stays.", text)

    def test_a_gate_without_a_place_is_refused(self):
        with self.assertRaises(ValueError):
            set_baseline.rewrite(GATE_TEXT.replace("BASELINE_VERSION", "OTHER"), "0.6.0", "a" * 40, None)

    def test_the_gate_of_the_repository_has_every_place(self):
        with open(set_baseline.GATE, encoding="utf-8") as handle:
            text = handle.read()
        version = set_baseline.VERSION.search(text).group(1)
        commit = set_baseline.COMMIT.search(text).group(1)
        self.assertEqual(set_baseline.rewrite(text, version, commit, None).count(commit), text.count(commit))


if __name__ == "__main__":
    unittest.main()
