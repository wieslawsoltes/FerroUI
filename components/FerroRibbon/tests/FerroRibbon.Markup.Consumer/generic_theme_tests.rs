//! Public Generic resource entry point, using actual compiled and runtime XAML.
//! Structural source coverage is checked separately by compose_generic_theme.py.
use ferroribbon::{
    controls::*,
    mvvm::{RibbonElementFactory, RibbonModelBindings},
    theming::RibbonThemeResources,
};
use ferroribbon_core::model::{RibbonButtonModelImpl, RibbonItemModelImpl, RibbonNodeModelExt};
use ferroui_base::{
    BoxedValue, Ref, TypeInfo,
    controls::{IResourceDictionary, ResourceDictionary, ResourceKey},
    data::core::{ValueType, ValueTypes},
    media::{Color, SolidColorBrush},
    styling::{ControlTheme, ThemeVariant},
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBlock, Window};
use std::rc::Rc;

fn source_types() -> &'static [&'static TypeInfo] {
    &[
        RibbonButton::TYPE,
        RibbonDropDownButton::TYPE,
        RibbonToggleButton::TYPE,
        RibbonSplitButton::TYPE,
        RibbonColorPicker::TYPE,
        RibbonCheckBox::TYPE,
        RibbonSeparator::TYPE,
        RibbonLabel::TYPE,
        RibbonButtonGroup::TYPE,
        RibbonStackPanel::TYPE,
        RibbonScreenTip::TYPE,
        Ribbon::TYPE,
        RibbonTabHeader::TYPE,
        RibbonTab::TYPE,
        RibbonGroup::TYPE,
        RibbonQuickAccessToolBar::TYPE,
        RibbonComboBox::TYPE,
        RibbonFontComboBox::TYPE,
        RibbonFontSizeComboBox::TYPE,
        RibbonSpinner::TYPE,
        RibbonTextBox::TYPE,
        RibbonSlider::TYPE,
        RibbonGallery::TYPE,
        RibbonColorPalette::TYPE,
        RibbonGridPicker::TYPE,
        RibbonSegmentedControl::TYPE,
        RibbonSearchBox::TYPE,
        RibbonTitleBar::TYPE,
        RibbonStatusBar::TYPE,
        RibbonZoomControl::TYPE,
        RibbonToolBar::TYPE,
        RibbonContextualToolBar::TYPE,
        RibbonMenuBar::TYPE,
        RibbonKeyTip::TYPE,
        RibbonBackstage::TYPE,
    ]
}

fn theme(value: BoxedValue) -> Ref<ControlTheme> {
    value.downcast_ref::<Ref<ControlTheme>>().unwrap().clone()
}

