//! The WinUI migration output is a common input to the interpreter and Rust emitter.
//!
//! The checked-in generated file is compiled by rustc, then the same lifecycle and live
//! source/target updates are exercised through both backends. Regenerate only this fixture:
//! `cargo test -p ferroui-markup-xaml-loader --features emitter --test winui_compiled
//! regenerate_winui_binding_fixture -- --ignored --exact`

use ferroui_base::controls::NameScope;
use ferroui_base::input::{FocusChangedEventArgs, InputElement, Key, KeyEventArgs};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{Ref, TypeInfo};
use ferroui_controls::navigation::NavigationCacheMode;
use ferroui_controls::primitives::{
    InfoBadgeTemplateSettings, InfoBarPanel, InfoBarTemplateSettings, ProgressRingTemplateSettings,
};
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    Frame, NavigationView, NavigationViewBackButtonVisible, NavigationViewItem,
    NavigationViewItemHeader, NavigationViewItemSeparator, NavigationViewPaneDisplayMode, Symbol,
    SymbolIcon,
};
use ferroui_controls::{
    AutoSuggestBox, Button, ContentControl, ContentPage, Control, FontIcon, FontIconSource,
    FramePage, InfoBadge, InfoBar, NumberBox, NumberBoxSpinButtonPlacementMode, NumericUpDown,
    Page, PathIconSource, ProgressRing, RadioButtons, RatingControl, RatingItemFontInfo,
    RatingItemImageInfo, RatingItemInfo, StackPanel, SymbolIconSource, TextBlock, TextBox,
};
use ferroui_markup_xaml::winui::WinUIBindings;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::generate_file;
use ferroui_markup_xaml_loader::winui::{migrate_winui_xaml, WinUIXBindPolicy, WinUIXamlOptions};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

#[allow(dead_code)]
#[rustfmt::skip]
#[path = "fixtures/winui_bindings.generated.rs"]
mod generated;

const ROOT: &str = include_str!("fixtures/winui_root.xaml");
const TEMPLATE: &str = include_str!("fixtures/winui_template.xaml");
const NAVIGATION: &str = include_str!("fixtures/winui_navigation.xaml");
const NATIVE_CONTROLS: &str = include_str!("fixtures/native_controls.xaml");

fn options() -> WinUIXamlOptions {
    WinUIXamlOptions {
        x_bind: WinUIXBindPolicy::PropertyPathsWithExplicitLifecycle,
        ..Default::default()
    }
}

fn start() -> UnitTestApplicationScope {
    ferroui_markup_xaml::register_types();
    // The native fixture realizes geometry-backed icon resources and text controls.
    // Use the established mock render/font services while retaining default control metadata.
    let mut services = TestServices::styled_window();
    services.theme = None;
    UnitTestApplication::start(services)
}

fn parts(root: &Control) -> (Ref<TextBlock>, Ref<TextBox>, Ref<TextBlock>) {
    // IDataTemplate::build returns the control; its separate template name scope is
    // consumed by the presenter rather than attached to that control automatically.
    let panel = root.to_ref().cast::<StackPanel>().unwrap();
    let children = panel.children();
    let initial = children.get(0).cast::<TextBlock>().unwrap();
    let editor = children.get(1).cast::<TextBox>().unwrap();
    let mirror = children.get(2).cast::<TextBlock>().unwrap();
    assert_eq!(initial.name().as_deref(), Some("initial"));
    assert_eq!(editor.name().as_deref(), Some("editor"));
    assert_eq!(mirror.name().as_deref(), Some("mirror"));
    (initial, editor, mirror)
}

