pub mod button_consumer;
pub mod checked_consumer;
#[cfg(test)]
mod click_delivery_tests;
pub mod container_consumer;
#[cfg(test)]
mod gallery_preview_tests;
#[cfg(test)]
mod localization_tests;
pub mod menu_consumer;
#[cfg(test)]
mod menu_tests;
#[cfg(test)]
mod overflow_tests;
pub mod quick_access_consumer;
pub mod ribbon_consumer;
pub mod split_consumer;
#[cfg(test)]
mod split_tests;
#[cfg(test)]
mod startup_culture_tests;
pub mod tab_header_consumer;
#[cfg(test)]
mod test_process;
extern crate self as ferroribbon_markup_consumer;
pub mod async_command_consumer;
pub mod badge_consumer;
#[cfg(test)]
#[path = "../fixtures/catalog_item_cases.rs"]
mod catalog_item_cases;
#[cfg(test)]
mod catalog_item_tests;
pub mod command_consumer;
pub mod consumer_grid;
pub mod event_consumer;
pub mod grid_picker_consumer;
#[cfg(test)]
mod grid_picker_tests;
pub mod group_items_consumer;
pub mod icon_consumer;
pub mod image_event_consumer;
#[cfg(test)]
mod input_model_tests;
pub mod item_content_consumer;
pub mod item_contract_consumer;
pub mod metrics_consumer;
pub mod model_consumer;
pub mod root_model_consumer;
#[cfg(test)]
mod root_model_tests;
pub mod screen_tip_consumer;
pub mod scroll_consumer;
#[cfg(test)]
mod scroll_wheel_tests;
#[cfg(test)]
mod shortcut_tests;
pub mod theme_consumer;
pub mod view_model;
// The emitter clones Copy literals
// non-null ContentTemplate assignment arms (reproduced by Backstage.xaml).
// Keep these proven generator lints confined to generated code.
#[allow(clippy::clone_on_copy, clippy::if_same_then_else)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_xaml};
pub static CONSUMER_ASSEMBLY: ferroui_base::metadata::MarkupAssembly =
    ferroui_base::metadata::MarkupAssembly {
        name: "ferroribbon_markup_consumer",
        crate_name: "ferroribbon_markup_consumer",
        xmlns_definitions: &[],
        xmlns_prefixes: &[],
        metadata: &[(
            ferroui_base::metadata::MarkupAssembly::USE_COMPILED_BINDINGS_BY_DEFAULT,
            "True",
        )],
    };

#[cfg(test)]
mod group_items_tests;
#[cfg(test)]
mod item_automation_tests;
#[cfg(test)]
mod item_contract_tests;
#[cfg(test)]
mod owned_binding_tests;

