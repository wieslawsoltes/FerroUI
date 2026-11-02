import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from compact_unwind import UnsupportedUnwind
from compact_unwind_link import FRAMES, INDEX, RESERVE, main, verify


class LinkTests(unittest.TestCase):
    def setUp(self):
        self.before = (100, [(1, 4, 0, 0, 0, 100, 100, 1)],
                       {FRAMES: (1, 2, 80, 20, None), INDEX: (1, 2, 40, 40, None),
                        b".text": (1, 6, 100, 200, b"text")}, b"coverage")
        self.after = (100, [(1, 4, 0, 0, 0, 92, 100, 1)],
                      {FRAMES: (1, 2, 80, 12, None), INDEX: (1, 2, 40, 40, None),
                       RESERVE: (8, 2, 92, 8, None), b".text": (1, 6, 100, 200, b"text")}, b"coverage")

    def test_only_the_reserved_frame_bytes_can_disappear(self):
        verify(self.before, self.after, 8)
        for changed in ((1, 6, 101, 200, b"text"), (1, 6, 100, 200, b"other")):
            with self.subTest(changed=changed), self.assertRaises(UnsupportedUnwind):
                bad = copy.deepcopy(self.after)
                bad[2][b".text"] = changed
                verify(self.before, bad, 8)
        for bad in ((101, *self.after[1:]), (*self.after[:3], b"different")):
            with self.assertRaises(UnsupportedUnwind):
                verify(self.before, bad, 8)
        for segment in ((1, 7, 0, 0, 0, 92, 100, 1), (1, 4, 0, 0, 0, 91, 100, 1)):
            bad = copy.deepcopy(self.after)
            bad[1][0] = segment
            with self.assertRaises(UnsupportedUnwind):
                verify(self.before, bad, 8)

    def test_failed_candidate_preserves_complete_ordinary_output(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/"result"
            args = ["-o", str(path), "one.rcgu.o", "-pie", "-fuse-ld=lld", "-Wl,--strip-all"]
            def ordinary(command):
                self.assertEqual(command[1:], args)
                path.write_bytes(b"ordinary")
                return type("Completed", (), {"returncode": 0})
            with patch("compact_unwind_link.subprocess.run", side_effect=ordinary) as cc:
                self.assertEqual(main(args), 0)
                cc.assert_called_once()
            self.assertEqual(path.read_bytes(), b"ordinary")

    def test_linker_status_and_side_outputs_are_preserved(self):
        args = ["-o", "ignored", "one.rcgu.o", "-pie", "-fuse-ld=lld", "-s", "-Wl,-Map,ignored.map"]
        with patch("compact_unwind_link.subprocess.run") as cc, patch("compact_unwind_link.inspect") as inspect:
            cc.return_value.returncode = 1
            self.assertEqual(main(args), 1)
            cc.return_value.returncode = 0
            self.assertEqual(main(args), 0)
            self.assertEqual(cc.call_count, 2)
            inspect.assert_not_called()


if __name__ == "__main__":
    unittest.main()