fn assert_root_behavior(root: Ref<StackPanel>) {
    let (initial, editor, mirror) = parts(&root);
    let scope = NameScope::get_name_scope(&root).expect("document root owns its name scope");
    assert_eq!(scope.find_as::<TextBlock>("initial"), Some(initial.clone()));
    assert_eq!(scope.find_as::<TextBox>("editor"), Some(editor.clone()));
    assert_eq!(scope.find_as::<TextBlock>("mirror"), Some(mirror.clone()));
    let bindings = WinUIBindings::get_for_root(&root).unwrap();
    assert!(!bindings.is_initialized());
    assert_eq!(initial.text(), None);
    assert_eq!(editor.text(), None);
    assert_eq!(mirror.text(), None);
    root.set_tag(Some(Rc::new("assigned after construction".to_string())));
    bindings.initialize().unwrap();
    assert_eq!(
        initial.text().as_deref(),
        Some("assigned after construction")
    );
    assert_eq!(editor.text(), initial.text());
    assert_eq!(mirror.text(), initial.text());
    root.set_tag(Some(Rc::new("live change".to_string())));
    assert_eq!(
        initial.text().as_deref(),
        Some("assigned after construction")
    );
    assert_eq!(editor.text().as_deref(), Some("live change"));
    assert_eq!(mirror.text().as_deref(), Some("live change"));

    editor.set_current_value(TextBox::text_property(), Some("user draft".into()));
    assert_eq!(mirror.text().as_deref(), Some("live change"));
    editor.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    assert_eq!(
        root.tag().unwrap().downcast_ref::<String>().unwrap(),
        "user draft"
    );
    assert_eq!(mirror.text().as_deref(), Some("user draft"));

    bindings.stop_tracking();
    root.set_tag(Some(Rc::new("while stopped".to_string())));
    assert_eq!(mirror.text().as_deref(), Some("user draft"));
    editor.set_current_value(TextBox::text_property(), Some("blocked write".into()));
    editor.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    assert_eq!(
        root.tag().unwrap().downcast_ref::<String>().unwrap(),
        "while stopped"
    );
    bindings.initialize().unwrap();
    assert_eq!(initial.text().as_deref(), Some("while stopped"));
    assert_eq!(editor.text(), initial.text());
    assert_eq!(mirror.text(), initial.text());
}

fn item(text: &str) -> Ref<Button> {
    let result = Button::new();
    result.set_content(Some(Rc::new(text.to_string())));
    result
}

fn assert_template_behavior(template: Rc<dyn IDataTemplate>) {
    let first = template.build(&None).unwrap();
    let second = template.build(&None).unwrap();
    let (initial, editor, mirror) = parts(&first);
    let (second_initial, second_editor, second_mirror) = parts(&second);
    let first_item = item("first");
    let second_item = item("second");
    first.set_data_context(Some(Rc::new(first_item.clone())));
    second.set_data_context(Some(Rc::new(second_item.clone())));
    let first_bindings = WinUIBindings::get_for_root(&first).unwrap();
    let second_bindings = WinUIBindings::get_for_root(&second).unwrap();
    assert!(!Rc::ptr_eq(&first_bindings, &second_bindings));
    assert_eq!(initial.text(), None);
    first_bindings.initialize().unwrap();
    assert_eq!(initial.text().as_deref(), Some("first"));
    assert_eq!(editor.text(), initial.text());
    assert_eq!(mirror.text(), initial.text());
    assert_eq!(second_initial.text(), None);
    second_bindings.initialize().unwrap();
    assert_eq!(second_initial.text().as_deref(), Some("second"));

    first_item.set_content(Some(Rc::new("changed source".to_string())));
    assert_eq!(initial.text().as_deref(), Some("first"));
    assert_eq!(editor.text().as_deref(), Some("changed source"));
    assert_eq!(mirror.text().as_deref(), Some("changed source"));
    editor.set_current_value(TextBox::text_property(), Some("changed target".into()));
    assert_eq!(
        first_item
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "changed target"
    );
    assert_eq!(mirror.text().as_deref(), Some("changed target"));
    assert_eq!(second_editor.text().as_deref(), Some("second"));

    let recycled_item = item("recycled item");
    first.set_data_context(Some(Rc::new(recycled_item.clone())));
    assert_eq!(initial.text().as_deref(), Some("recycled item"));
    assert_eq!(editor.text(), initial.text());
    assert_eq!(mirror.text(), initial.text());
    first_item.set_content(Some(Rc::new("old item is detached".to_string())));
    assert_eq!(mirror.text().as_deref(), Some("recycled item"));
    first_bindings.stop_tracking();
    editor.set_current_value(TextBox::text_property(), Some("blocked write".into()));
    assert_eq!(
        recycled_item
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "recycled item"
    );
    second_item.set_content(Some(Rc::new("second remains live".to_string())));
    assert_eq!(second_mirror.text().as_deref(), Some("second remains live"));
}