fn load(runtime: bool) -> Ref<StackPanel> {
    let value = if runtime {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("Generic.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap()
    } else {
        crate::compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Generic.xaml")
            .unwrap()
            .unwrap()
    };
    ValueTypes::as_object(&*value).unwrap().cast().unwrap()
}

fn generic(panel: &StackPanel) -> Rc<dyn IResourceDictionary> {
    panel
        .resources()
        .merged_dictionaries()
        .get(0)
        .resource_dictionary()
        .unwrap()
}

fn assert_source_padding(button: &RibbonButton, phase: &str) {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/generic-style-replacement.json")).unwrap();
    let row = fixture["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["phase"] == phase)
        .unwrap();
    let expected = row["padding"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect::<Vec<_>>();
    let actual = button.padding();
    assert_eq!(
        [actual.left, actual.top, actual.right, actual.bottom].as_slice(),
        expected.as_slice(),
        "source style boundary {phase}"
    );
}

fn reattach_generated(panel: &StackPanel, button: &Ref<RibbonButton>) {
    let index = panel.children().count() - 1;
    let existing = panel.children().get(index);
    assert!(existing.ptr_eq(&button.clone().upcast::<Control>()));
    panel.children().remove_at(index);
    panel.children().add(existing);
}

#[test]
fn generic_only_consumer_resolves_all_source_style_keys_and_preserves_family_order() {
    crate::tests::run_standalone(|| {
        let _framework_theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let panel = load(runtime);
            let dictionary = generic(&panel);
            let families = dictionary.merged_dictionaries_snapshot();
            assert_eq!(families.len(), 6);
            let family_keys = [
                "RibbonChromeButtonStyle".into(),
                "DefaultRibbonButtonStyle".into(),
                ResourceKey::Type(Ribbon::TYPE),
                ResourceKey::Type(RibbonBackstage::TYPE),
                "DefaultRibbonComboBoxStyle".into(),
                ResourceKey::Type(RibbonTitleBar::TYPE),
            ];
            for (provider, key) in families.iter().zip(family_keys) {
                assert!(provider.resource_dictionary().unwrap().contains_key(&key));
            }
            // Source default type keys: 33 in the four families, plus KeyTip and Backstage.
            let types = source_types();
            for &ty in types {
                let found = theme(
                    dictionary
                        .try_get_resource(&ResourceKey::Type(ty), None)
                        .flatten()
                        .unwrap(),
                );
                assert_eq!(found.target_type(), Some(ty), "{}", ty.name());
            }
            let keys = [
                "DefaultRibbonButtonStyle",
                "DefaultRibbonSplitButtonStyle",
                "DefaultRibbonComboBoxStyle",
                "RibbonChromeButtonStyle",
                "RibbonScrollButtonStyle",
                "RibbonApplicationButtonStyle",
                "RibbonDialogLauncherButtonStyle",
                "RibbonSplitPartButtonStyle",
                "RibbonBackstageNavButtonStyle",
                "RibbonMenuItemButtonStyle",
                "RibbonSpinButtonStyle",
                "RibbonGalleryScrollButtonStyle",
                "RibbonGalleryItemStyle",
                "RibbonSwatchButtonStyle",
                "RibbonMenuFlyoutPresenterStyle",
                "RibbonToolTipStyle",
                "RibbonFlyoutPresenterStyle",
            ];
            for key in keys {
                assert!(
                    theme(
                        dictionary
                            .try_get_resource(&key.into(), None)
                            .flatten()
                            .unwrap()
                    )
                    .target_type()
                    .is_some(),
                    "{key}"
                );
            }
            // BasedOn must retain the named style from this dictionary scope,
            // rather than binding to an independently loaded standalone leaf.
            for (ty, key) in [
                (RibbonButton::TYPE, "DefaultRibbonButtonStyle"),
                (RibbonDropDownButton::TYPE, "DefaultRibbonButtonStyle"),
                (RibbonSplitButton::TYPE, "DefaultRibbonSplitButtonStyle"),
                (RibbonColorPicker::TYPE, "DefaultRibbonSplitButtonStyle"),
                (RibbonComboBox::TYPE, "DefaultRibbonComboBoxStyle"),
                (RibbonFontComboBox::TYPE, "DefaultRibbonComboBoxStyle"),
                (RibbonFontSizeComboBox::TYPE, "DefaultRibbonComboBoxStyle"),
            ] {
                let actual = theme(
                    dictionary
                        .try_get_resource(&ResourceKey::Type(ty), None)
                        .flatten()
                        .unwrap(),
                );
                let base = theme(
                    dictionary
                        .try_get_resource(&key.into(), None)
                        .flatten()
                        .unwrap(),
                );
                assert!(
                    actual.based_on().unwrap().ptr_eq(&base),
                    "{} BasedOn identity",
                    ty.name()
                );
            }
            // Four families each own exactly one Shared include; Backstage does too.
            for i in 1..6 {
                let family = families[i].resource_dictionary().unwrap();
                let includes = family.merged_dictionaries_snapshot();
                assert_eq!(includes.len(), 1, "family {i}");
                let shared = includes[0].resource_dictionary().unwrap();
                assert!(
                    shared
                        .try_get_value(&"RibbonChromeButtonStyle".into())
                        .is_some()
                );
            }
        }
    });
}

#[test]
fn generic_templates_apply_to_direct_and_model_controls_and_allow_downstream_retemplating() {
    crate::tests::run_standalone(|| {
        let _framework_theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let panel = load(runtime);
            // Palette service without its optional Shared template include: Generic is
            // the sole source of ribbon templates in this independent consumer.
            let palette = RibbonThemeResources::with_shared_resources(ResourceDictionary::new());
            panel.resources().add_merged_dictionary(palette.clone());
            let ribbon = panel.children().get(0).cast::<Ribbon>().unwrap();
            let model = panel
                .resources()
                .try_get_value(&"Model".into())
                .unwrap()
                .unwrap();
            let model =
                ValueTypes::try_cast(&model, ValueType::of::<Rc<dyn RibbonButtonModelImpl>>())
                    .unwrap();
            let model = model
                .downcast_ref::<Rc<dyn RibbonButtonModelImpl>>()
                .unwrap()
                .clone();
            // The native Core upcast is used by factory callers, independently of markup metadata.
            let item: Rc<dyn RibbonItemModelImpl> = model.clone();
            let factory = RibbonElementFactory::new();
            factory.set_ribbon(Some(ribbon.clone()));
            let bindings = RibbonModelBindings::new();
            let generated = factory
                .create_item(Some(item), bindings.clone())
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            panel.children().add(generated.clone().upcast::<Control>());
            let w = Window::new();
            w.set_width(900.);
            w.set_height(600.);
            w.set_content(Some(Control::boxed(panel.clone())));
            w.show();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            w.update_layout();
            assert!(ribbon.template().is_some());
            assert!(
                ribbon
                    .get_visual_descendants()
                    .any(|c| c.cast::<RibbonButton>().is_some())
            );
            assert!(generated.template().is_some());
            for index in 1..=3 {
                let c = panel.children().get(index);
                assert!(c.get_effective_theme().is_some(), "consumer child {index}");
                assert!(
                    c.get_visual_descendants().next().is_some(),
                    "consumer child {index}"
                );
            }
            let default = theme(
                generic(&panel)
                    .try_get_resource(&ResourceKey::Type(RibbonButton::TYPE), None)
                    .flatten()
                    .unwrap(),
            );
            assert!(generated.get_effective_theme().unwrap().ptr_eq(&default));
            for variant in [
                ThemeVariant::light(),
                ThemeVariant::dark(),
                RibbonThemeResources::high_contrast_variant(),
            ] {
                panel.set_value(
                    ThemeVariant::requested_theme_variant_property(),
                    Some(variant.clone()),
                );
                w.update_layout();
                let expected = panel
                    .try_find_resource(&"RibbonForegroundBrush".into(), Some(&variant))
                    .flatten()
                    .unwrap();
                let expected = ValueTypes::as_object(&*expected)
                    .unwrap()
                    .cast::<SolidColorBrush>()
                    .unwrap();
                assert!(
                    generated
                        .foreground()
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .to_ref()
                        .cast::<SolidColorBrush>()
                        .unwrap()
                        .ptr_eq(&expected)
                );
                assert!(generated.get_effective_theme().unwrap().ptr_eq(&default));
            }
            panel.set_value(
                ThemeVariant::requested_theme_variant_property(),
                Some(ThemeVariant::dark()),
            );
            let changed = Color::from_argb(255, 16, 32, 48);
            palette.set_color("RibbonForegroundBrush", changed, Some("Dark"));
            w.update_layout();
            assert_eq!(
                generated
                    .foreground()
                    .unwrap()
                    .as_solid_color_brush()
                    .unwrap()
                    .color(),
                changed
            );
            let override_theme = theme(
                panel
                    .resources()
                    .try_get_value(&"ConsumerButton".into())
                    .unwrap()
                    .unwrap(),
            );
            assert_source_padding(&generated, "initial");
            generated.set_theme(Some(override_theme.clone()));
            w.update_layout();
            assert_source_padding(&generated, "explicit");
            assert!(
                generated
                    .get_visual_descendants()
                    .filter_map(|c| c.cast::<TextBlock>())
                    .any(|t| t.text().as_deref() == Some("Consumer template"))
            );
            // Source ClearValue restores implicit lookup; source Style=null is
            // a distinct local-null state (retained in the reference fixture).
            generated.clear_value(ferroui_base::StyledElement::theme_property());
            w.update_layout();
            assert_source_padding(&generated, "clearLocalExplicit");
            let implicit_theme = theme(
                panel
                    .resources()
                    .try_get_value(&"ImplicitConsumerButton".into())
                    .unwrap()
                    .unwrap(),
            );
            panel.resources().set(
                ResourceKey::Type(RibbonButton::TYPE),
                Some(Rc::new(implicit_theme.clone())),
            );
            w.update_layout();
            assert!(generated.get_effective_theme().unwrap().ptr_eq(&default));
            assert_source_padding(&generated, "replaceImplicit");
            reattach_generated(&panel, &generated);
            w.update_layout();
            assert!(
                generated
                    .get_effective_theme()
                    .unwrap()
                    .ptr_eq(&implicit_theme)
            );
            assert_source_padding(&generated, "reattachWithImplicit");
            let fresh = RibbonButton::new();
            panel.children().add(fresh.clone().upcast::<Control>());
            w.update_layout();
            assert_source_padding(&fresh, "freshWithImplicit");
            panel.children().remove_at(panel.children().count() - 1);
            panel
                .resources()
                .remove(&ResourceKey::Type(RibbonButton::TYPE));
            w.update_layout();
            assert!(
                generated
                    .get_effective_theme()
                    .unwrap()
                    .ptr_eq(&implicit_theme)
            );
            assert_source_padding(&generated, "removeImplicit");
            reattach_generated(&panel, &generated);
            w.update_layout();
            assert!(generated.get_effective_theme().unwrap().ptr_eq(&default));
            assert_source_padding(&generated, "reattachWithoutImplicit");
            model.set_label(Some("Updated model".into()));
            assert_eq!(generated.label().as_deref(), Some("Updated model"));
            bindings.dispose();
            model.set_label(Some("After disposal".into()));
            assert_eq!(generated.label().as_deref(), Some("Updated model"));
            w.close();
            w.set_content(None);
            factory.set_ribbon(None);
        }
    });
}

#[test]
fn generic_roots_are_independent_while_source_families_and_styles_are_shared() {
    crate::tests::run_standalone(|| {
        let _framework_theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let first = load(runtime);
            let second = load(runtime);
            let provider = first.resources().merged_dictionaries().get(0);
            let a = generic(&first);
            assert!(*a == *generic(&first));
            let b = generic(&second);
            assert!(*a != *b);
            let source: serde_json::Value =
                serde_json::from_str(include_str!("../fixtures/generic-resource-sharing.json"))
                    .unwrap();
            let families_a = a.merged_dictionaries_snapshot();
            let families_b = b.merged_dictionaries_snapshot();
            let mut style_count = 0;
            for (index, row) in source["families"].as_array().unwrap().iter().enumerate() {
                let family_a = families_a[index].resource_dictionary().unwrap();
                let family_b = families_b[index].resource_dictionary().unwrap();
                assert_eq!(
                    *family_a == *family_b,
                    row["sameAcrossGenericRoots"].as_bool().unwrap()
                );
                for style in row["styles"].as_array().unwrap() {
                    let name = style["key"].as_str().unwrap();
                    let key = if let Some(name) = name.strip_prefix("type:") {
                        let ty = source_types().iter().find(|ty| ty.name() == name).unwrap();
                        ResourceKey::Type(ty)
                    } else {
                        name.into()
                    };
                    let a = theme(family_a.try_get_resource(&key, None).flatten().unwrap());
                    let b = theme(family_b.try_get_resource(&key, None).flatten().unwrap());
                    assert_eq!(
                        a.ptr_eq(&b),
                        style["sameAcrossGenericRoots"].as_bool().unwrap(),
                        "{name}"
                    );
                    style_count += 1;
                }
                let marker: BoxedValue = Rc::new("family mutation".to_owned());
                family_a.set("SourceFamilySharingProbe".into(), Some(marker.clone()));
                assert!(Rc::ptr_eq(
                    &family_b
                        .try_get_value(&"SourceFamilySharingProbe".into())
                        .unwrap()
                        .unwrap(),
                    &marker
                ));
                family_a.remove(&"SourceFamilySharingProbe".into());
            }
            assert_eq!(style_count, 52);
            let shared_a = a.merged_dictionaries_snapshot()[0]
                .resource_dictionary()
                .unwrap();
            let shared_b = b.merged_dictionaries_snapshot()[0]
                .resource_dictionary()
                .unwrap();
            let items_shared = a.merged_dictionaries_snapshot()[1]
                .resource_dictionary()
                .unwrap()
                .merged_dictionaries_snapshot()[0]
                .resource_dictionary()
                .unwrap();
            let chrome_shared = a.merged_dictionaries_snapshot()[5]
                .resource_dictionary()
                .unwrap()
                .merged_dictionaries_snapshot()[0]
                .resource_dictionary()
                .unwrap();
            assert!(*shared_a == *shared_b);
            assert!(*shared_a == *items_shared);
            assert!(*shared_a == *chrome_shared);
            let style_a = theme(
                a.try_get_resource(&"RibbonChromeButtonStyle".into(), None)
                    .flatten()
                    .unwrap(),
            );
            let style_b = theme(
                b.try_get_resource(&"RibbonChromeButtonStyle".into(), None)
                    .flatten()
                    .unwrap(),
            );
            assert!(style_a.ptr_eq(&style_b));
            assert!(
                style_a.ptr_eq(&theme(
                    chrome_shared
                        .try_get_value(&"RibbonChromeButtonStyle".into())
                        .unwrap()
                        .unwrap()
                ))
            );
            let marker: BoxedValue = Rc::new("shared source mutation".to_owned());
            shared_a.set("SourceSharingProbe".into(), Some(marker.clone()));
            for alias in [&shared_b, &items_shared, &chrome_shared] {
                assert!(Rc::ptr_eq(
                    &alias
                        .try_get_resource(&"SourceSharingProbe".into(), None)
                        .flatten()
                        .unwrap(),
                    &marker
                ));
            }
            shared_a.remove(&"SourceSharingProbe".into());
            assert!(provider.owner().is_some());
            a.set("ConsumerOnly".into(), Some(Rc::new("A".to_string())));
            assert!(b.try_get_resource(&"ConsumerOnly".into(), None).is_none());
            first.resources().remove_merged_dictionary(&provider);
            assert!(provider.owner().is_none());
            assert!(
                first
                    .resources()
                    .try_get_resource(&"ConsumerOnly".into(), None)
                    .is_none()
            );
            first.resources().add_merged_dictionary(provider.clone());
            assert!(provider.owner().is_some());
            assert!(*a == *generic(&first));
            a.remove(&"ConsumerOnly".into());
        }
    });
}

