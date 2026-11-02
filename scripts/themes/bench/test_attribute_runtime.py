import unittest

from attribute_runtime import classify, family


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

    def test_receiver_and_trait_survive_generic_argument_normalization(self):
        self.assertEqual(family("<a::Entry<fn() -> Vec<u8>> as b::Source<T>>::load::<T>"),
                         "a::Entry as b::Source::load")


if __name__ == "__main__":
    unittest.main()
