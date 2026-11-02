import unittest

from profile_winui import executed_definitions


class ProfileCoverageTests(unittest.TestCase):
    def test_counts_unique_definitions_in_llvm_covered_output(self):
        base = "_RNvCs_12ferroui_base8executed"
        controls = "_RNvCs1234_16ferroui_controls8executed"
        counts = executed_definitions([base, f"example.cgu.0;{base}", controls, "Counters:"])
        self.assertEqual(counts["ferroui_base"], 1)
        self.assertEqual(counts["ferroui_controls"], 1)
        self.assertEqual(sum(counts.values()), 2)

    def test_generic_argument_does_not_count_as_defining_crate(self):
        counts = executed_definitions(["_RINvCs1234_3std5equalNtCs1_16ferroui_controls6ButtonE"])
        self.assertEqual(sum(counts.values()), 0)


if __name__ == "__main__":
    unittest.main()