fn assert_navigation_behavior(build: impl Fn() -> Ref<NavigationView>) {
    let weak = {
        let root = build();
        let second = build();
        let scope = NameScope::get_name_scope(&root).unwrap();
        let inbox = scope.find_as::<NavigationViewItem>("Inbox").unwrap();
        let archive = scope.find_as::<NavigationViewItem>("Archive").unwrap();
        let help = scope.find_as::<NavigationViewItem>("Help").unwrap();
        assert_eq!(
            root.pane_display_mode(),
            NavigationViewPaneDisplayMode::Left
        );
        assert_eq!(
            root.is_back_button_visible(),
            NavigationViewBackButtonVisible::Visible
        );
        assert!(root.is_back_enabled());
        assert_eq!(root.menu_items().count(), 3);
        assert_eq!(root.footer_menu_items().count(), 1);
        assert_eq!(inbox.menu_items().count(), 1);
        assert!(Control::from_boxed(&root.menu_items().get_at(1).unwrap())
            .unwrap()
            .cast::<NavigationViewItemHeader>()
            .is_some());
        assert!(Control::from_boxed(&root.menu_items().get_at(2).unwrap())
            .unwrap()
            .cast::<NavigationViewItemSeparator>()
            .is_some());
        assert!(Control::from_boxed(&root.content().unwrap())
            .unwrap()
            .cast::<Frame>()
            .is_some());
        let icon = inbox.icon().unwrap().cast::<SymbolIcon>().unwrap();
        assert_eq!(icon.symbol(), Symbol::Mail);
        assert_eq!(
            archive
                .icon()
                .unwrap()
                .cast::<SymbolIcon>()
                .unwrap()
                .symbol(),
            Symbol::Save
        );
        assert_eq!(
            root.auto_suggest_box()
                .unwrap()
                .query_icon()
                .unwrap()
                .cast::<SymbolIcon>()
                .unwrap()
                .symbol(),
            Symbol::Find
        );
        assert_eq!(inbox.info_badge().unwrap().value(), 5);
        let other_inbox = NameScope::get_name_scope(&second)
            .unwrap()
            .find_as::<NavigationViewItem>("Inbox")
            .unwrap();
        assert!(!icon.ptr_eq(&other_inbox.icon().unwrap()));
        assert!(!inbox
            .info_badge()
            .unwrap()
            .ptr_eq(&other_inbox.info_badge().unwrap()));
        root.apply_template();
        let invoke = |item: &NavigationViewItem| {
            let mut event = KeyEventArgs::new();
            event.key = Key::Enter;
            event.set_routed_event(Some(InputElement::key_down_event()));
            item.raise_event(&event);
            assert!(event.handled());
        };
        invoke(&archive);
        assert!(archive.is_selected());
        assert!(inbox.is_child_selected());
        assert!(Control::from_boxed(&root.selected_item().unwrap())
            .unwrap()
            .ptr_eq(&archive));
        assert!(second.selected_item().is_none());
        invoke(&help);
        assert!(!help.selects_on_invoked());
        assert!(archive.is_selected());
        root.menu_items().remove_at(0);
        assert!(root.selected_item().is_none());
        root.downgrade()
    };
    assert!(
        weak.upgrade().is_none(),
        "markup, selection and template subscriptions release the root"
    );
}