#[test]
fn private_shared_include_validates_uri_and_uses_native_authority_normalization() {
    crate::tests::run_standalone(|| {
        use ferroribbon::theming::RibbonSharedThemeInclude;
        use ferroui_base::utilities::{Uri, UriKind};
        let missing = RibbonSharedThemeInclude::new();
        assert!(missing.try_loaded().is_err());
        for uri in [
            "Themes/Shared.xaml",
            "ferres://another_library/Themes/Shared.xaml",
            "ferres://ferroribbon/Themes/UserExtension.xaml",
            "ferres://ferroribbon/themes/Shared.xaml",
        ] {
            let include = RibbonSharedThemeInclude::new();
            include.set_source(Some(Uri::new(uri, UriKind::RelativeOrAbsolute).unwrap()));
            assert!(include.try_loaded().is_err(), "{uri}");
        }
        let upper = RibbonSharedThemeInclude::new();
        upper.set_source(Some(
            Uri::absolute("ferres://FerroRibbon/Themes/Shared.xaml").unwrap(),
        ));
        let lower = RibbonSharedThemeInclude::new();
        lower.set_source(Some(
            Uri::absolute("ferres://ferroribbon/Themes/Shared.xaml").unwrap(),
        ));
        assert!(*upper.try_loaded().unwrap() == *lower.try_loaded().unwrap());
    });
}

