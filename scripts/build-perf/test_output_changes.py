"""Acknowledged output changes (label build-output-changes): compare.py reports the changes of the
generated output against the baseline and still requires each revision to repeat its own."""
import importlib.util
from pathlib import Path
import unittest


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


compare = load("compare", "compare.py")
scanner = load("scanner", "scanner.py")
UNITS = [{"package": "p", "name": "p", "kind": ["lib"], "features": [], "profile": {"opt_level": "0"}, "fresh": False}]


def row(label, generated, seconds=1.0):
    return {"label": label, "seconds": seconds, "units": UNITS, "generated": generated}


class CompareTests(unittest.TestCase):
    def test_changed_output_is_reported_and_each_revision_repeats_its_own(self):
        records = [row("baseline", {"p/a.rs": ["1"], "p/b.rs": ["2"]}), row("candidate", {"p/a.rs": ["1"], "p/b.rs": ["3"]}),
                   row("candidate", {"p/a.rs": ["1"], "p/b.rs": ["3"]}), row("baseline", {"p/a.rs": ["1"], "p/b.rs": ["2"]})]
        summary = compare.summarize(records)
        self.assertFalse(summary["same_generated_rust_and_metadata"])
        self.assertTrue(summary["generated_deterministic_per_revision"])
        self.assertEqual(summary["changed_generated_outputs"], ["p/b.rs"])

    def test_a_revision_that_does_not_repeat_its_output_is_not_deterministic(self):
        records = [row("baseline", {"p/a.rs": ["1"]}), row("candidate", {"p/a.rs": ["2"]}), row("candidate", {"p/a.rs": ["3"]})]
        self.assertFalse(compare.summarize(records)["generated_deterministic_per_revision"])


class ScannerTests(unittest.TestCase):
    def test_changed_files(self):
        self.assertEqual(scanner.changed_files({"a": "1", "b": "2"}, {"a": "1", "b": "3", "c": "4"}), ["b", "c"])


if __name__ == "__main__":
    unittest.main()