#[test]
fn interpreted_navigation_preserves_collections_icons_selection_and_instance_lifetimes() {
    let _app = start();
    assert_navigation_behavior(|| {
        FerroRuntimeXamlLoader::parse_winui_as(NAVIGATION, None, &options()).unwrap()
    });
}

#[test]
fn compiled_navigation_preserves_collections_icons_selection_and_instance_lifetimes() {
    let _app = start();
    assert_navigation_behavior(|| {
        let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
        generated::build_navigation_xaml(Some(provider)).unwrap()
    });
}

#[test]
fn interpreted_root_defers_initialization_and_tracks_live_source_and_target_changes() {
    let _app = start();
    assert_root_behavior(FerroRuntimeXamlLoader::parse_winui_as(ROOT, None, &options()).unwrap());
}

#[test]
fn interpreted_typed_templates_keep_live_bindings_and_recycling_independent() {
    let _app = start();
    let host: Ref<ContentControl> =
        FerroRuntimeXamlLoader::parse_winui_as(TEMPLATE, None, &options()).unwrap();
    assert_template_behavior(host.content_template().unwrap());
}

#[test]
fn compiled_root_defers_initialization_and_tracks_live_source_and_target_changes() {
    let _app = start();
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    assert_root_behavior(generated::build_root_xaml(Some(provider)).unwrap());
}

#[test]
fn compiled_typed_templates_keep_live_bindings_and_recycling_independent() {
    let _app = start();
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    let host = generated::build_template_xaml(Some(provider)).unwrap();
    assert_template_behavior(host.content_template().unwrap());
}

