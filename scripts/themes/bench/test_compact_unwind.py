import struct
import unittest

from compact_unwind import compact_program, compact_records, UnsupportedUnwind


class CompactUnwindTests(unittest.TestCase):
    def test_repeated_argument_rule_can_span_several_instructions(self):
        old = bytes.fromhex("2e00 43 2e00 42 0e8001 00")
        new, redundant, scaled = compact_program(old)
        self.assertEqual(new, bytes.fromhex("2e00 45 1370"))
        self.assertEqual((redundant, scaled), (1, 1))

    def test_keep_the_first_assignment_and_each_real_change(self):
        body = bytes.fromhex("2e00 41 2e08 42 2e00")
        self.assertEqual(compact_program(body), (body, 0, 0))

    def test_only_encode_exactly_representable_shorter_offsets(self):
        # 127 has a one-byte operand. 129 is not divisible by eight.
        body = bytes.fromhex("0e7f 41 0e8101")
        self.assertEqual(compact_program(body), (body, 0, 0))
        self.assertEqual(compact_program(bytes.fromhex("0e8004")), (bytes.fromhex("1340"), 0, 1))

    def test_leave_program_containing_a_state_stack_or_expression_untouched(self):
        for op in (0x0a, 0x0b, 0x0f, 0x10):
            body = bytes.fromhex("2e00 42 2e00") + bytes([op])
            self.assertEqual(compact_program(body), (body, 0, 0))

    def test_relocation_rebasing_and_cie_distances(self):
        cie = bytes.fromhex("14000000 00000000 017a5200017810011b0c070890010000")
        def fde(start):
            body = bytearray(struct.pack("<IIIIB", 0, start + 4, 0, 100, 0))
            body.extend(bytes.fromhex("2e00 41 2e00 41 2e00 41 0e8001"))
            body.extend(bytes(-len(body) % 4))
            struct.pack_into("<I", body, 0, len(body) - 4)
            return body
        a = len(cie)
        first = fde(a)
        b = a + len(first)
        old = cie + first + fde(b) + bytes(4)
        compact, relocations, stats = compact_records(old)
        new_b = relocations[b + 8] - 8
        self.assertLess(new_b, b)
        self.assertEqual(relocations[a + 8], a + 8)
        self.assertEqual(struct.unpack_from("<I", compact, new_b + 4)[0], new_b + 4)
        self.assertEqual(compact[-4:], bytes(4))
        self.assertEqual((stats["fdes"], stats["redundant_args"], stats["scaled_offsets"]), (2, 4, 2))

    def test_refuse_unsupported_cie_before_rewriting(self):
        foreign = bytes.fromhex("14000000 00000000 017a5200017c10011b0c070890010000")
        with self.assertRaises(UnsupportedUnwind):
            compact_records(foreign)