#[cfg(test)]
mod tests {
    #[test]
    fn checked_controls_compile_bind_two_way_and_dispatch_downstream_override() {
        run(|| {
            let _theme = NativeThemeScope::new();
            use ferroribbon::controls::{IRibbonItem, RibbonCheckBox};
            use ferroui_base::data::core::{ValueType, ValueTypes};
            use ferroui_base::media::TextLineBounds;
            use ferroui_controls::{Control, StackPanel, TextBlock, Window};
            use std::rc::Rc;
            let compiled =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Checked.xaml")
                    .unwrap()
                    .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("Checked.xaml"),
                Some(&CONSUMER_ASSEMBLY),
            )
            .unwrap();
            for value in [compiled, runtime] {
                let panel = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap();
                let toggle = panel
                    .children()
                    .get(0)
                    .cast::<checked_consumer::ConsumerToggle>()
                    .unwrap();
                let check = panel.children().get(1).cast::<RibbonCheckBox>().unwrap();
                TextBlock::set_text_line_bounds_on(&panel, TextLineBounds::Tight);
                assert_eq!(check.is_checked(), Some(true));
                check.set_is_checked(None);
                assert_eq!(toggle.is_checked(), None);
                toggle.set_is_checked(Some(false));
                assert_eq!(check.is_checked(), Some(false));
                let w = Window::new();
                w.set_content(Some(Control::boxed(panel)));
                w.show();
                w.measure(Size::new(300., 200.));
                w.arrange(Rect::new(0., 0., 300., 200.));
                assert!(check.visual_children().count() > 0);
                assert!(toggle.visual_children().count() > 0);
                let label = check
                    .get_visual_descendants()
                    .find_map(|v| {
                        v.cast::<TextBlock>()
                            .filter(|label| label.name().as_deref() == Some("PART_Label"))
                    })
                    .unwrap();
                assert_eq!(label.text_line_bounds(), TextLineBounds::Full);
                let item = ValueTypes::try_cast(
                    &Control::boxed(toggle.clone()),
                    ValueType::of::<Rc<dyn IRibbonItem>>(),
                )
                .unwrap();
                assert!(item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap().invoke());
                assert_eq!(toggle.calls(), 1);
                assert_eq!(check.is_checked(), Some(true));
                let old_box = check
                    .get_visual_descendants()
                    .find_map(|v| {
                        v.cast::<ferroui_controls::Border>()
                            .filter(|b| b.name().as_deref() == Some("PART_Box"))
                    })
                    .unwrap();
                let background = old_box.background().unwrap();
                check.set_template(None);
                check.apply_template();
                check.set_is_checked(Some(false));
                assert!(
                    Rc::ptr_eq(&background, &old_box.background().unwrap()),
                    "clearing the template must stop updates to discarded checkbox parts"
                );
                w.close();
                w.set_content(None);
            }
        });
    }
    #[test]
    fn actual_owner_attached_binding_root_subclass_and_expanded_items_compile_and_load() {
        run(|| {
            let _theme = NativeThemeScope::new();
            use ferroribbon::controls::{RibbonButton, RibbonItemHelper};
            use ferroui_base::data::core::ValueTypes;
            use ferroui_controls::{Control, StackPanel, Window};
            let compiled = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/OwnerRibbon.xaml",
            )
            .unwrap()
            .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("OwnerRibbon.xaml"),
                Some(&CONSUMER_ASSEMBLY),
            )
            .unwrap();
            let mut weak_roots = Vec::new();
            for value in [compiled, runtime] {
                let panel = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap();
                let root = panel
                    .children()
                    .get(0)
                    .cast::<ribbon_consumer::ConsumerRibbon>()
                    .unwrap();
                weak_roots.push(root.downgrade());
                let detached = panel.children().get(1).cast::<RibbonButton>().unwrap();
                assert!(
                    RibbonItemHelper::get_owner(&detached)
                        .unwrap()
                        .owner_ribbon()
                        .unwrap()
                        .ptr_eq(&root)
                );
                assert_eq!(root.tab_strip_items().count(), 1);
                let group = root.tabs().get(0).unwrap().groups().get(0).unwrap();
                let extra = group.slide_out_items().get(0).unwrap();
                assert!(
                    RibbonItemHelper::get_owner(&extra)
                        .unwrap()
                        .as_object()
                        .ptr_eq(&root)
                );
                let w = Window::new();
                w.set_content(Some(Control::boxed(panel)));
                w.show();
                w.measure(Size::new(800., 300.));
                w.arrange(Rect::new(0., 0., 800., 300.));
                ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                assert!(root.templates() > 0);
                assert!(group.slide_out_button().unwrap().is_visible());
                group.open_slide_out();
                assert!(group.is_slide_out_open());
                group.close_slide_out();
                w.close();
                w.set_content(None);
            }
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert!(
                weak_roots.iter().all(|r| r.upgrade().is_none()),
                "Owner bindings must not create a native root/child reference cycle"
            );
        });
    }
    #[test]
    fn named_ribbon_owner_binding_lifetime_matrix() {
        run(|| {
            let _theme = NativeThemeScope::new();
            use ferroui_base::data::core::ValueTypes;
            use ferroui_controls::{Control, StackPanel, Window};
            let source = include_str!("OwnerRibbon.xaml");
            let mut results = Vec::new();
            for (name,xaml) in [
                ("source",source.to_owned()),
                ("no-inline-resources",source.replace("<StackPanel.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source=\"ferres://FerroRibbon/Themes/Ribbon.xaml\" /></ResourceDictionary.MergedDictionaries></ResourceDictionary></StackPanel.Resources>","")),
                ("base-type",source.replace("local:ConsumerRibbon", "r:Ribbon")),
                ("unmounted",source.to_owned()),
                ("no-owner-bindings",source.replace(" r:RibbonItemHelper.Owner=\"{Binding ElementName=OwnerRoot}\"", "")),
                ("no-name",source.replace(" x:Name=\"OwnerRoot\"", "").replace(" r:RibbonItemHelper.Owner=\"{Binding ElementName=OwnerRoot}\"", "")),
                ("no-command-bindings",source.replace(" Command=\"{Binding Command}\" CommandParameter=\"{Binding Value}\"", "")),
            ] {
                let weak={
                    let value=ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(&xaml,Some(&CONSUMER_ASSEMBLY)).unwrap();
                    let panel=ValueTypes::as_object(&*value).unwrap().cast::<StackPanel>().unwrap();
                    let root=panel.children().get(0).cast::<ferroribbon::controls::Ribbon>().unwrap();
                    let weak=root.downgrade();
                    let w=Window::new();if name!="unmounted" {w.set_content(Some(Control::boxed(panel)));w.show();w.measure(Size::new(800.,300.));w.arrange(Rect::new(0.,0.,800.,300.));
                    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                    w.close();w.set_content(None);}weak
                };
                ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                results.push((name,weak.upgrade().is_some()));
            }
            assert!(
                results.iter().all(|(_, retained)| !retained),
                "retained roots: {results:?}"
            );
        });
    }
    #[test]
    fn source_style_ribbon_compiled_and_runtime_reuses_native_controls_and_commands() {
        run(|| {
            let _theme = NativeThemeScope::new();
            use ferroribbon::controls::{IRibbonItem, Ribbon, RibbonButton};
            use ferroui_base::data::core::{ValueType, ValueTypes};
            use ferroui_controls::{Control, Window};
            use std::rc::Rc;
            let compiled =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Ribbon.xaml")
                    .unwrap()
                    .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("Ribbon.xaml"),
                Some(&CONSUMER_ASSEMBLY),
            )
            .unwrap();
            for value in [compiled, runtime] {
                let ribbon = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<Ribbon>()
                    .unwrap();
                let vm = ribbon.data_context().unwrap();
                let vm = ValueTypes::try_convert(
                    Some(&vm),
                    ValueType::of::<Rc<command_consumer::CommandViewModel>>(),
                )
                .flatten()
                .unwrap();
                let vm = vm
                    .downcast_ref::<Rc<command_consumer::CommandViewModel>>()
                    .unwrap();
                let window = Window::new();
                window.set_content(Some(Control::boxed(ribbon.clone())));
                window.show();
                window.measure(Size::new(800., 200.));
                window.arrange(Rect::new(0., 0., 800., 200.));
                assert_eq!(ribbon.selected_tab_id().as_deref(), Some("home"));
                assert_eq!(
                    ribbon.reduction_strategy(),
                    ferroribbon_core::layout::RibbonReductionStrategy::GroupByGroup
                );
                let button = ribbon
                    .tabs()
                    .get(0)
                    .unwrap()
                    .groups()
                    .get(0)
                    .unwrap()
                    .items()
                    .get(0)
                    .unwrap()
                    .cast::<RibbonButton>()
                    .unwrap();
                let item = ValueTypes::try_cast(
                    &Control::boxed(button),
                    ValueType::of::<Rc<dyn IRibbonItem>>(),
                )
                .unwrap();
                assert!(item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap().invoke());
                assert_eq!(vm.executed(), 1);
                assert!(ribbon.select_tab_id("insert"));
                assert_eq!(ribbon.selected_tab_id().as_deref(), Some("insert"));
                window.close();
            }
        });
    }
    #[test]
    fn groups_panel_enums_and_scroll_interface_are_reusable_from_compiled_and_runtime_markup() {
        run(|| {
            use ferroribbon::{
                controls::RibbonPanelPresentation,
                primitives::{IRibbonScrollContent, RibbonGroupsPanel},
            };
            use ferroui_base::data::core::{ValueType, ValueTypes};
            use std::rc::Rc;
            let compiled = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/GroupsPanel.xaml",
            )
            .unwrap()
            .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("GroupsPanel.xaml"),
                Some(&CONSUMER_ASSEMBLY),
            )
            .unwrap();
            for value in [compiled, runtime] {
                let p = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<RibbonGroupsPanel>()
                    .unwrap();
                assert_eq!(p.panel_presentation(), RibbonPanelPresentation::Titles);
                assert_eq!(p.group_spacing(), 1.25);
                assert_eq!(p.groups().iter().count(), 1);
                let value = ferroui_controls::Control::boxed(p.clone());
                let interface =
                    ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRibbonScrollContent>>())
                        .unwrap();
                assert!(
                    interface
                        .downcast_ref::<Rc<dyn IRibbonScrollContent>>()
                        .unwrap()
                        .as_object()
                        .ptr_eq(&p)
                );
            }
        });
    }
    #[test]
    fn real_group_source_style_commands_native_inheritance_and_container_virtuals() {
        run(|| {
            let _theme = NativeThemeScope::new();
            use ferroribbon::controls::{IRibbonItem, RibbonButton, RibbonItemsContainer};
            use ferroui_base::data::core::{ValueType, ValueTypes};
            use ferroui_controls::{Control, Window};
            use std::rc::Rc;
            let compiled =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Groups.xaml")
                    .unwrap()
                    .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("Groups.xaml"),
                Some(&CONSUMER_ASSEMBLY),
            )
            .unwrap();
            for value in [compiled, runtime] {
                let group = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<container_consumer::ConsumerRibbonGroup>()
                    .unwrap();
                let window = Window::new();
                window.set_content(Some(Control::boxed(group.clone())));
                window.show();
                window.measure(Size::new(600., 300.));
                window.arrange(Rect::new(0., 0., 600., 300.));
                assert_eq!(group.template_count(), 1);
                assert_eq!(group.header().as_deref(), Some("Clipboard"));
                assert_eq!(group.items().count(), 3);
                assert_eq!(group.items_panel().children().count(), 3);
                assert!(group.dialog_launcher().unwrap().is_visible());
                assert!(
                    group
                        .items_panel()
                        .children()
                        .get(0)
                        .ptr_eq(&group.items().get(0).unwrap())
                );
                let vm = ValueTypes::try_convert(
                    Some(&group.data_context().unwrap()),
                    ValueType::of::<Rc<command_consumer::CommandViewModel>>(),
                )
                .flatten()
                .unwrap();
                let vm = vm
                    .downcast_ref::<Rc<command_consumer::CommandViewModel>>()
                    .unwrap();
                assert!(
                    group.dialog_launcher_command().is_some(),
                    "launcher command binding"
                );
                assert_eq!(
                    group
                        .dialog_launcher_command_parameter()
                        .and_then(|p| p.downcast_ref::<i32>().copied()),
                    Some(7),
                    "launcher parameter binding"
                );
                group.open_dialog_launcher().unwrap();
                assert_eq!(vm.executed(), 1);
                group
                    .items()
                    .get(0)
                    .unwrap()
                    .cast::<RibbonButton>()
                    .unwrap()
                    .perform_click();
                assert_eq!(vm.executed(), 2);
                let custom = group
                    .items()
                    .get(2)
                    .unwrap()
                    .cast::<container_consumer::ConsumerRibbonItemsContainer>()
                    .unwrap();
                assert_eq!(
                    custom
                        .items()
                        .get(0)
                        .unwrap()
                        .cast::<RibbonButton>()
                        .unwrap()
                        .current_size(),
                    ferroribbon_core::RibbonItemSize::Medium
                );
                assert!(
                    custom
                        .host()
                        .unwrap()
                        .children()
                        .get(0)
                        .ptr_eq(&custom.items().get(0).unwrap())
                );
                let boxed = Control::boxed(custom.clone());
                let item =
                    ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItem>>())
                        .flatten()
                        .unwrap();
                let item = item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap();
                let copy = item.create_linked_copy().unwrap();
                assert!(
                    copy.cast::<container_consumer::ConsumerRibbonItemsContainer>()
                        .is_some()
                );
                assert_eq!(
                    copy.cast::<RibbonItemsContainer>().unwrap().items().count(),
                    1
                );
                window.close();
            }
        });
    }

    #[test]
    fn real_container_content_collections_templates_and_compiled_notifications() {
        run(|| {
            let _theme = NativeThemeScope::new();
            use ferroribbon::controls::{RibbonButton, RibbonButtonGroup, RibbonStackPanel};
            use ferroui_base::data::core::ValueTypes;
            use ferroui_controls::{Control, StackPanel, TextBlock, Window};
            let compiled = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/Containers.xaml",
            )
            .unwrap()
            .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("Containers.xaml"),
                Some(&CONSUMER_ASSEMBLY),
            )
            .unwrap();
            for value in [compiled, runtime] {
                let root = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap();
                let group = root.children().get(0).cast::<RibbonButtonGroup>().unwrap();
                let stack = root.children().get(1).cast::<RibbonStackPanel>().unwrap();
                let count = root.children().get(2).cast::<TextBlock>().unwrap();
                let name = root.children().get(3).cast::<TextBlock>().unwrap();
                assert!(
                    ferroui_base::data::model::ModelTypes::find(&group.items())
                        .unwrap()
                        .as_notify_collection_changed(&group.items())
                        .is_some()
                );
                assert_eq!(group.items().count(), 2);
                assert_eq!(stack.items().count(), 2);
                assert_eq!(count.text().as_deref(), Some("2"));
                assert_eq!(name.text().as_deref(), Some("First"));
                let window = Window::new();
                window.set_content(Some(Control::boxed(root)));
                window.show();
                window.measure(Size::new(800., 300.));
                window.arrange(Rect::new(0., 0., 800., 300.));
                assert!(
                    group
                        .host()
                        .unwrap()
                        .children()
                        .get(0)
                        .ptr_eq(&group.items().get(0).unwrap())
                );
                let host = stack.host().unwrap().cast::<StackPanel>().unwrap();
                assert_eq!(host.spacing(), 7.5);
                assert_eq!(
                    host.orientation(),
                    ferroui_base::layout::Orientation::Vertical
                );
                let third = RibbonButton::new();
                third.set_name(Some("Third".into()));
                group.items().add(Some(third.upcast()));
                assert_eq!(count.text().as_deref(), Some("3"));
                group.items().move_item(2, 0);
                assert_eq!(name.text().as_deref(), Some("Third"));
                group.items().remove_at(0);
                assert_eq!(count.text().as_deref(), Some("2"));
                assert_eq!(name.text().as_deref(), Some("First"));
                window.close();
            }
        });
    }

    use super::*;
    use ferroribbon::primitives::RibbonUniformGridPanel;
    use ferroui_base::input::KeyboardNavigation;
    use ferroui_base::metadata::{MarkupType, MarkupTyped};
    use ferroui_base::threading::CancellationToken;
    use ferroui_base::{Rect, Ref, Size, StaticType, TypeInfo};
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
        FerroTestApplicationAttribute, FerroTestAssembly, HeadlessUnitTestSession,
    };
    #[test]
    fn screen_tip_library_resource_include_compiled_and_runtime_across_assemblies() {
        run(|| {
            use ferroribbon::controls::RibbonScreenTip;
            use ferroui_base::data::core::ValueTypes;
            use ferroui_controls::{Control, StackPanel, TextBlock, Window};
            let compiled = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/ScreenTipResources.xaml",
            )
            .unwrap()
            .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("ScreenTipResources.xaml"),
                Some(&CONSUMER_ASSEMBLY),
            )
            .unwrap();
            for value in [compiled, runtime] {
                let root = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap();
                let tip = root.children().get(0).cast::<RibbonScreenTip>().unwrap();
                let w = Window::new();
                w.set_width(500.);
                w.set_height(300.);
                w.set_content(Some(Control::boxed(root)));
                w.show();
                w.measure(Size::new(500., 300.));
                w.arrange(Rect::new(0., 0., 500., 300.));
                assert_eq!(tip.title_text(), "Library");
                let description = tip
                    .get_visual_descendants()
                    .filter_map(|v| v.cast::<TextBlock>())
                    .find(|v| v.name().as_deref() == Some("PART_Description"))
                    .unwrap();
                assert_eq!(description.text().as_deref(), Some("External dictionary"));
                tip.set_description(Some("Changed".into()));
                assert_eq!(description.text().as_deref(), Some("Changed"));
                w.close();
            }
        });
    }
    #[test]
    fn screen_tip_compiled_runtime_bindings_subclass_and_source_service() {
        run(|| {
            use ferroribbon::controls::{RibbonScreenTip, RibbonScreenTipService};
            use ferroui_base::data::core::ValueTypes;
            use ferroui_controls::{Control, TextBlock};
            let compiled = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/ScreenTip.xaml",
            )
            .unwrap()
            .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("ScreenTip.xaml"),
                Some(&CONSUMER_ASSEMBLY),
            )
            .unwrap();
            for value in [compiled, runtime] {
                let tip = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<screen_tip_consumer::ConsumerScreenTip>()
                    .unwrap();
                assert_eq!(tip.style_key(), <RibbonScreenTip as StaticType>::TYPE);
                tip.apply_template();
                assert_eq!(tip.applications(), 1);
                let texts = || {
                    tip.get_visual_descendants()
                        .filter_map(|v| v.cast::<TextBlock>())
                        .collect::<Vec<_>>()
                };
                assert!(
                    texts()
                        .iter()
                        .any(|t| t.text().as_deref() == Some("AB (Ctrl+S)"))
                );
                let vm = ValueTypes::as_object(&*tip.data_context().unwrap())
                    .unwrap()
                    .cast::<icon_consumer::IconViewModel>()
                    .unwrap();
                vm.set_icon_text("Changed".into());
                assert_eq!(tip.title().as_deref(), Some("Changed"));
                tip.set_title(Some("Two way".into()));
                assert_eq!(vm.icon_text(), "Two way");
                tip.set_is_extended(true);
                assert_eq!(tip.max_width(), 440.);
                let copy =
                    RibbonScreenTipService::create(None, Some(Control::boxed(tip.clone())), None)
                        .unwrap()
                        .unwrap();
                let copy = ValueTypes::as_object(&*copy)
                    .unwrap()
                    .cast::<RibbonScreenTip>()
                    .unwrap();
                assert_eq!(copy.title(), tip.title());
                assert!(!copy.is_extended());
                assert!(copy.template().is_none());
                let model = ferroribbon_core::model::RibbonScreenTip::new();
                model.set_title(Some("Model".into()));
                let output = RibbonScreenTipService::create(None, Some(model), None)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    ValueTypes::as_object(&*output)
                        .unwrap()
                        .cast::<RibbonScreenTip>()
                        .unwrap()
                        .title()
                        .as_deref(),
                    Some("Model")
                );
                tip.set_template(None);
                tip.apply_template();
            }
        });
    }
    #[test]
    fn public_runtime_theme_service_tracks_compiled_and_runtime_consumer_elements() {
        // Source EnsureResources intentionally retains a static cache. Actual item
        // constructors in other tests now populate it; preserve this test's exact
        // empty-cache/discovery assertions by running it in a fresh process.
        const ISOLATED: &str = "FERRORIBBON_THEME_CONSUMER_ISOLATED";
        if std::env::var_os(ISOLATED).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "tests::public_runtime_theme_service_tracks_compiled_and_runtime_consumer_elements", "--nocapture"])
                .env(ISOLATED, "1")
                .output().unwrap();
            assert!(
                output.status.success(),
                "isolated theme consumer failed:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        run(|| {
            use ferroribbon::theming::RibbonTheme;
            use ferroribbon_core::theming::{
                RibbonChromeStyle, RibbonThemePalette, RibbonThemeStyle,
            };
            use ferroui_base::{media::IBrush, styling::ThemeVariant};
            use ferroui_controls::{Button, StackPanel, primitives::TemplatedControl};
            use std::{cell::Cell, rc::Rc};
            let compiled = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/SharedControls.xaml",
            )
            .unwrap()
            .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("SharedControls.xaml"),
                Some(&CONSUMER_ASSEMBLY),
            )
            .unwrap();
            let compiled = ferroui_base::data::core::ValueTypes::as_object(&*compiled)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let app = Application::current().unwrap();
            let previous = app.resources();
            // The source's static cache reuses the constructor-created dictionary
            // from compiled App resources instead of merging another instance.
            let app_resources = ferroui_base::controls::ResourceDictionary::new();
            let nested = ferroui_base::controls::ResourceDictionary::new();
            let provider = compiled.resources().merged_dictionaries().get(0);
            let expected = provider
                .as_object()
                .unwrap()
                .to_ref()
                .cast::<ferroribbon::theming::RibbonThemeResources>()
                .unwrap();
            compiled.resources().merged_dictionaries().clear();
            nested.add_merged_dictionary(expected.clone());
            app_resources.add_merged_dictionary(nested);
            app.set_resources(app_resources);
            assert_eq!(RibbonTheme::ensure_resources(), expected);
            let count = Rc::new(Cell::new(0));
            let count2 = count.clone();
            let subscription = RibbonTheme::changed(move |sender, _| {
                assert!(sender.is_none());
                count2.set(count2.get() + 1);
            });
            for root in [
                compiled,
                ferroui_base::data::core::ValueTypes::as_object(&*runtime)
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap(),
            ] {
                RibbonTheme::apply_with_style(
                    Some(RibbonThemePalette::word()),
                    RibbonChromeStyle::Neutral,
                    RibbonThemeStyle::Office,
                )
                .unwrap();
                let button = root.children().get(1).cast::<Button>().unwrap();
                RibbonTheme::set_theme(Some(&root), Some(ThemeVariant::light())).unwrap();
                RibbonTheme::set_theme_brush(
                    Some(&button),
                    Some(TemplatedControl::foreground_property().as_property()),
                    Some("RibbonAccentBrush"),
                )
                .unwrap();
                let brush: Rc<dyn IBrush> = expected
                    .get_brush("RibbonAccentBrush", Some("Light"))
                    .unwrap()
                    .upcast::<ferroui_base::media::Brush>()
                    .into();
                assert!(*button.foreground().unwrap() == *brush);
                RibbonTheme::set_theme(Some(&root), Some(ThemeVariant::dark())).unwrap();
                assert_eq!(
                    button
                        .foreground()
                        .unwrap()
                        .as_solid_color_brush()
                        .unwrap()
                        .color(),
                    ferroribbon::theming::to_color(RibbonThemePalette::word().accent_dark)
                );
                RibbonTheme::apply_palette(Some(RibbonThemePalette::excel())).unwrap();
                assert_eq!(
                    button
                        .foreground()
                        .unwrap()
                        .as_solid_color_brush()
                        .unwrap()
                        .color(),
                    ferroribbon::theming::to_color(RibbonThemePalette::excel().accent_dark)
                );
                // App replacement does not invalidate the source's static cache.
                app.set_resources(ferroui_base::controls::ResourceDictionary::new());
                assert_eq!(RibbonTheme::resources(), expected);
            }
            assert_eq!(count.get(), 4);
            subscription.dispose();
            check_theme_metadata_xaml(&expected);
            check_public_changed_handlers_native_handle_views();
            app.set_resources(previous);
        });
    }
    fn check_theme_metadata_xaml(expected: &Ref<ferroribbon::theming::RibbonThemeResources>) {
        use ferroribbon::theming::*;
        use ferroui_controls::{Control, TextBlock, Window};
        use std::rc::Rc;
        let compiled = compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/ThemeMetadata.xaml",
        )
        .unwrap()
        .unwrap();
        let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("ThemeMetadata.xaml"),
            Some(&CONSUMER_ASSEMBLY),
        )
        .unwrap();
        for value in [compiled, runtime] {
            let root = ferroui_base::data::core::ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<theme_consumer::ThemeConsumer>()
                .unwrap();
            assert_eq!(root.color(), RibbonColor::from_rgb(170, 187, 204));
            assert_eq!(root.optional_color(), None);
            assert!(Rc::ptr_eq(
                &root.palette().unwrap(),
                &RibbonThemePalette::excel()
            ));
            assert_eq!(root.chrome(), RibbonChromeStyle::Neutral);
            assert_eq!(root.ribbon_style(), RibbonThemeStyle::Cad);
            assert_eq!(root.current_resources().as_ref(), Some(expected));
            assert!(Rc::ptr_eq(&root.presets(), &RibbonThemePalette::presets()));
            let w = Window::new();
            w.set_content(Some(Control::boxed(root.clone())));
            w.show();
            w.measure(Size::new(400., 300.));
            w.arrange(Rect::new(0., 0., 400., 300.));
            assert_eq!(
                root.children()
                    .get(0)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Excel")
            );
            assert_eq!(
                root.children()
                    .get(1)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("PowerPoint")
            );
            assert_eq!(
                root.children()
                    .get(2)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Excel")
            );
            root.set_presets(
                vec![
                    RibbonThemePalette::cad(),
                    RibbonThemePalette::word(),
                    RibbonThemePalette::outlook(),
                ]
                .into(),
            );
            assert_eq!(
                root.children()
                    .get(1)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Outlook")
            );
            assert_eq!(
                root.children()
                    .get(3)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Excel")
            );
            root.set_palette(Some(RibbonThemePalette::word()));
            assert_eq!(
                root.children()
                    .get(3)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Word")
            );
            root.set_optional_color(Some(RibbonColor::from_rgb(1, 2, 3)));
            assert_eq!(root.optional_color(), Some(RibbonColor::from_rgb(1, 2, 3)));
            w.close();
        }
    }
    #[test]
    fn static_event_subscriptions_work_in_compiled_and_runtime_markup() {
        run(|| {
            use event_consumer::StaticEventConsumer;
            for compiled in [true, false] {
                StaticEventConsumer::reset();
                let value = if compiled {
                    compiled_xaml::try_load(
                        None,
                        "ferres://ferroribbon_markup_consumer/StaticEvents.xaml",
                    )
                    .unwrap()
                    .unwrap()
                } else {
                    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                        include_str!("StaticEvents.xaml"),
                        Some(&CONSUMER_ASSEMBLY),
                    )
                    .unwrap()
                };
                assert!(
                    ferroui_base::data::core::ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<StaticEventConsumer>()
                        .is_some()
                );
                assert!(
                    StaticEventConsumer::TYPE
                        .markup()
                        .unwrap()
                        .find_event("Ping")
                        .unwrap()
                        .is_static
                );
                StaticEventConsumer::raise();
                assert_eq!(StaticEventConsumer::count(), 1);
                StaticEventConsumer::raise_signal();
                assert_eq!(StaticEventConsumer::count(), 2);
                let root = ferroui_base::data::core::ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StaticEventConsumer>()
                    .unwrap();
                let bound = root.tag().expect("bound synthesized remover");
                let remover = bound
                    .downcast_ref::<ferroui_base::metadata::MarkupDelegate>()
                    .expect("method delegate");
                remover.try_invoke(&[None]).unwrap();
                let meta = StaticEventConsumer::TYPE.markup().unwrap();
                let handler = ferroui_base::metadata::MarkupDelegate::for_method(
                    Some(std::rc::Rc::new(root.clone())),
                    meta,
                    meta.find_methods("OnPing").next().unwrap(),
                );
                remover
                    .try_invoke(&[ferroui_base::metadata::into_markup_value(handler)])
                    .unwrap();
                StaticEventConsumer::raise();
                assert_eq!(StaticEventConsumer::count(), 2);
                let signal_handler = ferroui_base::metadata::MarkupDelegate::for_method(
                    Some(std::rc::Rc::new(root.clone())),
                    meta,
                    meta.find_methods("OnChanged").next().unwrap(),
                );
                (meta.find_methods("remove_Signal").next().unwrap().invoke)(&[
                    ferroui_base::metadata::into_markup_value(signal_handler),
                ])
                .unwrap();
                StaticEventConsumer::raise_signal();
                assert_eq!(StaticEventConsumer::count(), 2);
                StaticEventConsumer::reset();
            }
        });
    }
    #[test]
    fn compiled_and_runtime_method_values_preserve_metadata_errors_and_weak_owner() {
        run(|| {
            use event_consumer::StaticEventConsumer;
            use ferroui_base::metadata::{MarkupDelegate, into_markup_value};
            for compiled in [true, false] {
                StaticEventConsumer::reset();
                let value = if compiled {
                    compiled_xaml::try_load(
                        None,
                        "ferres://ferroribbon_markup_consumer/MethodValues.xaml",
                    )
                    .unwrap()
                    .unwrap()
                } else {
                    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                        include_str!("MethodValues.xaml"),
                        Some(&CONSUMER_ASSEMBLY),
                    )
                    .unwrap()
                };
                let root = ferroui_base::data::core::ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StaticEventConsumer>()
                    .unwrap();
                let weak = root.downgrade();
                let boxed = root.tag().unwrap();
                let handler = boxed.downcast_ref::<MarkupDelegate>().unwrap().clone();
                assert_eq!(handler.method().unwrap().name, "OnValue");
                assert!(handler.try_invoke(&[]).is_err());
                assert_eq!(StaticEventConsumer::count(), 0);
                let result = handler.try_invoke(&[into_markup_value(41i32)]).unwrap();
                assert_eq!(
                    ferroui_base::metadata::from_markup_value::<i32>(&result),
                    Some(42)
                );
                assert_eq!(StaticEventConsumer::count(), 1);
                drop(root);
                drop(value);
                assert!(weak.upgrade().is_none());
                assert!(!handler.is_target_alive());
                assert!(
                    handler
                        .try_invoke(&[into_markup_value(41i32)])
                        .unwrap()
                        .is_none()
                );
                assert_eq!(StaticEventConsumer::count(), 1);
            }
        });
    }
    #[test]
    fn named_markup_handler_errors_stop_later_handlers_in_both_loading_modes() {
        run(|| {
            use event_consumer::StaticEventConsumer;
            use ferroui_base::metadata::{MarkupDelegate, MarkupInvokeError};
            for compiled in [true, false] {
                StaticEventConsumer::reset();
                let value = if compiled {
                    compiled_xaml::try_load(
                        None,
                        "ferres://ferroribbon_markup_consumer/FailHandlers.xaml",
                    )
                    .unwrap()
                    .unwrap()
                } else {
                    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                        include_str!("FailHandlers.xaml"),
                        Some(&CONSUMER_ASSEMBLY),
                    )
                    .unwrap()
                };
                let root = ferroui_base::data::core::ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StaticEventConsumer>()
                    .unwrap();
                let meta = StaticEventConsumer::TYPE.markup().unwrap();
                StaticEventConsumer::subscribe_signal(Some(MarkupDelegate::for_method(
                    Some(std::rc::Rc::new(root.clone())),
                    meta,
                    meta.find_methods("OnChanged").next().unwrap(),
                )));
                assert_eq!(
                    StaticEventConsumer::try_raise_signal(),
                    Err(MarkupInvokeError::Failed("handler-failed".into()))
                );
                assert_eq!(
                    StaticEventConsumer::count(),
                    1,
                    "later handler must not run"
                );
                StaticEventConsumer::reset();
            }
        });
    }
    fn check_public_changed_handlers_native_handle_views() {
        use event_consumer::StaticEventConsumer;
        use ferroribbon::theming::{RibbonChromeStyle, RibbonTheme};
        use ferroui_base::{
            data::core::ValueTypes,
            metadata::{MarkupDelegate, into_markup_value},
        };
        StaticEventConsumer::reset();
        let root = StaticEventConsumer::new();
        let weak = root.downgrade();
        let markup = StaticEventConsumer::TYPE.markup().unwrap();
        let method = markup.find_methods("OnChanged").next().unwrap();
        let root_value = into_markup_value(root.clone());
        let base = root.clone().upcast::<ferroui_controls::Control>();
        let base_value = into_markup_value(base.clone());
        assert!(ValueTypes::as_object(&**root_value.as_ref().unwrap()).is_some());
        let first = MarkupDelegate::for_method(root_value.clone(), markup, method);
        let second = MarkupDelegate::for_method(base_value.clone(), markup, method);
        assert_eq!(first, second);
        assert!(!first.ptr_eq(&second));
        let theme = RibbonTheme::MARKUP;
        let changed = theme.find_event("Changed").unwrap();
        assert!(changed.is_static && changed.handler_nullable);
        (changed.add)(&[None]).unwrap();
        (changed.remove.unwrap())(&[None]).unwrap();
        (changed.add)(&[into_markup_value(first.clone())]).unwrap();
        (theme.find_methods("add_Changed").next().unwrap().invoke)(&[into_markup_value(
            second.clone(),
        )])
        .unwrap();
        RibbonTheme::apply_chrome_style(RibbonChromeStyle::Neutral).unwrap();
        assert_eq!(StaticEventConsumer::count(), 2);
        (theme.find_methods("remove_Changed").next().unwrap().invoke)(&[into_markup_value(
            MarkupDelegate::for_method(base_value.clone(), markup, method),
        )])
        .unwrap();
        RibbonTheme::apply_chrome_style(RibbonChromeStyle::Neutral).unwrap();
        assert_eq!(StaticEventConsumer::count(), 3);
        (changed.remove.unwrap())(&[into_markup_value(MarkupDelegate::for_method(
            root_value.clone(),
            markup,
            method,
        ))])
        .unwrap();
        RibbonTheme::apply_chrome_style(RibbonChromeStyle::Neutral).unwrap();
        assert_eq!(StaticEventConsumer::count(), 3);
        drop(root);
        drop(base);
        drop(root_value);
        drop(base_value);
        drop(first);
        drop(second);
        assert!(weak.upgrade().is_none());
    }
    fn assert_common_properties(control: &command_consumer::CommandConsumer) {
        use ferroribbon::controls::RibbonSimplifiedLabel;
        use ferroribbon_core::{RibbonItemSize, RibbonSimplifiedVisibility};
        assert_eq!(control.id().as_deref(), Some("consumer.command"));
        assert_eq!(control.label().as_deref(), Some("Paste"));
        assert_eq!(control.key_tip().as_deref(), Some("V"));
        assert_eq!(control.shortcut().as_deref(), Some("Ctrl+V"));
        assert_eq!(
            control.icon().unwrap().downcast_ref::<String>().unwrap(),
            "paste"
        );
        assert_eq!(
            control
                .large_icon()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "paste-large"
        );
        assert_eq!(control.size(), RibbonItemSize::Large);
        assert_eq!(control.current_size(), RibbonItemSize::Small);
        assert!(control.is_simplified());
        assert!(!control.actual_show_label());
        assert_eq!(
            control.simplified_visibility(),
            RibbonSimplifiedVisibility::Pinned
        );
        assert_eq!(control.simplified_label(), RibbonSimplifiedLabel::Show);
        assert!(!control.show_label());
        assert!(!control.can_add_to_quick_access());
        assert_eq!(
            control.size_definition().as_deref(),
            Some("Large,Medium,Small")
        );
        let object = ferroui_controls::Control::boxed(control.to_ref());
        let item = ferroui_base::data::core::ValueTypes::try_convert(
            Some(&object),
            ferroui_base::data::core::ValueType::of::<
                std::rc::Rc<dyn ferroribbon::controls::IRibbonItem>,
            >(),
        )
        .flatten()
        .unwrap();
        let item = item
            .downcast_ref::<std::rc::Rc<dyn ferroribbon::controls::IRibbonItem>>()
            .unwrap();
        assert_eq!(item.id(), Some("consumer.command".into()));
        item.apply_layout(ferroribbon::controls::RibbonItemLayout::new(
            RibbonItemSize::Medium,
            None,
        ));
        assert_eq!(control.layouts(), 1);
        assert_eq!(control.current_size(), RibbonItemSize::Medium);
        control.set_current_size(RibbonItemSize::Small);
    }
    pub(super) struct NativeThemeScope {
        app: Ref<Application>,
        style: std::rc::Rc<dyn ferroui_base::styling::IStyle>,
    }
    impl NativeThemeScope {
        pub(super) fn new() -> Self {
            let app = Application::current().unwrap();
            let style = ferroui_themes_simple::SimpleTheme::new().as_style();
            app.styles().add(style.clone());
            Self { app, style }
        }
    }
    impl Drop for NativeThemeScope {
        fn drop(&mut self) {
            self.app.styles().remove(self.style.clone());
        }
    }
    fn build() -> AppBuilder {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    }
    const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
        name: "FerroRibbon.Compiled.Consumer",
        test_application: Some(FerroTestApplicationAttribute::new(build)),
        test_isolation: None,
    };
    pub(super) fn run(test: impl FnOnce() + Send + 'static) {
        let session = HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY));
        dispatch_in_session(&session, test);
    }
    // Standalone application scenarios need a fresh dispatcher thread: source
    // static services intentionally outlive replacement Applications in a session.
    pub(super) fn run_standalone(test: impl FnOnce() + Send + 'static) {
        let session = HeadlessUnitTestSession::start_new(build);
        dispatch_in_session(&session, test);
    }
    fn dispatch_in_session(
        session: &HeadlessUnitTestSession,
        test: impl FnOnce() + Send + 'static,
    ) {
        session
            .dispatch(
                move || {
                    ferroribbon::register_types();
                    TypeInfo::register_namespaces(&[(
                        "ferroribbon_markup_consumer::factory_consumer",
                        "FerroRibbon.Consumer",
                    )]);
                    TypeInfo::register_all(&[
                        <factory_consumer::ConsumerElementFactory as StaticType>::TYPE,
                    ]);
                    TypeInfo::register_namespaces(&[(
                        "ferroribbon_markup_consumer::ribbon_consumer",
                        "FerroRibbon.Consumer",
                    )]);
                    TypeInfo::register_namespaces(&[(
                        "ferroribbon_markup_consumer::checked_consumer",
                        "FerroRibbon.Consumer",
                    )]);
                    TypeInfo::register_namespaces(&[(
                        "ferroribbon_markup_consumer::quick_access_consumer",
                        "FerroRibbon.Consumer",
                    )]);
                    TypeInfo::register_all(&[
                        <checked_consumer::ConsumerToggle as StaticType>::TYPE,
                        <ribbon_consumer::ConsumerRibbon as StaticType>::TYPE,
                        <quick_access_consumer::ConsumerQuickAccessToolBar as StaticType>::TYPE,
                    ]);
                    TypeInfo::register_namespaces(&[(
                        "ferroribbon_markup_consumer::tab_header_consumer",
                        "FerroRibbon.Consumer",
                    )]);
                    TypeInfo::register_all(&[
                        <tab_header_consumer::ConsumerTabHeader as StaticType>::TYPE,
                    ]);
                    TypeInfo::register_namespaces(&[(
                        "ferroribbon_markup_consumer::container_consumer",
                        "FerroRibbon.Consumer",
                    )]);
                    TypeInfo::register_all(&[
                        <container_consumer::ConsumerRibbonGroup as StaticType>::TYPE,
                        <container_consumer::ConsumerRibbonItemsContainer as StaticType>::TYPE,
                    ]);
                    TypeInfo::register_namespaces(&[(
                        "ferroribbon_markup_consumer::button_consumer",
                        "FerroRibbon.Consumer",
                    )]);
                    TypeInfo::register_all(&[
                        <button_consumer::ConsumerRibbonButton as StaticType>::TYPE,
                        <button_consumer::ClickDeliveryConsumer as StaticType>::TYPE,
                    ]);
                    event_consumer::register();
                    model_consumer::register();
                    icon_sequence_consumer::register();
                    item_contract_consumer::register();
                    group_items_consumer::register();
                    ferroui_base::metadata::MarkupAssembly::register(&CONSUMER_ASSEMBLY);
                    TypeInfo::register_namespaces(&[
                        (
                            "ferroribbon_markup_consumer::screen_tip_consumer",
                            "FerroRibbon.Consumer",
                        ),
                        (
                            "ferroribbon_markup_consumer::theme_consumer",
                            "FerroRibbon.Consumer",
                        ),
                        (
                            "ferroribbon_markup_consumer::badge_consumer",
                            "FerroRibbon.Consumer",
                        ),
                        (
                            "ferroribbon_markup_consumer::item_content_consumer",
                            "FerroRibbon.Consumer",
                        ),
                        (
                            "ferroribbon_markup_consumer::icon_consumer",
                            "FerroRibbon.Consumer",
                        ),
                        (
                            "ferroribbon_markup_consumer::command_consumer",
                            "FerroRibbon.Consumer",
                        ),
                        (
                            "ferroribbon_markup_consumer::scroll_consumer",
                            "FerroRibbon.Consumer",
                        ),
                        (
                            "ferroribbon_markup_consumer::metrics_consumer",
                            "FerroRibbon.Consumer",
                        ),
                        (
                            "ferroribbon_markup_consumer::consumer_grid",
                            "FerroRibbon.Consumer",
                        ),
                    ]);
                    ferroui_markup_xaml::register_types();
                    MarkupType::register_all(&[<view_model::GridViewModel as MarkupTyped>::MARKUP]);
                    MarkupType::register_all(&[
                        <command_consumer::CommandViewModel as MarkupTyped>::MARKUP,
                    ]);
                    TypeInfo::register_all(&[
                        <screen_tip_consumer::ConsumerScreenTip as StaticType>::TYPE,
                        <theme_consumer::ThemeConsumer as StaticType>::TYPE,
                        <badge_consumer::ConsumerBadge as StaticType>::TYPE,
                        <item_content_consumer::ConsumerItemContent as StaticType>::TYPE,
                        <icon_consumer::ConsumerIcon as StaticType>::TYPE,
                        <icon_consumer::IconViewModel as StaticType>::TYPE,
                        <command_consumer::CommandConsumer as StaticType>::TYPE,
                        <scroll_consumer::ConsumerScroll as StaticType>::TYPE,
                        <scroll_consumer::ConsumerAdaptive as StaticType>::TYPE,
                        <consumer_grid::ConsumerGrid as StaticType>::TYPE,
                        <metrics_consumer::MetricsConsumer as StaticType>::TYPE,
                    ]);
                    compiled_markup::register();
                    test();
                },
                CancellationToken::none(),
            )
            .wait()
            .unwrap();
    }
    #[test]
    fn independent_compiled_and_runtime_shared_resources_badge_binding_and_template_replacement() {
        run(|| {
            use ferroribbon::{controls::RibbonKeyTip, theming::RibbonThemeResources};
            use ferroui_controls::{Control, StackPanel, TextBlock, Window};
            for (file, text) in [
                ("SharedControls.xaml", include_str!("SharedControls.xaml")),
                ("BadgeTemplate.xaml", include_str!("BadgeTemplate.xaml")),
            ] {
                let compiled = compiled_xaml::try_load(
                    None,
                    &format!("ferres://ferroribbon_markup_consumer/{file}"),
                )
                .unwrap()
                .unwrap();
                let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                    text, None, None, None, false,
                )
                .unwrap();
                for value in [compiled, runtime] {
                    let root = ferroui_base::data::core::ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<Control>()
                        .unwrap();
                    let window = Window::new();
                    window.set_width(500.);
                    window.set_height(300.);
                    window.set_content(Some(Control::boxed(root.clone())));
                    window.show();
                    window.measure(Size::new(500., 300.));
                    window.arrange(Rect::new(0., 0., 500., 300.));
                    if file == "SharedControls.xaml" {
                        let panel = root.cast::<StackPanel>().unwrap();
                        let badge = panel.children().get(0).cast::<RibbonKeyTip>().unwrap();
                        assert_eq!(badge.text().as_deref(), Some("AB"));
                        assert!(!badge.is_hit_test_visible());
                        assert!(!KeyboardNavigation::get_is_tab_stop(&badge));
                        let vm = badge
                            .data_context()
                            .unwrap()
                            .downcast_ref::<Ref<icon_consumer::IconViewModel>>()
                            .unwrap()
                            .clone();
                        badge.set_text(Some("CD".into()));
                        assert_eq!(vm.icon_text(), "CD");
                        vm.set_icon_text("EF".into());
                        assert_eq!(badge.text().as_deref(), Some("EF"));
                        assert_eq!(
                            badge
                                .get_visual_descendants()
                                .find_map(|v| v.cast::<TextBlock>())
                                .unwrap()
                                .text()
                                .as_deref(),
                            Some("EF")
                        );
                        let button = panel
                            .children()
                            .get(1)
                            .cast::<ferroui_controls::Button>()
                            .unwrap();
                        assert_eq!(RibbonKeyTip::get_key_tip(&button).as_deref(), Some("R"));
                        assert!(button.presenter().is_some());
                        let resources = panel
                            .resources()
                            .merged_dictionaries()
                            .get(0)
                            .as_object()
                            .unwrap()
                            .to_ref()
                            .cast::<RibbonThemeResources>()
                            .unwrap();
                        assert_eq!(resources.merged_dictionaries().count(), 1);
                    } else {
                        let badge = root.cast::<badge_consumer::ConsumerBadge>().unwrap();
                        assert_eq!(badge.style_key(), <RibbonKeyTip as StaticType>::TYPE);
                        let text = badge
                            .get_visual_descendants()
                            .find_map(|v| v.cast::<TextBlock>())
                            .unwrap();
                        assert_eq!(text.text().as_deref(), Some("Custom"));
                        badge.set_text(Some("Changed".into()));
                        assert_eq!(text.text().as_deref(), Some("Changed"));
                    }
                    window.close();
                }
            }
        });
    }
    #[test]
    fn compiled_and_runtime_dynamic_resources_follow_native_dictionary_identity_and_theme() {
        run(|| {
            use ferroribbon::theming::RibbonThemeResources;
            use ferroribbon_core::theming::{
                RibbonChromeStyle, RibbonThemePalette, RibbonThemeStyle,
            };
            use ferroui_base::controls::{IResourceProvider, ResourceDictionary};
            use ferroui_base::styling::ThemeVariant;
            use ferroui_controls::Border;
            use std::rc::Rc;
            let shared = ResourceDictionary::new();
            shared.set("ConsumerSharedMarker", Some(Rc::new(42i32)));
            let resources = RibbonThemeResources::with_shared_resources(shared.clone());
            use ferroui_base::data::core::{ValueType, ValueTypes};
            let boxed: ferroui_base::BoxedValue = Rc::new(resources.clone());
            let converted = ValueTypes::try_convert(
                Some(&boxed),
                ValueType::of::<Rc<dyn ferroui_base::controls::IResourceDictionary>>(),
            )
            .flatten()
            .expect("inherited native dictionary interface");
            let dictionary = converted
                .downcast_ref::<Rc<dyn ferroui_base::controls::IResourceDictionary>>()
                .unwrap();
            assert!(dictionary.as_object().unwrap().to_ref().ptr_eq(&resources));
            let provider: Rc<dyn IResourceProvider> =
                resources.clone().upcast::<ResourceDictionary>().into();
            let app = Application::current().unwrap();
            app.resources().merged_dictionaries().add(provider.clone());
            let compiled = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/ThemeBindings.xaml",
            )
            .unwrap()
            .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                include_str!("ThemeBindings.xaml"),
                None,
                None,
                None,
                false,
            )
            .unwrap();
            for value in [compiled, runtime] {
                let root = value.downcast_ref::<Ref<Border>>().unwrap();
                let window = ferroui_controls::Window::new();
                window.set_content(Some(ferroui_controls::Control::boxed(root.clone())));
                window.show();
                root.set_value(
                    ThemeVariant::requested_theme_variant_property(),
                    Some(ThemeVariant::light()),
                );
                let light = resources
                    .get_brush("RibbonAccentBrush", Some("Light"))
                    .unwrap();
                assert!(
                    root.background()
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .to_ref()
                        .ptr_eq(&light)
                );
                resources
                    .apply_palette_and_chrome(
                        Some(RibbonThemePalette::excel()),
                        RibbonChromeStyle::Neutral,
                    )
                    .unwrap();
                assert_eq!(
                    root.background()
                        .unwrap()
                        .as_solid_color_brush()
                        .unwrap()
                        .color(),
                    ferroribbon::theming::to_color(RibbonThemePalette::excel().accent)
                );
                assert!(
                    root.background()
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .to_ref()
                        .ptr_eq(&light)
                );
                root.set_value(
                    ThemeVariant::requested_theme_variant_property(),
                    Some(ThemeVariant::dark()),
                );
                let dark = resources
                    .get_brush("RibbonAccentBrush", Some("Dark"))
                    .unwrap();
                assert!(
                    root.background()
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .to_ref()
                        .ptr_eq(&dark)
                );
                resources
                    .apply(
                        Some(RibbonThemePalette::cad()),
                        RibbonChromeStyle::Neutral,
                        RibbonThemeStyle::Cad,
                    )
                    .unwrap();
                assert_eq!(
                    root.background()
                        .unwrap()
                        .as_solid_color_brush()
                        .unwrap()
                        .color(),
                    ferroribbon::theming::to_color(RibbonThemePalette::cad().accent_dark)
                );
                assert!(
                    root.background()
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .to_ref()
                        .ptr_eq(&dark)
                );
                root.set_value(
                    ThemeVariant::requested_theme_variant_property(),
                    Some(RibbonThemeResources::high_contrast_variant()),
                );
                let high_contrast = resources
                    .get_brush("RibbonAccentBrush", Some("HighContrast"))
                    .unwrap();
                assert!(
                    root.background()
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .to_ref()
                        .ptr_eq(&high_contrast)
                );
                assert_eq!(
                    provider
                        .try_get_resource(&"ConsumerSharedMarker".into(), None)
                        .flatten()
                        .unwrap()
                        .downcast_ref::<i32>(),
                    Some(&42)
                );
                window.close();
            }
            app.resources().merged_dictionaries().remove(&provider);
            assert!(
                <RibbonThemeResources as StaticType>::TYPE
                    .default_constructor()
                    .is_some()
            );
        });
    }
    #[test]
    fn compiled_and_runtime_item_content_share_props_bindings_metrics_and_derived_layout() {
        run(|| {
            use ferroribbon::primitives::{
                RibbonIconPresenter, RibbonItemContentPart, RibbonMetrics,
            };
            use ferroribbon_core::RibbonItemSize;
            use ferroui_base::data::core::{ValueType, ValueTypes};
            use ferroui_controls::{TextBlock, Viewbox};
            let compiled = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/ItemContent.xaml",
            )
            .unwrap()
            .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                include_str!("ItemContent.xaml"),
                None,
                None,
                None,
                false,
            )
            .unwrap();
            for value in [compiled, runtime] {
                let p = value
                    .downcast_ref::<Ref<item_content_consumer::ConsumerItemContent>>()
                    .unwrap();
                assert_eq!(p.children().snapshot().len(), 5);
                assert_eq!(p.label().as_deref(), Some("AB"));
                assert_eq!(p.item_size(), RibbonItemSize::Large);
                assert_eq!(p.part(), RibbonItemContentPart::All);
                assert_eq!(p.metrics(), Some(RibbonMetrics::compact()));
                let model = ValueTypes::try_convert(
                    p.data_context().as_ref(),
                    ValueType::of::<Ref<icon_consumer::IconViewModel>>(),
                )
                .flatten()
                .unwrap();
                let model = model
                    .downcast_ref::<Ref<icon_consumer::IconViewModel>>()
                    .unwrap();
                p.set_label(Some("Edit".into()));
                assert_eq!(model.icon_text(), "Edit");
                model.set_icon_text("Paste Special".into());
                p.measure(Size::INFINITY);
                p.arrange(Rect::from_size(p.desired_size()));
                assert!(p.measures() > 0);
                assert_eq!(p.desired_size().height, 62.);
                assert_eq!(
                    p.children()
                        .get(1)
                        .cast::<TextBlock>()
                        .unwrap()
                        .line_height(),
                    13.
                );
                let icon = p.children().get(0).cast::<RibbonIconPresenter>().unwrap();
                let box1 = icon.children().get(0).cast::<Viewbox>().unwrap();
                let element = box1.child().unwrap();
                p.set_item_size(RibbonItemSize::Medium);
                assert!(box1.child().is_none());
                assert!(
                    icon.children()
                        .get(0)
                        .cast::<Viewbox>()
                        .unwrap()
                        .child()
                        .unwrap()
                        .ptr_eq(&element)
                );
                p.set_part(RibbonItemContentPart::LabelAndChevron);
                assert!(!icon.has_icon());
                assert!(element.parent().is_none());
            }
        });
    }
    #[test]
    fn item_content_part_metadata_preserves_unknown_integers_and_nullable_metrics() {
        run(|| {
            use ferroribbon::primitives::{RibbonItemContent, RibbonItemContentPart};
            let xaml = r#"<rs:RibbonItemContent xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:rs="https://github.com/wieslawsoltes/RibbonSpace" Label="Text" Part="99" ItemSize="99" Metrics="{x:Null}"/>"#;
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                xaml, None, None, None, false,
            )
            .unwrap();
            let p = value.downcast_ref::<Ref<RibbonItemContent>>().unwrap();
            assert_eq!(p.part(), RibbonItemContentPart::from_raw(99));
            assert!(p.metrics().is_none());
            p.measure(Size::INFINITY);
            assert_eq!(p.desired_size().height, 22.);
        });
    }
    #[test]
    fn font_mirroring_compiled_and_runtime_uses_native_property_and_original_callback_order() {
        run(|| {
            use ferroribbon::compat::FontIcon;
            use ferroui_base::RelativePoint;
            use ferroui_base::data::core::ValueTypes;
            use ferroui_base::media::FlowDirection;
            use ferroui_controls::StackPanel;
            let compiled = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/FontMirroring.xaml",
            )
            .unwrap()
            .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("FontMirroring.xaml"),
                Some(&crate::CONSUMER_ASSEMBLY),
            )
            .unwrap();
            for value in [compiled, runtime] {
                let panel = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap();
                let first = panel.children().get(0).cast::<FontIcon>().unwrap();
                let second = panel.children().get(1).cast::<FontIcon>().unwrap();
                assert!(first.mirrored_when_right_to_left());
                assert_eq!(first.flow_direction(), FlowDirection::RightToLeft);
                assert_eq!(first.render_transform().unwrap().value().m11, -1.);
                assert_eq!(first.render_transform_origin(), RelativePoint::CENTER);
                assert!(second.mirrored_when_right_to_left());
                assert_eq!(second.flow_direction(), FlowDirection::RightToLeft);
                assert!(second.render_transform().is_none());
                assert_eq!(second.render_transform_origin(), RelativePoint::TOP_LEFT);
                second.set_mirrored_when_right_to_left(false);
                second.set_mirrored_when_right_to_left(true);
                assert_eq!(second.render_transform().unwrap().value().m11, -1.);
                first.set_mirrored_when_right_to_left(false);
                assert_eq!(first.render_transform().unwrap().value().m11, 1.);
            }
        });
    }
    #[test]
    fn compiled_and_runtime_font_inputs_initialize_before_clone_with_library_metadata() {
        run(|| {
            use ferroribbon::{compat::FontIcon, primitives::RibbonIconPresenter};
            use ferroui_base::data::core::{ValueType, ValueTypes};
            use ferroui_base::media::{FontFamily, FontStyle, FontWeight};
            let compiled =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/FontIcon.xaml")
                    .unwrap()
                    .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                include_str!("FontIcon.xaml"),
                None,
                None,
                None,
                false,
            )
            .unwrap();
            for value in [compiled, runtime] {
                let p = value.downcast_ref::<Ref<RibbonIconPresenter>>().unwrap();
                assert!(p.has_icon());
                let source =
                    ValueTypes::try_convert(p.icon().as_ref(), ValueType::of::<Ref<FontIcon>>())
                        .flatten()
                        .unwrap();
                let source = source.downcast_ref::<Ref<FontIcon>>().unwrap();
                let clone = p.children().get(0).cast::<FontIcon>().unwrap();
                assert_ne!(&clone, source);
                assert_eq!(clone.glyph().as_deref(), Some("\u{e000}"));
                assert_eq!(clone.font_size(), 22.);
                assert!(clone.width().is_nan() && clone.height().is_nan());
                assert_eq!(clone.font_family(), FontFamily::new("Consumer Symbols"));
                assert_eq!(clone.font_style(), FontStyle::Italic);
                assert_eq!(clone.font_weight(), FontWeight::Bold);
                assert_eq!(
                    clone
                        .foreground()
                        .unwrap()
                        .as_solid_color_brush()
                        .unwrap()
                        .color()
                        .to_uint32(),
                    0xff0000ff
                );
                source.set_glyph(Some("Changed".into()));
                assert_eq!(clone.glyph().as_deref(), Some("\u{e000}"));
                p.set_icon_size(30.);
                assert_eq!(
                    p.children()
                        .get(0)
                        .cast::<FontIcon>()
                        .unwrap()
                        .glyph()
                        .as_deref(),
                    Some("Changed")
                );
            }
        });
    }
    #[test]
    fn runtime_glyph_uses_the_same_native_font_icon_and_source_resource_key() {
        run(|| {
            use ferroribbon::{compat::FontIcon, primitives::RibbonIconPresenter};
            use ferroui_base::media::FontFamily;
            use std::rc::Rc;
            let resources = Application::current().unwrap().resources();
            let key = "SymbolThemeFontFamily".into();
            let previous = resources.try_get_value(&key);
            resources.set(
                key.clone(),
                Some(Rc::new(FontFamily::new("Runtime Symbols"))),
            );
            let xaml = r##"<rs:RibbonIconPresenter xmlns="https://github.com/ferroui" xmlns:rs="https://github.com/wieslawsoltes/RibbonSpace" Icon="&#xE000;" IconSize="24" Foreground="#123456"/>"##;
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                xaml, None, None, None, false,
            )
            .unwrap();
            let p = value.downcast_ref::<Ref<RibbonIconPresenter>>().unwrap();
            let icon = p.children().get(0).cast::<FontIcon>().unwrap();
            assert_eq!(icon.glyph().as_deref(), Some("\u{e000}"));
            assert_eq!(icon.font_size(), 24.);
            assert_eq!(icon.font_family(), FontFamily::new("Runtime Symbols"));
            assert_eq!(icon.foreground(), p.foreground());
            resources.remove(&key);
            if let Some(previous) = previous {
                resources.set(key, previous);
            }
        });
    }
    #[test]
    fn compiled_icon_binding_uses_library_properties_and_downstream_grid_override() {
        run(|| {
            use ferroui_base::BoxedValue;
            use ferroui_base::data::core::{ValueType, ValueTypes};
            use ferroui_controls::{TextBlock, Viewbox};
            use std::rc::Rc;
            let value =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Icon.xaml")
                    .unwrap()
                    .unwrap();
            let p = value
                .downcast_ref::<Ref<icon_consumer::ConsumerIcon>>()
                .unwrap();
            assert!(p.has_icon());
            assert_eq!(p.icon_size(), 18.);
            let text = p.children().get(0).cast::<TextBlock>().unwrap();
            assert_eq!(text.text().as_deref(), Some("AB"));
            assert_eq!(
                text.foreground()
                    .unwrap()
                    .as_solid_color_brush()
                    .unwrap()
                    .color()
                    .to_uint32(),
                0xff123456
            );
            let model = ValueTypes::try_convert(
                p.data_context().as_ref(),
                ValueType::of::<Ref<icon_consumer::IconViewModel>>(),
            )
            .flatten()
            .unwrap();
            let model = model
                .downcast_ref::<Ref<icon_consumer::IconViewModel>>()
                .unwrap();
            model.set_icon_text("Long".into());
            assert!(p.children().get(0).cast::<Viewbox>().is_some());
            p.set_icon_size(24.);
            assert_eq!(model.icon_size(), 24.);
            p.measure(Size::INFINITY);
            p.arrange(Rect::new(0., 0., 24., 24.));
            assert!(p.measures() > 0);
            model.set_icon_text("M0,0 L8,8".into());
            assert!(p.has_icon());
            let normalized: BoxedValue = Rc::new(p.clone());
            assert!(
                ValueTypes::try_convert(
                    Some(&normalized),
                    ValueType::of::<Ref<ferroribbon::primitives::RibbonIconPresenter>>()
                )
                .flatten()
                .is_some()
            );
        });
    }
    #[test]
    fn runtime_icon_object_property_preserves_element_identity_across_size_changes() {
        run(|| {
            use ferroribbon::primitives::RibbonIconPresenter;
            use ferroui_controls::{Border, Viewbox};
            let xaml = r#"<rs:RibbonIconPresenter xmlns="https://github.com/ferroui" xmlns:rs="https://github.com/wieslawsoltes/RibbonSpace" IconSize="20"><rs:RibbonIconPresenter.Icon><Border Width="8" Height="6" /></rs:RibbonIconPresenter.Icon></rs:RibbonIconPresenter>"#;
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                xaml, None, None, None, false,
            )
            .unwrap();
            let p = value.downcast_ref::<Ref<RibbonIconPresenter>>().unwrap();
            assert!(p.has_icon());
            let old = p.children().get(0).cast::<Viewbox>().unwrap();
            let icon = old.child().unwrap().cast::<Border>().unwrap();
            p.set_icon_size(32.);
            assert!(old.child().is_none());
            let new = p.children().get(0).cast::<Viewbox>().unwrap();
            assert!(new.child().unwrap().ptr_eq(&icon));
            assert_eq!(new.width(), 32.);
            p.set_icon(None);
            assert!(icon.visual_parent().is_none() && icon.parent().is_none());
        });
    }
    #[test]
    fn independent_actual_push_dropdown_and_subclass_templates_bindings_and_clicks() {
        run(|| {
            use ferroribbon::controls::{IRibbonItem, RibbonButton, RibbonDropDownButton};
            use ferroui_base::data::core::{ValueType, ValueTypes};
            use ferroui_controls::{Control, StackPanel, Window};
            use std::rc::Rc;
            let _theme = NativeThemeScope::new();
            let compiled =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Buttons.xaml")
                    .unwrap()
                    .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                include_str!("Buttons.xaml"),
                None,
                None,
                None,
                false,
            )
            .unwrap();
            for value in [compiled, runtime] {
                let root = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap();
                let button = root.children().get(0).cast::<RibbonButton>().unwrap();
                let derived = root
                    .children()
                    .get(1)
                    .cast::<button_consumer::ConsumerRibbonButton>()
                    .unwrap();
                assert!(
                    root.children()
                        .get(2)
                        .cast::<RibbonDropDownButton>()
                        .unwrap()
                        .show_chevron()
                );
                let window = Window::new();
                window.set_content(Some(Control::boxed(&root)));
                window.show();
                window.measure(Size::new(700., 300.));
                window.arrange(Rect::new(0., 0., 700., 300.));
                let vm = root.data_context().unwrap();
                let vm = ValueTypes::try_convert(
                    Some(&vm),
                    ValueType::of::<Rc<command_consumer::CommandViewModel>>(),
                )
                .flatten()
                .unwrap();
                let vm = vm
                    .downcast_ref::<Rc<command_consumer::CommandViewModel>>()
                    .unwrap();
                assert!(
                    button.data_context().is_some(),
                    "button inherited DataContext"
                );
                assert!(
                    button.command_parameter().is_some(),
                    "button parameter binding"
                );
                let errors = ferroui_controls::DataValidationErrors::get_errors(&button)
                    .unwrap_or_default()
                    .iter()
                    .map(|value| {
                        value
                            .downcast_ref::<ferroui_base::data::BindingError>()
                            .map(ToString::to_string)
                            .or_else(|| value.downcast_ref::<String>().cloned())
                            .unwrap_or_else(|| "unknown error value".into())
                    })
                    .collect::<Vec<_>>();
                let expression =
                    ferroui_base::data::BindingOperations::get_binding_expression_base(
                        &button,
                        ferroui_controls::Button::command_property().as_property(),
                    )
                    .unwrap();
                let error = expression
                    .as_any()
                    .downcast_ref::<ferroui_base::data::core::BindingExpression>()
                    .map(|e| e.error_type());
                assert!(
                    button.command().is_some(),
                    "button Command binding: {errors:?} {error:?}"
                );
                assert_eq!(
                    button.command_parameter().unwrap().downcast_ref::<i32>(),
                    Some(&7)
                );
                assert!(button.is_enabled());
                assert!(
                    button.is_effectively_enabled(),
                    "native effective enablement"
                );
                assert!(Rc::ptr_eq(
                    &ferroribbon::commands::source_core_command(&button.command().unwrap())
                        .unwrap(),
                    &vm.command()
                ));
                button.perform_click();
                assert_eq!(vm.executed(), 1);
                button.set_command_parameter(Some(Rc::new(-1i32)));
                assert_eq!(vm.value(), -1);
                assert!(!button.is_effectively_enabled());
                button.perform_click();
                assert_eq!(vm.executed(), 1);
                let content = button
                    .get_visual_descendants()
                    .find_map(|child| child.cast::<ferroribbon::primitives::RibbonItemContent>())
                    .unwrap();
                assert_eq!(content.label().as_deref(), Some("Paste"));
                button.set_label(Some("Updated".into()));
                assert_eq!(content.label().as_deref(), Some("Updated"));
                let object = Control::boxed(&derived);
                let interface =
                    ValueTypes::try_convert(Some(&object), ValueType::of::<Rc<dyn IRibbonItem>>())
                        .flatten()
                        .unwrap();
                let interface = interface.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap();
                let copy = interface
                    .create_linked_copy()
                    .unwrap()
                    .cast::<button_consumer::ConsumerRibbonButton>()
                    .unwrap();
                derived.set_label(Some("Linked".into()));
                assert_eq!(copy.label().as_deref(), Some("Linked"));
                window.close();
            }
        });
    }
    #[test]
    fn independent_source_simple_items_compiled_and_runtime_templates() {
        run(|| {
            use ferroribbon::controls::{RibbonLabel, RibbonSeparator};
            use ferroui_base::layout::Orientation;
            use ferroui_controls::{Control, StackPanel, TextBlock, Window};
            let _theme = NativeThemeScope::new();
            let compiled = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/SimpleItems.xaml",
            )
            .unwrap()
            .unwrap();
            let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                include_str!("SimpleItems.xaml"),
                None,
                None,
                None,
                false,
            )
            .unwrap();
            for value in [compiled, runtime] {
                let panel = ferroui_base::data::core::ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap();
                let separator = panel.children().get(0).cast::<RibbonSeparator>().unwrap();
                let label = panel.children().get(1).cast::<RibbonLabel>().unwrap();
                assert_eq!(separator.orientation(), Orientation::Horizontal);
                assert_eq!(label.label().as_deref(), Some("Ready"));
                let window = Window::new();
                window.set_content(Some(Control::boxed(&panel)));
                window.show();
                separator.apply_template();
                label.apply_template();
                assert!(separator.get_visual_descendants().any(|c| {
                    c.cast::<ferroui_base::StyledElement>()
                        .is_some_and(|c| c.name().as_deref() == Some("PART_Line"))
                }));
                let text = label
                    .get_visual_descendants()
                    .find_map(|c| c.cast::<TextBlock>())
                    .unwrap();
                assert_eq!(text.text().as_deref(), Some("Ready"));
                label.set_label(Some("Updated".into()));
                assert_eq!(text.text().as_deref(), Some("Updated"));
                window.close();
            }
        });
    }
    #[test]
    fn compiled_core_command_binding_inherits_native_base_and_two_way_parameter() {
        run(|| {
            let _theme = NativeThemeScope::new();
            use ferroribbon::controls::{IRibbonCommandSource, RibbonItemHelper};
            use ferroui_base::data::core::{ValueType, ValueTypes};
            use ferroui_controls::{Control, Window};
            use std::rc::Rc;
            let value =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Command.xaml")
                    .unwrap()
                    .unwrap();
            let control = value
                .downcast_ref::<Ref<command_consumer::CommandConsumer>>()
                .unwrap();
            let vm = control.data_context().unwrap();
            let vm = ValueTypes::try_convert(
                Some(&vm),
                ValueType::of::<Rc<command_consumer::CommandViewModel>>(),
            )
            .flatten()
            .unwrap();
            let vm = vm
                .downcast_ref::<Rc<command_consumer::CommandViewModel>>()
                .unwrap();
            assert_common_properties(control);
            assert!(
                ferroribbon::commands::source_core_command(&control.command().unwrap()).is_some()
            );
            assert_eq!(
                control.command_parameter().unwrap().downcast_ref::<i32>(),
                Some(&7)
            );
            let object = Control::boxed(control);
            let source = ValueTypes::try_convert(
                Some(&object),
                ValueType::of::<Rc<dyn IRibbonCommandSource>>(),
            )
            .flatten()
            .unwrap();
            let source = source
                .downcast_ref::<Rc<dyn IRibbonCommandSource>>()
                .unwrap();
            assert!(RibbonItemHelper::execute(
                source.as_ref(),
                control.command_parameter().as_ref()
            ));
            assert_eq!(vm.executed(), 1);
            control.set_command_parameter(Some(Rc::new(-1i32)));
            assert!(!control.is_enabled());
            assert_eq!(vm.value(), -1);
            let window = Window::new();
            window.set_content(Some(Control::boxed(control)));
            window.show();
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert!(control.is_loaded());
            window.close();
            assert!(!control.is_loaded());
        });
    }
    #[test]
    fn runtime_core_command_binding_and_abstract_base_constructor_metadata() {
        run(|| {
            let _theme = NativeThemeScope::new();
            use ferroribbon::controls::RibbonControlBase;
            assert!(
                <RibbonControlBase as StaticType>::TYPE
                    .default_constructor()
                    .is_none()
            );
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                include_str!("Command.xaml"),
                None,
                None,
                None,
                false,
            )
            .unwrap();
            let control = value
                .downcast_ref::<Ref<command_consumer::CommandConsumer>>()
                .unwrap();
            assert_common_properties(control);
            assert!(
                ferroribbon::commands::source_core_command(&control.command().unwrap()).is_some()
            );
            assert_eq!(
                control.command_parameter().unwrap().downcast_ref::<i32>(),
                Some(&7)
            );
            control.set_command_parameter(Some(std::rc::Rc::new(-1i32)));
            assert!(!control.is_enabled());
        });
    }
    #[test]
    fn independent_compiled_linked_source_attached_binding_and_property_protocol() {
        run(|| {
            use ferroribbon::controls::RibbonItemHelper;
            use ferroui_base::input::InputElement;
            use ferroui_controls::Panel;
            let value =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Linked.xaml")
                    .unwrap()
                    .unwrap();
            let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
            let original = panel.children().get(0);
            let copy = panel.children().get(1);
            assert!(
                RibbonItemHelper::get_source_item(&copy)
                    .unwrap()
                    .ptr_eq(&original)
            );
            RibbonItemHelper::link_one_way(
                &original,
                &copy,
                InputElement::is_enabled_property().as_property(),
            );
            original.set_is_enabled(false);
            assert!(!copy.is_enabled());
            RibbonItemHelper::unlink(&copy);
            original.set_is_enabled(true);
            assert!(!copy.is_enabled());
            assert!(
                RibbonItemHelper::get_source_item(&copy)
                    .unwrap()
                    .ptr_eq(&original)
            );
        });
    }
    #[test]
    fn runtime_linked_source_attached_binding_uses_same_native_static_metadata() {
        run(|| {
            use ferroribbon::controls::RibbonItemHelper;
            use ferroui_controls::Panel;
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                include_str!("Linked.xaml"),
                None,
                None,
                None,
                false,
            )
            .unwrap();
            let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
            assert!(
                RibbonItemHelper::get_source_item(&panel.children().get(1))
                    .unwrap()
                    .ptr_eq(&panel.children().get(0))
            );
        });
    }
    #[test]
    fn compiled_scroll_subclass_and_foreign_contract_keep_content_identity() {
        run(|| {
            use ferroribbon::primitives::{IRibbonScrollContent, RibbonScrollPanel};
            use ferroui_base::data::core::{ValueType, ValueTypes};
            use ferroui_controls::Control;
            use std::rc::Rc;
            let value =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Scroll.xaml")
                    .unwrap()
                    .unwrap();
            let panel = value
                .downcast_ref::<Ref<scroll_consumer::ConsumerScroll>>()
                .unwrap();
            panel.measure(Size::new(100., 30.));
            panel.arrange(Rect::new(0., 0., 100., 30.));
            assert!(panel.has_overflow());
            assert_eq!(panel.arrangements(), 1);
            let child = panel.content().unwrap();
            assert!(child.ptr_eq(&panel.children().get(0)));
            let iface = ValueTypes::try_convert(
                Some(&Control::boxed(&child)),
                ValueType::of::<Rc<dyn IRibbonScrollContent>>(),
            )
            .flatten()
            .unwrap();
            let iface = iface
                .downcast_ref::<Rc<dyn IRibbonScrollContent>>()
                .unwrap();
            assert_eq!(iface.extent_width(), 300.5);
            assert!(iface.as_object().ptr_eq(&child));
            assert!(
                Control::from_boxed(&(Rc::new(iface.clone()) as ferroui_base::BoxedValue))
                    .unwrap()
                    .ptr_eq(&child)
            );
            panel.scroll_by(120.);
            panel.arrange(Rect::new(0., 0., 100., 30.));
            assert_eq!(child.bounds().x, -120.);
            panel
                .content()
                .unwrap()
                .cast::<scroll_consumer::ConsumerAdaptive>()
                .unwrap()
                .set_extent_width(75.);
            panel.measure(Size::new(100., 30.));
            panel.arrange(Rect::new(0., 0., 100., 30.));
            assert!(!panel.has_overflow());
            assert_eq!(panel.offset(), 0.);
            assert!(panel.clone().upcast::<RibbonScrollPanel>().ptr_eq(panel));
        });
    }
    #[test]
    fn runtime_scroll_content_property_uses_the_native_library_control() {
        run(|| {
            use ferroribbon::primitives::{RibbonScrollPanel, RibbonUniformGridPanel};
            let xaml = r#"<rs:RibbonScrollPanel xmlns="https://github.com/ferroui" xmlns:rs="https://github.com/wieslawsoltes/RibbonSpace" MeasureInfinite="True" UseLayoutRounding="False"><rs:RibbonScrollPanel.Content><rs:RibbonUniformGridPanel Columns="5" ItemWidth="30" UseLayoutRounding="False" /></rs:RibbonScrollPanel.Content></rs:RibbonScrollPanel>"#;
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                xaml, None, None, None, false,
            )
            .unwrap();
            let panel = value.downcast_ref::<Ref<RibbonScrollPanel>>().unwrap();
            let child = panel.content().unwrap();
            assert!(child.cast::<RibbonUniformGridPanel>().is_some());
            panel.measure(Size::new(100., 40.));
            panel.arrange(Rect::new(0., 0., 100., 40.));
            assert!(panel.has_overflow());
            assert!(child.ptr_eq(&panel.children().get(0)));
            panel.scroll_by(500.);
            assert_eq!(panel.offset(), 58.);
        });
    }
    #[test]
    fn compiled_metric_path_and_two_way_domain_enum_binding_follow_replacement() {
        run(|| {
            use ferroribbon::primitives::RibbonMetrics;
            use ferroribbon_core::RibbonDensity;
            use ferroui_controls::{StackPanel, TextBlock};
            let value = compiled_xaml::try_load(
                None,
                "ferres://ferroribbon_markup_consumer/MetricsBindings.xaml",
            )
            .unwrap()
            .unwrap();
            let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
            let source = panel
                .children()
                .get(0)
                .cast::<metrics_consumer::MetricsConsumer>()
                .unwrap();
            let label = panel.children().get(1).cast::<TextBlock>().unwrap();
            let target = panel
                .children()
                .get(2)
                .cast::<metrics_consumer::MetricsConsumer>()
                .unwrap();
            assert_eq!(label.text().as_deref(), Some("30"));
            assert_eq!(target.density(), RibbonDensity::Touch);
            assert!(std::sync::Arc::ptr_eq(
                &target.custom_metrics().unwrap(),
                &RibbonMetrics::touch()
            ));
            target.set_custom_metrics(None);
            assert!(source.custom_metrics().is_none());
            let custom = RibbonMetrics::comfortable().with_values(|v| v.row_height = 23.5);
            source.set_custom_metrics(Some(custom.clone()));
            assert!(std::sync::Arc::ptr_eq(
                &target.custom_metrics().unwrap(),
                &custom
            ));
            source.set_metrics(RibbonMetrics::compact());
            assert_eq!(label.text().as_deref(), Some("20"));
            target.set_density(RibbonDensity::from_raw(99));
            assert_eq!(source.density().raw(), 99);
            source.set_density(RibbonDensity::Comfortable);
            assert_eq!(target.density(), RibbonDensity::Comfortable);
        });
    }
    #[test]
    fn runtime_unknown_enum_and_null_preserve_the_core_representation() {
        run(|| {
            let xaml = r#"<local:MetricsConsumer xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:local="using:FerroRibbon.Consumer" xmlns:rs="https://github.com/wieslawsoltes/RibbonSpace" Density="99" OptionalDensity="{x:Null}" Metrics="{x:Static rs:RibbonMetrics.Compact}"/>"#;
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                xaml, None, None, None, false,
            )
            .unwrap();
            let consumer = value
                .downcast_ref::<Ref<metrics_consumer::MetricsConsumer>>()
                .unwrap();
            assert_eq!(consumer.density().raw(), 99);
            assert_eq!(consumer.optional_density(), None);
            assert!(std::sync::Arc::ptr_eq(
                &consumer.metrics(),
                &ferroribbon::primitives::RibbonMetrics::compact()
            ));
        });
    }
    #[test]
    fn compiled_foreign_enums_and_shared_metrics_use_original_domain_values() {
        run(|| {
            let value =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Metrics.xaml")
                    .unwrap()
                    .unwrap();
            let consumer = value
                .downcast_ref::<Ref<metrics_consumer::MetricsConsumer>>()
                .unwrap();
            assert_eq!(consumer.density(), ferroribbon_core::RibbonDensity::Touch);
            assert_eq!(
                consumer.optional_density(),
                Some(ferroribbon_core::RibbonDensity::Compact)
            );
            assert!(std::sync::Arc::ptr_eq(
                &consumer.metrics(),
                &ferroribbon::primitives::RibbonMetrics::touch()
            ));
        });
    }
    #[test]
    fn compiled_consumer_binds_library_properties_and_preserves_children() {
        run(|| {
            let value =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Grid.xaml")
                    .unwrap()
                    .unwrap();
            let panel = value.downcast_ref::<Ref<RibbonUniformGridPanel>>().unwrap();
            assert_eq!(panel.columns(), 2);
            assert_eq!(panel.children().count(), 3);
            let data = panel.data_context().unwrap();
            let model = data.downcast_ref::<view_model::GridViewModel>().unwrap();
            let first = panel.children().get(0);
            assert!(!KeyboardNavigation::get_is_tab_stop(&first));
            model.set_columns(3);
            assert_eq!(panel.columns(), 3);
            panel.set_columns(1);
            assert_eq!(model.columns(), 1);
            panel.set_first_row(1);
            assert_eq!(panel.first_row(), 1);
            panel.measure(Size::INFINITY);
            panel.arrange(Rect::from_size(panel.desired_size()));
            assert!(first.ptr_eq(&panel.children().get(0)));
            assert!(!KeyboardNavigation::get_is_tab_stop(&first));
        });
    }
    #[test]
    fn compiled_and_runtime_gallery_rows_restore_animated_local_tab_stop_by_identity() {
        run(|| {
            use ferroui_base::data::BindingPriority;
            use ferroui_base::data::core::ValueTypes;
            let property = KeyboardNavigation::is_tab_stop_property();
            for runtime in [false, true] {
                for local in [None, Some(false), Some(true)] {
                    for animated in [false, true] {
                        let value = if runtime {
                            ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                                include_str!("Grid.xaml"),
                                Some(&CONSUMER_ASSEMBLY),
                            )
                            .unwrap()
                        } else {
                            compiled_xaml::try_load(
                                None,
                                "ferres://ferroribbon_markup_consumer/Grid.xaml",
                            )
                            .unwrap()
                            .unwrap()
                        };
                        let panel = ValueTypes::as_object(&*value)
                            .unwrap()
                            .cast::<RibbonUniformGridPanel>()
                            .unwrap();
                        let child = panel.children().get(2);
                        let style = child
                            .set_value_with_priority(property, true, BindingPriority::Style)
                            .unwrap();
                        if let Some(value) = local {
                            child.set_value(property, value);
                        }
                        let animation = child
                            .set_value_with_priority(property, animated, BindingPriority::Animation)
                            .unwrap();
                        for _ in 0..3 {
                            for first_row in [0, 1] {
                                panel.set_first_row(first_row);
                                // The source's plain FirstRow setter does not invalidate layout.
                                panel.invalidate_measure();
                                panel.measure(Size::INFINITY);
                                panel.invalidate_arrange();
                                panel.arrange(Rect::from_size(panel.desired_size()));
                                assert_eq!(
                                    child.get_local_value(property),
                                    if first_row == 0 { Some(false) } else { local }
                                );
                                assert_eq!(child.get_value(property), animated);
                                assert!(child.ptr_eq(&panel.children().get(2)));
                            }
                        }
                        animation.dispose();
                        assert_eq!(child.get_value(property), local.unwrap_or(true));
                        assert_eq!(child.get_local_value(property), local);
                        style.dispose();
                    }
                }
            }
        });
    }
    #[test]
    fn downstream_subclass_overrides_virtual_and_calls_library_parent() {
        run(|| {
            let value =
                compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Derived.xaml")
                    .unwrap()
                    .unwrap();
            let panel = value
                .downcast_ref::<Ref<consumer_grid::ConsumerGrid>>()
                .unwrap();
            panel.measure(Size::INFINITY);
            assert_eq!(panel.desired_size(), Size::new(43., 10.));
            assert!(
                panel
                    .clone()
                    .upcast::<RibbonUniformGridPanel>()
                    .ptr_eq(panel)
            );
        });
    }

    #[test]
    fn runtime_markup_uses_the_same_library_class_and_plain_properties() {
        run(|| {
            let xaml = r#"<rs:RibbonUniformGridPanel xmlns="https://github.com/ferroui" xmlns:rs="https://github.com/wieslawsoltes/RibbonSpace" Columns="2" FirstRow="1" MaxRows="1" ItemWidth="20" ItemHeight="10"><Button/><Button/><Button/></rs:RibbonUniformGridPanel>"#;
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
                xaml, None, None, None, false,
            )
            .unwrap();
            let panel = value.downcast_ref::<Ref<RibbonUniformGridPanel>>().unwrap();
            assert_eq!(panel.columns(), 2);
            assert_eq!(panel.first_row(), 1);
            assert_eq!(panel.max_rows(), 1);
            assert_eq!(panel.children().count(), 3);
            panel.measure(Size::INFINITY);
            panel.arrange(Rect::from_size(panel.desired_size()));
            assert!(!KeyboardNavigation::get_is_tab_stop(
                &panel.children().get(0)
            ));
            assert!(KeyboardNavigation::get_is_tab_stop(
                &panel.children().get(2)
            ));
        });
    }
}

