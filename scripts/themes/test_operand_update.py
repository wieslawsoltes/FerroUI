import hashlib
import json
from pathlib import Path
import struct
from tempfile import TemporaryDirectory
import unittest

from operand_update import fingerprint, publish


def generation(dir_, *, source=b"native caller", value=1, wire_version=1, options=b"(false, true)\n"):
    dir_.mkdir()
    module, prefix = b"theme::xaml", b"xaml"
    name = "xaml-000000000000000a.fxb"
    # One color row keyed A, with the same declaration/value position.
    header = b"FXB\0" + bytes((wire_version, 6))
    pool = b"\0\1\1\0\2A" if wire_version == 3 else b"\1\1\0\1A"
    leaf = header + pool + b"\0" + struct.pack("<I", value) + b"\1\1\0\0"
    meta = {"module": module.decode(), "version": 1, "caller": fingerprint(source),
            "blocks": {name: [6, fingerprint(leaf)]}}
    manifest = dir_ / "xaml.fxb.json"
    manifest.write_text(json.dumps(meta))
    (dir_ / "xaml.rs").write_bytes(source)
    (dir_ / "xaml.fxp-options.rs").write_bytes(options)
    (dir_ / name).write_bytes(leaf)
    def field(b):
        return len(b).to_bytes(4, "little") + b
    slot = struct.pack("<QHBI", 10, 0, 6, 0)
    digest = hashlib.sha256(b"FerroUI/FXU/1" + b"".join(field(v) for v in (module, prefix, source, options))
                            + slot + field(leaf)).digest()
    schema = b"FXS\1" + digest + field(module) + field(prefix) + b"\1\0\0\0" + slot
    (dir_ / "xaml.fxs").write_bytes(schema)
    return manifest


class OperandUpdateTests(unittest.TestCase):
    def setUp(self):
        temp = TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.dir = Path(temp.name)
        self.old = generation(self.dir / "old")
        self.output = self.dir / "update.fxu"

    def test_empty_and_changed_updates_bind_to_baseline(self):
        self.assertEqual(publish(self.old, self.old, self.output), (0, 72))
        new = generation(self.dir / "new", value=5, wire_version=3)
        count, size = publish(self.old, new, self.output)
        data = self.output.read_bytes()
        self.assertEqual((count, size), (1, len(data)))
        self.assertEqual(data[:4], b"FXU\1")
        self.assertEqual(data[4:36], hashlib.sha256((self.old.parent / "xaml.fxs").read_bytes()).digest())
        self.assertEqual(data[-32:], hashlib.sha256(data[:-32]).digest())
        leaf = (new.parent / "xaml-000000000000000a.fxb").read_bytes()
        self.assertEqual(data[36:-32], struct.pack("<III", 1, 0, len(leaf)) + leaf)

    def test_reject_incompatible_generation_and_preserve_published_update(self):
        self.output.write_bytes(b"last good")
        for i, kwargs in enumerate(({"source": b"new code"}, {"options": b"(true, true)\n"})):
            new = generation(self.dir / str(i), **kwargs)
            with self.assertRaisesRegex(ValueError, "native caller"):
                publish(self.old, new, self.output)
            self.assertEqual(self.output.read_bytes(), b"last good")

    def test_reject_torn_schema_leaf_caller_and_duplicate_manifest_keys(self):
        new = generation(self.dir / "new")
        for name in ["xaml.fxs", "xaml-000000000000000a.fxb", "xaml.rs"]:
            path = new.parent / name
            data = path.read_bytes()
            path.write_bytes(data + b"!")
            with self.assertRaises(ValueError):
                publish(self.old, new, self.output)
            path.write_bytes(data)
        new.write_text('{"version":1,"version":1}')
        with self.assertRaisesRegex(ValueError, "duplicate"):
            publish(self.old, new, self.output)
        self.assertFalse(self.output.exists())

    def test_protect_inputs_and_reject_manifest_traversal(self):
        source = self.old.parent / "xaml.rs"
        with self.assertRaises(ValueError):
            publish(self.old, self.old, source)
        self.output.symlink_to(source)
        with self.assertRaises(ValueError):
            publish(self.old, self.old, self.output)
        self.assertEqual(source.read_bytes(), b"native caller")
        new = generation(self.dir / "new")
        data = json.loads(new.read_text())
        data["blocks"] = {"../outside.fxb": [1, "0" * 16]}
        new.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, "slot"):
            publish(self.old, new, self.dir / "invalid.fxu")


if __name__ == "__main__":
    unittest.main()
