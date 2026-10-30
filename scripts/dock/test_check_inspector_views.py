"""The fidelity guard rejects omitted views and changed binding contracts."""
from pathlib import Path
import tempfile
import unittest
from check_inspector_views import check_views, migrate_main_view


class InspectorViewChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.source, self.target = root / 'source', root / 'target'
        self.source.mkdir()
        self.target.mkdir()
        self.original = '<UserControl xmlns="https://github.com/avaloniaui" xmlns:m="clr-namespace:Dock.Model.Core;assembly=Dock.Model" x:Class="Dock.Avalonia.Diagnostics.Controls.DockableDebugView"><TextBox Text="{Binding Title, Mode=TwoWay}"/></UserControl>'
        self.migrated = self.original.replace('https://github.com/avaloniaui', 'https://github.com/ferroui').replace('Dock.Avalonia.Diagnostics', 'FerroDock.Ferro.Diagnostics').replace('clr-namespace:Dock.Model.Core;assembly=Dock.Model', 'clr-namespace:FerroDock.Model.Core;assembly=FerroDock.Model')
        (self.source / 'DockableDebugView.axaml').write_text(self.original)
        (self.target / 'DockableDebugView.xaml').write_text(self.migrated)

    def test_only_namespace_migration_is_accepted(self):
        self.assertEqual(check_views(self.source, self.target), [])

    def test_changed_binding_mode_is_rejected(self):
        (self.target / 'DockableDebugView.xaml').write_text(self.migrated.replace('TwoWay', 'OneWay'))
        self.assertTrue(check_views(self.source, self.target))

    def test_missing_and_extra_views_are_rejected(self):
        (self.target / 'DockableDebugView.xaml').unlink()
        (self.target / 'OtherDebugView.xaml').write_text(self.migrated)
        self.assertEqual(len(check_views(self.source, self.target)), 2)

    def test_absent_original_inputs_cannot_pass_vacuously(self):
        (self.source / 'DockableDebugView.axaml').unlink()
        (self.target / 'DockableDebugView.xaml').unlink()
        self.assertTrue(check_views(self.source, self.target))

    def test_main_inspector_keeps_all_content_with_only_declared_mvvm_changes(self):
        original = '<UserControl xmlns:diagnostics="clr-namespace:Dock.Avalonia.Diagnostics.Controls"><DataTemplate DataType="core:IDock"><TextBlock/></DataTemplate><DockPanel DataContext="{Binding #Root}"><Button Click="OnClearEvents"/></DockPanel></UserControl>'
        (self.source / 'RootDockDebug.axaml').write_text(original)
        main = self.target / 'RootDockDebug.xaml'
        main.write_text(migrate_main_view(original))
        self.assertEqual(check_views(self.source, self.target), [])
        main.write_text(main.read_text().replace('<TextBlock/>', ''))
        self.assertTrue(check_views(self.source, self.target))

    def test_core_contracts_use_original_model_owner(self):
        original = '<UserControl xmlns:core="clr-namespace:Dock.Model.Core;assembly=Dock.Model" xmlns:diagnostics="clr-namespace:Dock.Avalonia.Diagnostics.Controls"><DataTemplate DataType="core:IDockWindow"/><DataTemplate DataType="core:IDock"/></UserControl>'
        (self.source / 'RootDockDebug.axaml').write_text(original)
        migrated = migrate_main_view(original)
        self.assertIn('core:IDockWindow', migrated)
        self.assertIn('DataType="core:IDock"', migrated)
        self.assertIn('assembly=FerroDock.Model', migrated)
        main = self.target / 'RootDockDebug.xaml'
        main.write_text(migrated)
        self.assertEqual(check_views(self.source, self.target), [])
        main.write_text(migrated.replace('assembly=FerroDock.Model', 'assembly=FerroDock.Ferro'))
        self.assertTrue(check_views(self.source, self.target))

    def test_window_property_view_requires_original_core_owner(self):
        original = '<UserControl xmlns:core="clr-namespace:Dock.Model.Core;assembly=Dock.Model" x:DataType="core:IDockWindow"><TextBox Text="{Binding Title, Mode=TwoWay}"/></UserControl>'
        (self.source / 'DockWindowDebugView.axaml').write_text(original)
        migrated = original.replace('clr-namespace:Dock.Model.Core;assembly=Dock.Model', 'clr-namespace:FerroDock.Model.Core;assembly=FerroDock.Model')
        view = self.target / 'DockWindowDebugView.xaml'
        view.write_text(migrated)
        self.assertEqual(check_views(self.source, self.target), [])
        view.write_text(migrated.replace('assembly=FerroDock.Model', 'assembly=FerroDock.Ferro'))
        self.assertTrue(check_views(self.source, self.target))

    def test_controls_contracts_require_original_model_owner_in_main_and_property_views(self):
        original = '<UserControl xmlns:m="clr-namespace:Dock.Model.Controls;assembly=Dock.Model" x:DataType="m:IRootDock"><TextBlock Text="{Binding Windows.Count}"/></UserControl>'
        migrated = original.replace('clr-namespace:Dock.Model.Controls;assembly=Dock.Model', 'clr-namespace:FerroDock.Model.Controls;assembly=FerroDock.Model')
        (self.source / 'RootDockDebugView.axaml').write_text(original)
        view = self.target / 'RootDockDebugView.xaml'
        view.write_text(migrated)
        self.assertEqual(check_views(self.source, self.target), [])
        view.write_text(migrated.replace('assembly=FerroDock.Model', 'assembly=FerroDock.Ferro'))
        self.assertTrue(check_views(self.source, self.target))
        self.assertIn('clr-namespace:FerroDock.Model.Controls;assembly=FerroDock.Model', migrate_main_view(original))