#[cfg(test)]
mod key_tip_peers;

pub mod combo_consumer;
#[cfg(test)]
mod combo_tests;
#[cfg(test)]
mod keyboard_tests;
#[cfg(test)]
mod model_bindings_tests;
#[cfg(test)]
mod quick_access_tests;
#[cfg(test)]
mod state_tests;
pub mod text_input_consumer;
#[cfg(test)]
mod text_input_tests;
#[cfg(test)]
mod visibility_tests;

pub mod spinner_consumer;
#[cfg(test)]
mod spinner_tests;

pub mod slider_consumer;
#[cfg(test)]
mod slider_tests;

pub mod backstage_consumer;
#[cfg(test)]
mod backstage_tests;
pub mod factory_backstage_consumer;
#[cfg(test)]
mod factory_backstage_tests;
pub mod factory_consumer;
#[cfg(test)]
mod factory_tests;

pub mod root_backstage_consumer;
#[cfg(test)]
mod root_backstage_tests;

#[cfg(test)]
mod model_graph_tests;

#[cfg(test)]
mod model_object_tests;

#[cfg(test)]
mod model_leaf_tests;

pub mod icon_sequence_consumer;

#[cfg(test)]
mod icon_sequence_tests;

#[cfg(test)]
mod collection_reference_tests;