#[test]
fn shared_resource_notifications_survive_duplicate_paths_and_release_detached_consumers() {
    crate::tests::run_standalone(|| {
        let _framework_theme = crate::tests::NativeThemeScope::new();
        use std::cell::Cell;
        for runtime in [false, true] {
            let first = load(runtime);
            let second = load(runtime);
            let extra = load(runtime);
            let first_provider = first.resources().merged_dictionaries().get(0);
            let second_provider = second.resources().merged_dictionaries().get(0);
            let extra_provider = extra.resources().merged_dictionaries().get(0);
            extra.resources().remove_merged_dictionary(&extra_provider);
            first
                .resources()
                .add_merged_dictionary(extra_provider.clone());
            let shared = generic(&first).merged_dictionaries_snapshot()[0]
                .resource_dictionary()
                .unwrap();
            let a = Rc::new(Cell::new(0));
            let b = Rc::new(Cell::new(0));
            let sub_a = {
                let count = a.clone();
                first.resources_changed(move |_| count.set(count.get() + 1))
            };
            let sub_b = {
                let count = b.clone();
                second.resources_changed(move |_| count.set(count.get() + 1))
            };
            shared.set(
                "NativeSharingNotificationProbe".into(),
                Some(Rc::new(1_i32)),
            );
            assert_eq!((a.get(), b.get()), (1, 1));
            first.resources().remove_merged_dictionary(&first_provider);
            a.set(0);
            b.set(0);
            shared.set(
                "NativeSharingNotificationProbe".into(),
                Some(Rc::new(2_i32)),
            );
            assert_eq!(
                (a.get(), b.get()),
                (1, 1),
                "remaining path must stay subscribed"
            );
            first.resources().remove_merged_dictionary(&extra_provider);
            a.set(0);
            b.set(0);
            shared.set(
                "NativeSharingNotificationProbe".into(),
                Some(Rc::new(3_i32)),
            );
            assert_eq!(
                (a.get(), b.get()),
                (0, 1),
                "last detached path must stop notifications"
            );
            let weak_first = first.downgrade();
            drop(first);
            assert!(
                weak_first.upgrade().is_none(),
                "cache must not own consumer"
            );
            second
                .resources()
                .remove_merged_dictionary(&second_provider);
            b.set(0);
            shared.remove(&"NativeSharingNotificationProbe".into());
            assert_eq!(b.get(), 0);
            let weak_second = second.downgrade();
            drop(second);
            assert!(weak_second.upgrade().is_none());
            sub_a.dispose();
            sub_b.dispose();
        }
    });
}

