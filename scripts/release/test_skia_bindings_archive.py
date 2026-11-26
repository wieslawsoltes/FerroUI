"""Tests of skia_bindings_archive.py: python3 -m unittest discover -s scripts/release -p 'test_*.py'."""

import gzip
import io
import os
import sys
import tarfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import skia_bindings_archive as archive  # noqa: E402


class ArchiveTests(unittest.TestCase):
    def test_the_binding_and_the_crates_that_use_it_agree(self):
        self.assertEqual(archive.problems(), [])

    def test_the_name_has_both_versions(self):
        name = archive.archive_name()
        self.assertTrue(name.startswith(f"skia-bindings-{archive.binding_version()}-ferroui-"), name)
        self.assertTrue(name.endswith(".tar.gz"), name)

    def test_the_archive_is_the_same_twice_and_holds_the_binding_under_one_folder(self):
        first, second = archive.archive_bytes(), archive.archive_bytes()
        self.assertEqual(first, second)
        with tarfile.open(fileobj=io.BytesIO(gzip.decompress(first))) as packed:
            names = packed.getnames()
        self.assertIn("skia-bindings/Cargo.toml", names)
        self.assertIn("skia-bindings/FERROUI-PATCH.md", names)
        self.assertTrue(all(name.startswith("skia-bindings") for name in names))


if __name__ == "__main__":
    unittest.main()