#[cfg(test)]
mod menu_model_tests;

#[cfg(test)]
mod combo_model_tests;

pub mod segmented_consumer;
#[cfg(test)]
mod segmented_tests;

#[cfg(test)]
mod color_picker_tests;
pub mod palette_consumer;
#[cfg(test)]
mod palette_tests;

pub mod dialog_consumer;
#[cfg(test)]
mod dialog_host_tests;

#[cfg(test)]
mod named_objects_tests;
#[cfg(test)]
mod open_enum_tests;

pub mod gallery_consumer;
#[cfg(test)]
mod gallery_tests;
#[cfg(test)]
mod root_search_tests;
pub mod toolbar_consumer;
#[cfg(test)]
mod toolbar_control_tests;
#[cfg(test)]
mod toolbar_tests;

pub mod zoom_consumer;
#[cfg(test)]
mod zoom_tests;

pub mod status_consumer;
#[cfg(test)]
mod status_tests;

pub mod title_consumer;
#[cfg(test)]
mod title_tests;

pub mod application_menu_consumer;
#[cfg(test)]
mod application_menu_items_consumer_tests;

#[cfg(test)]
mod customization_tests;

pub mod context_menu_consumer;
#[cfg(test)]
mod context_menu_consumer_tests;

pub mod dialog_lifecycle_consumer;
#[cfg(test)]
mod dialog_lifecycle_consumer_tests;

#[cfg(test)]
mod flyout_picker_tests;

#[cfg(test)]
mod custom_factory_tests;

#[cfg(test)]
mod control_base_overflow_tests;

#[cfg(test)]
mod custom_resource_consumer_tests;

#[cfg(test)]
mod generic_theme_tests;

#[cfg(test)]
mod namespace_tests;

#[cfg(test)]
mod runtime_mvvm_tests;

#[cfg(test)]
mod runtime_item_tests;

pub mod image_icon_consumer;
#[cfg(test)]
mod image_icon_consumer_tests;
#[cfg(test)]
mod image_uri_tests;

#[cfg(test)]
#[path = "../fixtures/menu_image_uri_cases.rs"]
mod menu_image_uri_cases;

pub mod dialog_future_consumer;

pub mod qat_scope_consumer;

#[cfg(test)]
mod state_entry_tests;

#[cfg(test)]
mod icon_source_tests;

#[cfg(test)]
mod bitmap_icon_tests;
#[cfg(test)]
mod symbol_icon_tests;