#[test]
fn default_palette_shared_roots_follow_source_direct_load_identity() {
    crate::tests::run_standalone(|| {
        let _framework_theme = crate::tests::NativeThemeScope::new();
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/generic-resource-sharing.json"))
                .unwrap();
        let a = RibbonThemeResources::new();
        let b = RibbonThemeResources::new();
        let a = a
            .merged_dictionaries()
            .get(0)
            .resource_dictionary()
            .unwrap();
        let b = b
            .merged_dictionaries()
            .get(0)
            .resource_dictionary()
            .unwrap();
        let panel = load(false);
        let canonical = generic(&panel).merged_dictionaries_snapshot()[0]
            .resource_dictionary()
            .unwrap();
        let shared = &source["families"][0];
        assert_eq!(*a == *b, shared["directRootSame"].as_bool().unwrap());
        assert_eq!(
            *a == *canonical,
            shared["directRootSameAsGenericFamily"].as_bool().unwrap()
        );
        for row in shared["styles"].as_array().unwrap() {
            let name = row["key"].as_str().unwrap();
            let key = if let Some(name) = name.strip_prefix("type:") {
                ResourceKey::Type(source_types().iter().find(|ty| ty.name() == name).unwrap())
            } else {
                name.into()
            };
            let sa = theme(a.try_get_resource(&key, None).flatten().unwrap());
            let sb = theme(b.try_get_resource(&key, None).flatten().unwrap());
            let sc = theme(canonical.try_get_resource(&key, None).flatten().unwrap());
            assert_eq!(
                sa.ptr_eq(&sb),
                row["directSame"].as_bool().unwrap(),
                "{name}"
            );
            assert_eq!(
                sa.ptr_eq(&sc),
                row["directSameAsGenericFamily"].as_bool().unwrap(),
                "{name}"
            );
        }
        a.set("PaletteRootProbe".into(), Some(Rc::new(1_i32)));
        assert!(
            b.try_get_resource(&"PaletteRootProbe".into(), None)
                .is_none()
        );
        assert!(
            canonical
                .try_get_resource(&"PaletteRootProbe".into(), None)
                .is_none()
        );
    });
}

