use ferroui_base::metadata::from_markup_value;
use ferroui_base::{Ref, StaticType, TypeInfo};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{SelectorBar, StackPanel};
use ferroui_controls_flip_view::FlipView;
use ferroui_controls_icons::{BitmapIcon, ImageIcon};
use ferroui_controls_password_box::{PasswordBox, PasswordRevealMode};
use ferroui_controls_split_menu::SplitMenuFlyoutItem;
use ferroui_controls_teaching_tip::TeachingTip;
use ferroui_markup_xaml::RuntimeXamlLoaderDocument;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

const DOCUMENT: &str = include_str!("DefaultNamespaceRoot.xaml");

fn register() {
    ferroui_markup_xaml::register_types();
    ferroui_controls::register_types();
    ferroui_controls_flip_view::register_types();
    ferroui_controls_icons::register_types();
    ferroui_controls_password_box::register_types();
    ferroui_controls_split_menu::register_types();
    ferroui_controls_teaching_tip::register_types();
}

fn assert_resolved(root: &Ref<StackPanel>) {
    let expected: [&TypeInfo; 7] = [
        SelectorBar::TYPE,
        BitmapIcon::TYPE,
        ImageIcon::TYPE,
        FlipView::TYPE,
        TeachingTip::TYPE,
        PasswordBox::TYPE,
        SplitMenuFlyoutItem::TYPE,
    ];
    let children = root.children().snapshot();
    assert_eq!(children.len(), expected.len());
    for (child, expected) in children.iter().zip(expected) {
        assert!(std::ptr::eq(child.get_type(), expected), "{} is not {}", child.get_type().name(), expected.name());
    }
    let password = children[5].clone().cast::<PasswordBox>().unwrap();
    assert_eq!(password.password_reveal_mode(), PasswordRevealMode::Hidden);
    let bar = children[0].clone().cast::<SelectorBar>().unwrap();
    assert_eq!(bar.items().count(), 1);
}

#[test]
fn the_compiled_loader_resolves_the_controls_without_a_prefix() {
    let _scope = UnitTestApplication::start(TestServices::styled_window());
    register();
    crate::compiled_markup::register();
    let value = crate::compiled_xaml::try_load(
        None,
        "ferres://ferroui_controls_default_namespace_tests/DefaultNamespaceRoot.xaml",
    )
    .unwrap()
    .unwrap();
    assert_resolved(value.downcast_ref::<Ref<StackPanel>>().unwrap());
}

#[test]
fn the_run_time_loader_resolves_the_controls_without_a_prefix() {
    let _scope = UnitTestApplication::start(TestServices::styled_window());
    register();
    FerroRuntimeXamlLoader::register();
    let value = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(DOCUMENT), None)
        .unwrap_or_else(|error| panic!("the document failed to load: {}", error.message()));
    assert_resolved(&from_markup_value::<Ref<StackPanel>>(&Some(value)).expect("a StackPanel"));
}
