"""Independent fixtures for the isolated dependency provenance guard."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from check_framework_patch import verify

class FrameworkPatchTests(unittest.TestCase):
    def make_fixture(self, root):
        package = root / 'vendor' / 'ferroui-base'
        package.mkdir(parents=True)
        (package / 'unchanged.rs').write_bytes(b'original source\n')
        (package / 'modified.rs').write_bytes(b'patched source\n')
        patch = root / 'vendor' / 'ferroui-base-resource-api.patch'
        patch.write_bytes(b'declared patch\n')
        digest = lambda value: hashlib.sha256(value).hexdigest()
        manifest = {'modified_files':['modified.rs'],
                    'original_sha256':{'unchanged.rs':digest(b'original source\n'), 'modified.rs':digest(b'old source\n')},
                    'modified_sha256':{'modified.rs':digest(b'patched source\n')},
                    'patch_sha256':digest(patch.read_bytes())}
        (root / 'vendor' / 'ferroui-base-provenance.json').write_text(json.dumps(manifest))
        return package

    def test_exact_package_accepts_only_the_declared_change(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.make_fixture(root)
            self.assertEqual(verify(root), [])

    def test_separate_markup_package_uses_its_own_provenance_and_patch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = self.make_fixture(root)
            vendor = root / 'vendor'
            package.rename(vendor / 'ferroui-markup-xaml')
            manifest_path = vendor / 'ferroui-base-provenance.json'
            manifest = json.loads(manifest_path.read_text())
            manifest['patch_file'] = 'ferroui-markup-xaml-instance-check.patch'
            (vendor / 'ferroui-base-resource-api.patch').rename(vendor / manifest['patch_file'])
            manifest_path.rename(vendor / 'ferroui-markup-xaml-provenance.json')
            (vendor / 'ferroui-markup-xaml-provenance.json').write_text(json.dumps(manifest))
            self.assertEqual(verify(root, 'ferroui-markup-xaml'), [])
            (vendor / manifest['patch_file']).write_bytes(b'unrecorded patch')
            self.assertTrue(verify(root, 'ferroui-markup-xaml'))

    def test_missing_modified_extra_and_patch_tampering_are_rejected(self):
        for change in ['missing', 'changed-original', 'changed-patch-target', 'extra', 'patch']:
            with self.subTest(change=change), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                package = self.make_fixture(root)
                if change == 'missing':
                    (package / 'unchanged.rs').unlink()
                elif change == 'changed-original':
                    (package / 'unchanged.rs').write_bytes(b'unexpected change')
                elif change == 'changed-patch-target':
                    (package / 'modified.rs').write_bytes(b'additional change')
                elif change == 'extra':
                    (package / 'unreviewed.rs').write_bytes(b'extra source')
                else:
                    (root / 'vendor' / 'ferroui-base-resource-api.patch').write_bytes(b'different patch')
                self.assertTrue(verify(root))

if __name__ == '__main__':
    unittest.main()
