import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from attribute_runtime import classify, family, section_map, text_identity


class AttributionTest(unittest.TestCase):
    def test_aliases_overlaps_and_padding_partition_the_section(self):
        metadata = {"address": 0x1000, "bytes": 32}
        report = classify([
            "1000 10 T core::ptr::drop_glue::<A>",
            "1000 10 T core::ptr::drop_glue::<B>",
            "1008 10 T func",
            "1018 04 T core::ptr::drop_glue::<C>",
            "1018 04 T aliased",
            "2000 08 T outside",
        ], metadata)
        self.assertEqual(report["categories"], {
            "Compiler-generated destruction": 8, "Overlapping categories": 8,
            "Other Rust and native code": 8, "Aliases from multiple categories": 4,
            "No sized symbol": 4,
        })
        self.assertEqual(report["unique_ranges"], 3)
        self.assertEqual(report["alias_records"], 2)
        self.assertEqual(report["partial_overlap_bytes"], 8)

    @unittest.skipUnless(all(shutil.which(tool) for tool in ("cc", "nm", "readelf")), "requires ELF tools")
    def test_unwind_records_partition_the_linked_section(self):
        with tempfile.TemporaryDirectory() as tmp:
            executable = Path(tmp) / "fixture"
            subprocess.run(["cc", "-x", "c", "-", "-fasynchronous-unwind-tables", "-o", executable],
                           input="int f(int x) { return x + 1; } int main(void) { return f(1); }",
                           text=True, check=True)
            if executable.read_bytes()[:4] != b"\x7fELF":
                self.skipTest("requires an ELF target")
            sections = section_map(executable)
            symbols = subprocess.check_output(["nm", "-n", "-S", executable], text=True)
            report = classify(symbols.splitlines(), text_identity(executable, sections[".text"]), executable)
            unwind = report["unwind"]
            self.assertGreaterEqual(unwind["fdes_by_category"]["Other Rust and native code"], 2)
            self.assertEqual(sum(unwind["fde_bytes"].values()) + unwind["cie_and_padding_bytes"],
                             sections[".eh_frame"][2])
            if "index_bytes" in unwind:
                self.assertEqual(sum(unwind["index_bytes"].values()) + unwind["index_header_bytes"],
                                 sections[".eh_frame_hdr"][2])

    def test_receiver_and_trait_survive_generic_argument_normalization(self):
        self.assertEqual(family("<a::Entry<fn() -> Vec<u8>> as b::Source<T>>::load::<T>"),
                         "a::Entry as b::Source::load")


if __name__ == "__main__":
    unittest.main()
