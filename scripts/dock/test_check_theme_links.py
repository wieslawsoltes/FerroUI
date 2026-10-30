"""Failure-path coverage of the upstream Simple project's linked resources."""
from pathlib import Path
import tempfile
import unittest
from check_theme_links import check_links, FLUENT_URI, SIMPLE_URI


class ThemeLinksTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        root = Path(self.temporary.name)
        self.fluent, self.simple = root / 'fluent', root / 'simple'
        for theme in [self.fluent, self.simple]:
            (theme / 'Controls').mkdir(parents=True)
            (theme / 'Assets').mkdir()
            (theme / 'Assets/target.png').write_bytes(b'original-image-bytes')
        (self.fluent / 'Controls/Target.xaml').write_text(FLUENT_URI + 'Assets/target.png')
        (self.simple / 'Controls/Target.xaml').write_text(SIMPLE_URI + 'Assets/target.png')

    def test_original_links_and_only_their_assembly_owner_migration_are_accepted(self):
        self.assertEqual(check_links(self.fluent, self.simple), [])

    def test_missing_extra_and_wrong_owner_inputs_are_rejected(self):
        (self.simple / 'Controls/Target.xaml').write_text(FLUENT_URI + 'Assets/target.png')
        (self.simple / 'Controls/Extra.xaml').write_text('extra')
        (self.simple / 'Assets/target.png').unlink()
        self.assertEqual(check_links(self.fluent, self.simple), [
            'extra linked file: Controls/Extra.xaml',
            'changed linked file: Controls/Target.xaml',
            'missing linked file: Assets/target.png',
        ])

    def test_changed_original_image_bytes_are_rejected(self):
        (self.simple / 'Assets/target.png').write_bytes(b'different-image-bytes')
        self.assertEqual(check_links(self.fluent, self.simple), ['changed linked file: Assets/target.png'])