#[test]
fn compiled_shared_factories_follow_recreated_native_applications_without_retaining_them() {
    use ferroui_base::WeakRef;
    use ferroui_controls::Application;
    use std::cell::RefCell;
    type PreviousApplicationResources = (Rc<dyn IResourceDictionary>, WeakRef<Application>);
    ferroui_base::ferro_thread_local! {
        static PREVIOUS: RefCell<Option<PreviousApplicationResources>> = const { RefCell::new(None) };
    }
    // Keep the native dispatcher thread. Unchanged PerTest isolation supplies
    // a fresh Application for each dispatch, exercising real lifecycle services.
    crate::tests::run(|| {
        let _framework_theme = crate::tests::NativeThemeScope::new();
        let panel = load(false);
        let shared = generic(&panel).merged_dictionaries_snapshot()[0]
            .resource_dictionary()
            .unwrap();
        let tip = theme(
            shared
                .try_get_resource(&"RibbonToolTipStyle".into(), None)
                .flatten()
                .unwrap(),
        );
        assert!(tip.based_on().is_some());
        PREVIOUS.with_borrow_mut(|slot| {
            *slot = Some((shared, Application::current().unwrap().downgrade()))
        });
    });
    crate::tests::run(|| {
        let _framework_theme = crate::tests::NativeThemeScope::new();
        let panel = load(true);
        let shared = generic(&panel).merged_dictionaries_snapshot()[0]
            .resource_dictionary()
            .unwrap();
        let (old_dictionary, old_application) = PREVIOUS.with_borrow_mut(Option::take).unwrap();
        assert!(
            old_application.upgrade().is_none(),
            "shared resources must not retain the previous Application"
        );
        assert!(
            *old_dictionary != *shared,
            "new Application needs its own deferred native markup context"
        );
        let tip = theme(
            shared
                .try_get_resource(&"RibbonToolTipStyle".into(), None)
                .flatten()
                .unwrap(),
        );
        assert!(tip.based_on().is_some());
    });
}
