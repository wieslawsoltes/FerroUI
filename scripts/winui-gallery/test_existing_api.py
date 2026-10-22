"""The existing public source surface cannot be silently renamed or removed."""

import copy
import tempfile
from pathlib import Path
import unittest

import existing_api as api


def declaration(identity="FerroUI.Controls.Page", **overrides):
    namespace, name = identity.rsplit(".", 1)
    return dict({
        "id": identity, "namespace": namespace, "name": name,
        "base": "ContentControl", "members": ["new", "title"],
        "registered": True, "public_declaration": True,
        "declaration_paths": ["src/Controls/page.rs"],
    }, **overrides)


class ExistingApiTests(unittest.TestCase):
    def setUp(self):
        self.row = declaration()
        self.baseline = {self.row["id"]: self.row}

    def test_type_or_namespace_rename_is_a_removal_even_when_short_name_matches(self):
        renamed = declaration("FerroUI.Controls.FramePage")
        errors = api.compatibility_errors(self.baseline, {renamed["id"]: renamed})
        self.assertEqual(1, len(errors))
        self.assertIn("Removed or renamed type/namespace: FerroUI.Controls.Page", errors[0])

    def test_removed_members_registration_and_visibility_are_independent_failures(self):
        current = {self.row["id"]: declaration(members=["new"], registered=False, public_declaration=False)}
        errors = api.compatibility_errors(self.baseline, current)
        self.assertEqual(3, len(errors))
        self.assertTrue(any("members" in error and "title" in error for error in errors))
        self.assertTrue(any("registration" in error for error in errors))
        self.assertTrue(any("visibility" in error for error in errors))

    def test_base_changes_fail_without_treating_a_same_name_type_as_evidence(self):
        current = {self.row["id"]: declaration(base="Control")}
        self.assertEqual(1, len(api.compatibility_errors(self.baseline, current)))
        self.assertIn("Base changed", api.compatibility_errors(self.baseline, current)[0])

    def test_additive_frame_page_preserves_the_existing_page_contract(self):
        native = declaration(members=["new", "set_title", "title"])
        frame_page = declaration("FerroUI.Controls.FramePage", base="UserControl",
                                 declaration_paths=["src/Controls/frame_page.rs"])
        current = {native["id"]: native, frame_page["id"]: frame_page}
        self.assertEqual([], api.compatibility_errors(self.baseline, current))

    def test_new_namespace_collisions_fail_even_if_merged_members_look_complete(self):
        collision = declaration(declaration_paths=["src/Controls/page.rs", "src/Controls/frame_page.rs"])
        errors = api.compatibility_errors(self.baseline, {collision["id"]: collision})
        self.assertEqual(1, len(errors))
        self.assertIn("Namespace identity collision", errors[0])
        new = declaration("FerroUI.Controls.BrandNew", declaration_paths=["src/Controls/a.rs", "src/Controls/b.rs"])
        self.assertEqual(1, len(api.compatibility_errors(self.baseline, self.baseline | {new["id"]: new})))

    def test_existing_conditional_origins_are_not_reinterpreted_as_new_collisions(self):
        row = declaration(declaration_paths=["src/Controls/unix.rs", "src/Controls/windows.rs"])
        self.assertEqual([], api.compatibility_errors({row["id"]: row}, {row["id"]: copy.deepcopy(row)}))

    def test_collector_excludes_private_helpers_and_keeps_namespaces_distinct(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            crate = root / "src/Controls"
            (crate / "navigation").mkdir(parents=True)
            (crate / "Cargo.toml").write_text('[package]\nname = "controls"\nversion = "0.1.0"\n')
            (crate / "register_types.rs").write_text('''
const NAMESPACES: &[(&str, &str)] = &[
    ("controls", "FerroUI.Controls"),
    ("controls::navigation", "FerroUI.Navigation"),
];
const TYPES: &[&TypeInfo] = &[Page::TYPE];
''')
            (crate / "page.rs").write_text('''
pub struct Page { pub title: String }
ferro_class!(Page: ContentControl);
impl Page { pub fn new() {} fn private_helper() {} }
struct Hidden { pub field: i32 }
pub(crate) struct CrateOnly { pub field: i32 }
''')
            (crate / "navigation/page.rs").write_text("pub struct Page {}\nferro_class!(Page: ContentControl);")
            result = api.collect_surface(root)
        self.assertEqual({"FerroUI.Controls.Page", "FerroUI.Navigation.Page"}, set(result))
        self.assertEqual(["new", "title"], result["FerroUI.Controls.Page"]["members"])
        self.assertTrue(result["FerroUI.Controls.Page"]["registered"])
        self.assertNotEqual(result["FerroUI.Controls.Page"]["declaration_paths"], result["FerroUI.Navigation.Page"]["declaration_paths"])

    def test_native_navigation_namespaces_do_not_merge_distinct_event_contracts(self):
        page_args = declaration("FerroUI.Controls.NavigationEventArgs", base=None,
                                members=["navigation_type", "page", "parameter"])
        frame_args = declaration("FerroUI.Navigation.NavigationEventArgs", base=None,
                                 members=["content", "navigation_mode", "parameter", "source_page_type"],
                                 declaration_paths=["src/Controls/navigation.rs"])
        current = {page_args["id"]: page_args, frame_args["id"]: frame_args}
        self.assertEqual([], api.compatibility_errors({page_args["id"]: page_args}, current))
        self.assertTrue(api.compatibility_errors({page_args["id"]: page_args}, {frame_args["id"]: frame_args}))


if __name__ == "__main__":
    unittest.main()