fn assert_native_controls(root: Ref<StackPanel>) {
    let scope = NameScope::get_name_scope(&root).expect("native document name scope");
    let existing_page = scope.find_as::<ContentPage>("ExistingPage").unwrap();
    assert!(Page::TYPE.is_assignable_from(existing_page.get_type()));
    assert_eq!(from_markup_value::<&'static TypeInfo>(&existing_page.tag()), Some(Page::TYPE));
    assert!(!Page::TYPE.is_assignable_from(FramePage::TYPE));
    assert!(std::ptr::eq(TypeInfo::find("FerroUI.Controls", "Page").unwrap(), Page::TYPE));
    let page = scope.find_as::<FramePage>("NativePage").unwrap();
    assert_eq!(page.navigation_cache_mode(), NavigationCacheMode::Required);
    let frame = scope.find_as::<Frame>("NativeFrame").unwrap();
    assert_eq!(frame.current_source_page_type(), Some(FramePage::TYPE));
    let number = scope.find_as::<NumberBox>("Number").unwrap();
    assert_eq!(number.value(), 12.0);
    assert_eq!(number.spin_button_placement_mode(), NumberBoxSpinButtonPlacementMode::Inline);
    assert!(scope.find_as::<NumericUpDown>("ExistingNumeric").is_some());
    assert!(!NumericUpDown::TYPE.is_assignable_from(NumberBox::TYPE));
    assert_eq!(scope.find_as::<RadioButtons>("Options").unwrap().selected_index(), 1);
    assert_eq!(scope.find_as::<FontIcon>("Font").unwrap().glyph(), "B");
    assert_eq!(scope.find_as::<SymbolIcon>("Symbol").unwrap().symbol(), Symbol::Accept);
    let search = scope.find_as::<AutoSuggestBox>("Search").unwrap();
    assert_eq!(search.text(), "Find");
    assert_eq!(search.query_icon().unwrap().cast::<SymbolIcon>().unwrap().symbol(), Symbol::Find);
    assert!(scope.find_as::<InfoBar>("Bar").unwrap().is_open());
    let badge = scope.find_as::<InfoBadge>("Badge").unwrap();
    assert_eq!(badge.value(), 7);
    assert_eq!(badge.icon_source().unwrap().cast::<FontIconSource>().unwrap().glyph(), "A");
    let ring = scope.find_as::<ProgressRing>("Ring").unwrap();
    assert_eq!((ring.maximum(), ring.value()), (200.0, 42.0));
    let rating = scope.find_as::<RatingControl>("Rating").unwrap();
    assert_eq!(rating.placeholder_value(), 2.5);
    assert_eq!(rating.item_info().unwrap().cast::<RatingItemFontInfo>().unwrap().unset_glyph(), "D");
    assert_eq!(scope.find_as::<InfoBarPanel>("BarPanel").unwrap().children().count(), 1);
    let navigation = scope.find_as::<NavigationView>("Navigation").unwrap();
    assert_eq!(navigation.menu_items().count(), 3);
    assert_eq!(scope.find_as::<NavigationViewItem>("Inbox").unwrap().icon().unwrap().cast::<SymbolIcon>().unwrap().symbol(), Symbol::Mail);

    let resources = root.resources();
    assert!(from_markup_value::<Ref<SymbolIconSource>>(&resources.get(&"SymbolSource".into())).is_some());
    assert!(from_markup_value::<Ref<PathIconSource>>(&resources.get(&"PathSource".into())).is_some());
    assert!(from_markup_value::<Ref<RatingItemInfo>>(&resources.get(&"RatingInfo".into())).is_some());
    assert!(from_markup_value::<Ref<RatingItemImageInfo>>(&resources.get(&"RatingImages".into())).is_some());
    assert!(from_markup_value::<Ref<InfoBarTemplateSettings>>(&resources.get(&"BarSettings".into())).is_some());
    assert!(from_markup_value::<Ref<InfoBadgeTemplateSettings>>(&resources.get(&"BadgeSettings".into())).is_some());
    assert!(from_markup_value::<Ref<ProgressRingTemplateSettings>>(&resources.get(&"RingSettings".into())).is_some());
}

#[test]
fn native_presentation_namespace_constructs_controls_without_import_policy() {
    let _app = start();
    assert_native_controls(FerroRuntimeXamlLoader::parse_as(NATIVE_CONTROLS, None).unwrap());
    assert!(migrate_winui_xaml(NATIVE_CONTROLS, &options()).unwrap().edits.is_empty());
}

#[test]
fn compiled_native_presentation_namespace_constructs_the_same_controls() {
    let _app = start();
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    assert_native_controls(generated::build_native_controls_xaml(Some(provider)).unwrap());
}

#[test]
fn generated_winui_binding_fixture_matches_the_current_compiler() {
    assert_eq!(
        include_str!("fixtures/winui_bindings.generated.rs"),
        emitted_fixture(),
        "regenerate the focused fixture with the command in this module's header"
    );
}

fn emitted_fixture() -> String {
    ferroui_markup_xaml::register_types();
    let mut documents: Vec<_> = [
        ("root.xaml", ROOT),
        ("template.xaml", TEMPLATE),
        ("navigation.xaml", NAVIGATION),
    ]
    .into_iter()
    .map(|(name, source)| {
        let native = migrate_winui_xaml(source, &options())
            .unwrap()
            .into_xaml()
            .unwrap();
        (name, native)
    })
    .collect();
    // Ordinary native XAML goes straight to the compiler with no migration policy.
    documents.push(("native_controls.xaml", NATIVE_CONTROLS.to_string()));
    let borrowed: Vec<_> = documents
        .iter()
        .map(|(name, native)| (*name, native.as_str()))
        .collect();
    let file = generate_file(
        "FerroUI.WinUI.BindingFixture",
        "ferres://FerroUI.WinUI.BindingFixture/",
        &borrowed,
        &RuntimeXamlLoaderConfiguration::new(),
        &[],
    );
    for (document, reason) in &file.documents {
        assert!(reason.is_none(), "{document}: {reason:?}");
    }
    file.source
}

#[test]
#[ignore = "updates the checked-in fixture; run explicitly after reviewing emitter changes"]
fn regenerate_winui_binding_fixture() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/winui_bindings.generated.rs");
    std::fs::write(path, emitted_fixture()).unwrap();
}
