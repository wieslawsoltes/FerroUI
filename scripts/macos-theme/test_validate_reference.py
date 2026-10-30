"""Offline, synthetic input tests. Synthetic images never become native evidence."""
from __future__ import annotations

import copy
import json
from pathlib import Path
import struct
import tempfile
import unittest
import zlib

from validate_reference import asset_path, EvidenceError, png_info, validate


def png() -> bytes:
    def chunk(kind: bytes, value: bytes) -> bytes:
        return struct.pack(">I", len(value)) + kind + value + struct.pack(">I", zlib.crc32(kind + value) & 0xffffffff)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(b"\x00\xFF\xFF\xFF\xFF")) + chunk(b"IEND", b""))


class ReferenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.path = self.root / "native-reference.json"
        self.manifest = {"schemaVersion": 2, "osVersion": "26.6.2", "reduceMotion": False,
                         "reduceTransparency": False, "increaseContrast": False, "captures": []}
        control = {"identifier": "push", "size": "regular", "enabled": True,
                   "width": 70, "height": 24, "intrinsicWidth": 70, "intrinsicHeight": 24,
                   "fittingWidth": 70, "fittingHeight": 24, "alignmentInsets": [0, 0, 0, 0]}
        for appearance in ("light", "dark"):
            for active in (True, False):
                stem = f"{appearance}-{'active' if active else 'inactive'}"
                for role in ("view", "window"):
                    (self.root / f"{stem}-{role}.png").write_bytes(png())
                self.manifest["captures"].append({
                    "appearance": appearance, "activeWindow": active, "requestedActiveWindow": active,
                    "applicationActive": True, "mainWindow": active, "scale": 1,
                    "viewImage": f"{stem}-view.png", "windowServerImage": f"{stem}-window.png",
                    "colors": {"label": {"red": 0, "green": 0, "blue": 0, "alpha": 1}},
                    "controls": [copy.deepcopy(control)],
                    "glassSurface": {"className": "NSGlassEffectView", "boundsInContent": [10, 10, 600, 64], "fullyVisible": True}})

    def report(self, scope: str = "state-coverage") -> dict:
        self.path.write_text(json.dumps(self.manifest))
        return validate(self.path, scope)

    def codes(self, scope: str = "state-coverage") -> set[str]:
        return {finding["code"] for finding in self.report(scope)["findings"]}

    def test_complete_synthetic_prerequisites_pass_but_fidelity_remains_unverified(self):
        for scope in ("inventory", "state-coverage", "optics-prerequisites"):
            with self.subTest(scope=scope):
                report = self.report(scope)
                self.assertEqual("pass", report["status"])
                self.assertEqual("unverified", report["nativeFidelity"])
                self.assertEqual(8, len(report["images"]))

    def test_recovered_failure_shape_does_not_claim_active_reference(self):
        self.manifest["schemaVersion"] = 1
        self.manifest["reduceMotion"] = True
        self.manifest["reduceTransparency"] = True
        for capture in self.manifest["captures"]:
            capture["activeWindow"] = False
            del capture["windowServerImage"]
        inventory = self.report("inventory")
        self.assertEqual("pass", inventory["status"])
        self.assertEqual("fail", self.report()["status"])
        self.assertTrue({"activation_mismatch", "duplicate_observed_state", "missing_observed_state"} <= self.codes())
        self.assertEqual("fail", self.report("optics-prerequisites")["status"])

    def test_missing_active_observation_fails(self):
        del self.manifest["captures"][0]
        self.assertIn("missing_observed_state", self.codes())

    def test_requested_state_does_not_count_as_observed(self):
        self.manifest["captures"][0]["activeWindow"] = False
        self.assertIn("activation_mismatch", self.codes())

    def test_app_activation_required_for_v2_active_capture(self):
        self.manifest["captures"][0]["applicationActive"] = False
        self.assertIn("incoherent_activation", self.codes())

    def test_main_window_required_for_v2_active_capture(self):
        self.manifest["captures"][0]["mainWindow"] = False
        self.assertIn("incoherent_activation", self.codes())

    def test_missing_activation_metadata_is_not_a_false_default(self):
        del self.manifest["captures"][0]["applicationActive"]
        self.assertIn("invalid_capture", self.codes())

    def test_accessibility_reference_is_separate_from_default_optics(self):
        for key in ("reduceMotion", "reduceTransparency", "increaseContrast"):
            with self.subTest(key=key):
                self.manifest[key] = True
                self.assertEqual("pass", self.report()["status"])
                self.assertEqual("fail", self.report("optics-prerequisites")["status"])
                self.manifest[key] = False

    def test_view_cache_alone_is_not_compositor_evidence(self):
        for capture in self.manifest["captures"]:
            capture.pop("windowServerImage")
        self.assertEqual("pass", self.report()["status"])
        self.assertEqual("fail", self.report("optics-prerequisites")["status"])

    def test_boolean_strings_rejected(self):
        self.manifest["captures"][0]["activeWindow"] = "false"
        self.assertIn("invalid_capture", self.codes())

    def test_boolean_scale_rejected(self):
        self.manifest["captures"][0]["scale"] = True
        self.assertIn("invalid_capture", self.codes())

    def test_nonfinite_metrics_rejected(self):
        self.manifest["captures"][0]["controls"][0]["width"] = float("nan")
        self.assertIn("invalid_manifest", self.codes())

    def test_zero_geometry_rejected(self):
        self.manifest["captures"][0]["controls"][0]["height"] = 0
        self.assertIn("invalid_capture", self.codes())

    def test_intrinsic_no_metric_sentinel_is_valid(self):
        self.manifest["captures"][0]["controls"][0]["intrinsicWidth"] = -1
        self.assertEqual("pass", self.report()["status"])

    def test_duplicate_controls_rejected(self):
        capture = self.manifest["captures"][0]
        capture["controls"].append(copy.deepcopy(capture["controls"][0]))
        self.assertIn("invalid_capture", self.codes())

    def test_inconsistent_control_inventory_rejected(self):
        self.manifest["captures"][0]["controls"][0]["identifier"] = "different"
        self.assertIn("inconsistent_control_inventory", self.codes())

    def test_extended_srgb_is_valid_but_alpha_is_bounded(self):
        color = self.manifest["captures"][0]["colors"]["label"]
        color["red"] = 1.1
        color["green"] = -0.03
        self.assertEqual("pass", self.report()["status"])
        color["alpha"] = 1.1
        self.assertIn("invalid_capture", self.codes())

    def test_reused_image_rejected(self):
        self.manifest["captures"][1]["viewImage"] = self.manifest["captures"][0]["viewImage"]
        self.assertIn("invalid_capture", self.codes())

    def test_missing_png_rejected(self):
        (self.root / self.manifest["captures"][0]["viewImage"]).unlink()
        self.assertIn("invalid_capture", self.codes())

    def test_unsafe_paths_rejected(self):
        for name in ("../outside.png", "/etc/passwd", "C:/outside.png", "..\\outside.png", "", "a\x00.png"):
            with self.subTest(name=name), self.assertRaises(EvidenceError):
                asset_path(self.root, name)

    def test_symlink_rejected(self):
        (self.root / "alias.png").symlink_to(self.root / "light-active-view.png")
        with self.assertRaises(EvidenceError):
            asset_path(self.root, "alias.png")

    def test_corrupt_png_crc_rejected(self):
        path = self.root / "bad.png"
        data = bytearray(png())
        data[20] ^= 1
        path.write_bytes(data)
        with self.assertRaises(EvidenceError):
            png_info(path)

    def test_truncated_and_trailing_png_rejected(self):
        path = self.root / "bad.png"
        for data in (png()[:-1], png() + b"junk", b"not an image"):
            with self.subTest(size=len(data)), self.assertRaises(EvidenceError):
                path.write_bytes(data)
                png_info(path)

    def test_later_os_is_not_macos_26_evidence(self):
        self.manifest["osVersion"] = "27.0.0"
        self.assertIn("invalid_manifest", self.codes())

    def test_duplicate_json_keys_rejected(self):
        self.path.write_text('{"schemaVersion":1,"schemaVersion":2}')
        self.assertEqual("fail", validate(self.path)["status"])

    def test_integer_overflow_is_an_evidence_error(self):
        self.manifest["captures"][0]["scale"] = 10 ** 1000
        self.assertIn("invalid_capture", self.codes())

    def test_optics_requires_measured_glass(self):
        del self.manifest["captures"][0]["glassSurface"]
        self.assertEqual("pass", self.report()["status"])
        self.assertEqual("fail", self.report("optics-prerequisites")["status"])

    def test_zero_glass_geometry_rejected(self):
        self.manifest["captures"][0]["glassSurface"]["boundsInContent"][3] = 0
        self.assertIn("invalid_capture", self.codes())

    def test_clipped_glass_cannot_validate_optics(self):
        self.manifest["captures"][0]["glassSurface"]["fullyVisible"] = False
        self.assertEqual("fail", self.report("optics-prerequisites")["status"])

    def test_report_is_deterministic(self):
        self.assertEqual(self.report(), self.report())


if __name__ == "__main__":
    unittest.main()
