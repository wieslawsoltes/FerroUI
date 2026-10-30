"""Tests of the Windows source paths of `winui_sources.py` on a case-sensitive file system (Linux).

The project of WinUI imports `dev\\InkToolbar\\InkToolbar.vcxitems`, which git stores as
`InkToolBar/InkToolBar.vcxitems`.

    python3 -m unittest discover -s scripts/winui-theme -p 'test_*.py'
"""

from pathlib import Path
import tempfile
import unittest

import winui_sources as sources


class WindowsProjectPathTests(unittest.TestCase):
    def test_mixed_case_project_and_page_keep_inventory_and_waiver_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            actual = root / "controls/dev/InkToolBar"
            actual.mkdir(parents=True)
            (root / "controls/ProjectImports.targets").write_text(
                '<Project><Import Project="$(MSBuildThisFileDirectory)'
                'dev\\InkToolbar\\InkToolbar.vcxitems" /></Project>',
                encoding="utf-8",
            )
            (actual / "InkToolBar.vcxitems").write_text(
                '<Project><Page Include="$(MSBuildThisFileDirectory)InkToolbar.xaml">'
                '<Type>DefaultStyle</Type></Page></Project>',
                encoding="utf-8",
            )
            (actual / "InkToolBar.xaml").write_text(
                '<ResourceDictionary xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation">'
                '<Style TargetType="InkToolbar" /></ResourceDictionary>',
                encoding="utf-8",
            )

            logical_path = "controls/dev/InkToolbar/InkToolbar.xaml"
            self.assertEqual(sources._pages(str(root)), [("DefaultStyle", "Undefined", logical_path)])
            items = sources.document_items(str(root), logical_path)
            self.assertEqual([(item.document, item.key) for item in items], [(logical_path, "{InkToolbar}")])
            # The styles of InkToolbar are ported by ferroui-controls-inking (#283): no waiver.
            self.assertIsNone(sources.waiver_for(sources.load_waivers(), items[0]))

    def test_ambiguous_case_fallback_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Control.xaml").touch()
            (root / "CONTROL.xaml").touch()
            if len(list(root.iterdir())) != 2:
                self.skipTest("requires a case-sensitive filesystem")
            with self.assertRaisesRegex(ValueError, "ambiguous Windows source path"):
                sources.resolve_source_path(str(root / "control.xaml"))


if __name__ == "__main__":
    unittest.main()
