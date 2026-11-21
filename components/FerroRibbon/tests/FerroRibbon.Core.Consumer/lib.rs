//! Independent Core API consumer.
#[cfg(test)]
use ferroribbon_core::collections::RibbonFixedItems;
#[cfg(test)]
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{ObservableObject, ObservableObjectImpl};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::cell::RefCell;

pub struct ConsumerModel {
    base: ObservableObject,
    value: RefCell<i32>,
}
impl Default for ConsumerModel {
    fn default() -> Self {
        Self {
            base: ObservableObject::construct(),
            value: RefCell::new(0),
        }
    }
}
impl INotifyPropertyChanged for ConsumerModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for ConsumerModel {}
impl ConsumerModel {
    pub fn set_value(&self, value: i32) -> bool {
        ObservableObject::set_property::<i32>(self, &self.value, value, "Value")
    }
}
#[test]
fn consumes_original_algorithms_and_base_from_an_independent_workspace() {
    use ferroribbon_core::{RibbonGroupState, layout::*};
    let model = ConsumerModel::default();
    assert!(model.set_value(12));
    assert!(!model.set_value(12));
    let groups = [RibbonGroupLayoutInfo::new(vec![100., 80., 60., 40.])];
    let result = RibbonAdaptiveLayout::compute(&groups, 80.);
    assert_eq!(result.states, [RibbonGroupState::Medium]);
    assert!(result.fits);
}

#[test]
fn consumes_commands_through_existing_native_interface() {
    use ferroribbon_core::commands::{ICommand, RibbonAsyncCommand, RibbonRelayCommandOf};
    use futures_executor::LocalPool;
    use futures_util::task::LocalSpawnExt;
    use std::{cell::Cell, rc::Rc};
    let value = Rc::new(Cell::new(0));
    let output = value.clone();
    let command: Rc<dyn ICommand> = RibbonRelayCommandOf::<i32>::new(move |n| output.set(n), None);
    command.execute(Some(&42_i32));
    assert_eq!(value.get(), 42);
    let completed = Rc::new(Cell::new(false));
    let result = completed.clone();
    let mut executor = LocalPool::new();
    let spawner = executor.spawner();
    let command: Rc<dyn ICommand> = RibbonAsyncCommand::new(
        Rc::new(move |future| {
            spawner
                .spawn_local(async move { future.await.unwrap() })
                .unwrap();
        }),
        move || {
            result.set(true);
            std::future::ready(Ok(()))
        },
    );
    command.execute(None);
    executor.run_until_stalled();
    assert!(completed.get());
}

#[test]
fn consumes_icon_and_screentip_defaults_with_shared_identity() {
    use ferroribbon_core::model::{RibbonIcon, RibbonIconLayer, RibbonIcons, RibbonScreenTip};
    use std::rc::Rc;
    let icon = RibbonIcon::glyph(Some(RibbonIcons::Bold.into()));
    assert_eq!(icon.view_box_size, 24.);
    let tip = RibbonScreenTip::with_title(Some("Bold".into()));
    tip.set_image(Some(icon.clone()));
    assert!(Rc::ptr_eq(&tip.image().unwrap(), &icon));
    assert!(tip.description().is_none());
    assert!(tip.shortcut().is_none());
    let stroke = RibbonIcon::stroke(Some("M0,0 L10,10".into()));
    assert_eq!(
        RibbonIconLayer::parse(stroke.value.as_ref())
            .get(0)
            .unwrap()
            .stroke_thickness(),
        1.5
    );
}

#[test]
fn consumes_catalog_with_shared_descriptor_and_invocation_parameter() {
    use ferroribbon_core::{RibbonString, commands::*};
    use std::{any::Any, cell::Cell, rc::Rc};
    let catalog = RibbonCommandCatalog::new();
    let calls = Rc::new(Cell::new(0));
    let output = calls.clone();
    let parameter: Rc<dyn Any> = Rc::new(42_i32);
    let expected = parameter.clone();
    let descriptor = catalog.register_action(
        Some("Σave𐐀".into()),
        Some("Save".into()),
        Rc::new(move |value| {
            assert!(std::ptr::eq(value.unwrap(), expected.as_ref()));
            output.set(output.get() + 1);
        }),
        None,
        RibbonCommandMetadata::default(),
    );
    let lookup: RibbonString = "ςAVE𐐨".into();
    assert!(Rc::ptr_eq(
        &catalog.find(Some(&lookup)).unwrap(),
        &descriptor
    ));
    assert!(catalog.execute(Some(&lookup), Some(parameter)));
    assert_eq!(calls.get(), 1);
    descriptor.set_is_enabled(false);
    assert!(!catalog.execute(Some(&lookup), None));
    assert!(catalog.unregister(Some(&lookup)));
    assert_eq!(descriptor.property_changed().handler_count(), 0);
}

#[test]
fn consumes_gestures_and_modifier_flags_without_native_controls() {
    use ferroribbon_core::RibbonString;
    use ferroribbon_core::commands::{RibbonKeyGesture, RibbonModifierKeys};
    let gesture = RibbonKeyGesture::parse(Some(&"Ctrl+Shift+]".into())).unwrap();
    assert_eq!(
        gesture.modifiers,
        RibbonModifierKeys::CONTROL | RibbonModifierKeys::SHIFT
    );
    assert!(gesture.matches(
        Some(&"]".into()),
        RibbonModifierKeys::CONTROL | RibbonModifierKeys::SHIFT
    ));
    assert_eq!(gesture.to_ribbon_string(), "Ctrl+Shift+]".into());
    assert_eq!(
        RibbonKeyGesture::parse(None).unwrap_err().message,
        "'' is not a valid key gesture.".into()
    );
    let invalid = RibbonString::from_utf16([0xd800, 0x2b, 0x41]);
    let error = RibbonKeyGesture::parse(Some(&invalid)).unwrap_err();
    let diagnostic: &dyn std::error::Error = &error;
    assert_eq!(
        diagnostic.to_string(),
        "'\\u{d800}+A' is not a valid key gesture."
    );
    assert_eq!(error.message.as_utf16()[1], 0xd800);
    for alias in ["Esc", "Escape", "ESC"] {
        let gesture = RibbonKeyGesture::parse(Some(&alias.into())).unwrap();
        assert_eq!(gesture.key, Some("Escape".into()));
        assert!(gesture.matches(Some(&"Esc".into()), RibbonModifierKeys::NONE));
    }
}

#[cfg(test)]
mod derived_model_consumer {
    use super::*;
    use ferroribbon_core::{RibbonItemSize, model::*};
    use std::{cell::Cell, rc::Rc};
    struct ConsumerButton {
        base: RibbonButtonModel,
        notifications: Cell<usize>,
    }
    impl INotifyPropertyChanged for ConsumerButton {
        fn property_changed(&self) -> &Event<str> {
            self.base.property_changed()
        }
    }
    impl ObservableObjectImpl for ConsumerButton {
        fn on_property_changed(&self, name: &str) {
            self.notifications.set(self.notifications.get() + 1);
            self.property_changed().raise(name);
        }
    }
    impl RibbonNodeModelImpl for ConsumerButton {
        fn node_model(&self) -> &RibbonNodeModel {
            self.base.item_model().node_model()
        }
    }
    impl RibbonItemModelImpl for ConsumerButton {
        fn item_model(&self) -> &RibbonItemModel {
            self.base.item_model()
        }
    }
    impl RibbonButtonModelImpl for ConsumerButton {
        fn button_model(&self) -> &RibbonButtonModel {
            &self.base
        }
    }
    #[test]
    fn derives_model_and_preserves_dispatch_identity_and_shared_collections() {
        register_ribbon_button_model::<ConsumerButton>().unwrap();
        let model = Rc::new(ConsumerButton {
            base: RibbonButtonModel::construct(Some("custom".into()), None),
            notifications: Cell::new(0),
        });
        RibbonButtonModel::initialize(
            model.as_ref(),
            Some(RibbonIcon::glyph(Some("a".into()))),
            None,
            RibbonItemSize::Small,
        );
        assert_eq!(model.notifications.get(), 3);
        let base: Rc<dyn RibbonNodeModelImpl> = model.clone();
        let restored = try_cast_node::<dyn RibbonButtonModelImpl>(Some(base.clone()))
            .unwrap()
            .unwrap();
        assert!(std::ptr::addr_eq(Rc::as_ptr(&restored), Rc::as_ptr(&model)));
        base.set_label(Some("Name".into()));
        assert_eq!(model.notifications.get(), 4);
        let keyword_handle = base.keywords();
        keyword_handle.add(Some("search".into()));
        assert!(keyword_handle.ptr_eq(&model.keywords()));
        let menu = RibbonMenuItemModel::new(Some("menu".into()), None);
        menu.items().add(Some(base.clone()));
        assert!(Rc::ptr_eq(
            &downcast_node::<ConsumerButton>(menu.items().get(0).unwrap())
                .ok()
                .unwrap(),
            &model
        ));
        assert_eq!(
            base.to_ribbon_string().to_utf8().unwrap(),
            "ConsumerButton(custom, Name)"
        );
    }
}

#[test]
fn consumes_inputs_containers_and_segment_identity() {
    use ferroribbon_core::{RibbonString, model::*};
    use std::rc::Rc;
    let spinner = RibbonSpinnerModel::with_value(Some("value".into()), None, -3.5);
    assert_eq!(spinner.value(), -3.5);
    spinner.set_minimum(100.0);
    assert_eq!(spinner.value(), -3.5);
    let item: Rc<dyn RibbonItemModelImpl> = spinner.clone();
    let row = RibbonRowModel::with_items(None, [Some(item)]);
    row.items().get(0).unwrap().set_label(Some("Shared".into()));
    assert_eq!(spinner.label(), Some("Shared".into()));
    let combo = RibbonComboBoxModel::with_items(
        None,
        None,
        [Some(RibbonObject::value(RibbonString::from("a"))), None],
    );
    assert_eq!(combo.items().count(), 2);
    let segment: Rc<dyn RibbonSegmentModelImpl> = RibbonSegmentModel::new(Some("s".into()), None);
    let selected = RibbonSegmentedModel::new(None, None);
    selected.segments().add(Some(segment.clone()));
    selected.set_selected_segment(Some(segment.clone()));
    assert!(Rc::ptr_eq(&selected.selected_segment().unwrap(), &segment));
    assert_eq!(RibbonGridSize::new(2, 3).to_string(), "3x2");
}

#[test]
fn consumes_core_color_calculations_and_shared_palettes() {
    use ferroribbon_core::theming::*;
    let color = RibbonColor::parse(Some(&"#4472c4".into())).unwrap();
    assert_eq!(color.to_string(), "#4472C4");
    assert!(color.lighten(0.4).lightness() > color.lightness());
    let office = RibbonColorPalette::office_theme_colors();
    let grid = RibbonColorPalette::build_theme_grid(Some(&office));
    assert!(std::rc::Rc::ptr_eq(
        office[0].as_ref().unwrap(),
        grid[0][0].as_ref().unwrap()
    ));
    let word = RibbonThemePalette::word();
    assert!(std::rc::Rc::ptr_eq(
        &word,
        &RibbonThemePalette::presets()[0]
    ));
    assert_eq!(word.get_accent_subtle(true).a, 0x33);
}

#[test]
fn consumes_tab_group_gallery_and_backstage_shared_models() {
    use ferroribbon_core::{model::*, theming::RibbonColor};
    use std::rc::Rc;
    let group: Rc<dyn RibbonGroupModelImpl> = RibbonGroupModel::new(Some("group".into()), None);
    let tab = RibbonTabModel::new(Some("tab".into()), None);
    tab.groups().add(Some(group.clone()));
    tab.groups().get(0).unwrap().set_row_count(99);
    assert_eq!(group.row_count(), 3);
    let context = RibbonContextualGroupModel::new(Some("ctx".into()), None);
    assert!(!context.is_visible());
    context.set_is_visible(true);
    tab.set_contextual_group_id(context.id());
    assert!(tab.is_contextual());
    let item: Rc<dyn RibbonGalleryItemModelImpl> =
        RibbonGalleryItemModel::new(Some("choice".into()), None);
    let gallery = RibbonGalleryModel::new(None, None);
    gallery.items().add(Some(item.clone()));
    gallery.set_selected_item(Some(item.clone()));
    assert!(Rc::ptr_eq(&gallery.selected_item().unwrap(), &item));
    let picker = RibbonColorPickerModel::new(None, None);
    assert_eq!(
        picker.selected_color(),
        Some(RibbonColor::from_rgb(255, 0, 0))
    );
    let entry: Rc<dyn RibbonBackstageItemModelImpl> = RibbonBackstageItemModel::new(None, None);
    let backstage = RibbonBackstageModel::new();
    backstage.items().add(Some(entry.clone()));
    backstage.set_selected_item(Some(entry.clone()));
    assert!(Rc::ptr_eq(&backstage.selected_item().unwrap(), &entry));
}

pub struct ConsumerRibbonModel {
    base: ferroribbon_core::model::RibbonModel,
    pub changes: RefCell<Vec<String>>,
}
impl Default for ConsumerRibbonModel {
    fn default() -> Self {
        Self {
            base: ferroribbon_core::model::RibbonModel::construct(),
            changes: RefCell::default(),
        }
    }
}
impl INotifyPropertyChanged for ConsumerRibbonModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for ConsumerRibbonModel {
    fn on_property_changed(&self, name: &str) {
        self.changes.borrow_mut().push(name.into());
        self.property_changed().raise(name);
    }
}
impl ferroribbon_core::model::RibbonModelImpl for ConsumerRibbonModel {
    fn ribbon_model(&self) -> &ferroribbon_core::model::RibbonModel {
        &self.base
    }
}
#[test]
fn derived_root_queries_share_nodes_and_preserve_owner_dispatch() {
    use ferroribbon_core::model::*;
    use std::{any::Any, rc::Rc};
    register_ribbon_model::<ConsumerRibbonModel>().unwrap();
    let owner = Rc::new(ConsumerRibbonModel::default());
    let root: Rc<dyn RibbonModelImpl> = owner.clone();
    let tab = RibbonTabModel::new(Some("home".into()), None);
    let group = RibbonGroupModel::new(Some("commands".into()), None);
    let toggle = RibbonToggleButtonModel::new(Some("toggle".into()), None);
    toggle.set_command_id(Some("bold".into()));
    group.items().add(Some(toggle.clone()));
    tab.groups().add(Some(group));
    root.tabs().add(Some(tab.clone()));
    root.set_selected_tab(Some(tab));
    assert_eq!(*owner.changes.borrow(), ["SelectedTabId", "SelectedTab"]);
    let query = root.clone().find_items_by_command(Some("BOLD".into()));
    let inherited_static = ConsumerRibbonModel::flatten(Some(RibbonItemsSource::from_collection(
        root.quick_access_items(),
    )));
    assert_eq!(inherited_static.iter().count(), 0);
    let from_query = query.iter().next().unwrap().unwrap();
    assert!(std::ptr::addr_eq(
        Rc::as_ptr(&from_query),
        Rc::as_ptr(&toggle)
    ));
    root.set_command_checked(Some("bold".into()), true);
    assert!(toggle.is_checked());
    assert_eq!(query.iter().count(), 1);
    let cast = try_cast_model::<dyn RibbonModelImpl>(Some(owner.clone() as Rc<dyn Any>))
        .unwrap()
        .unwrap();
    assert!(Rc::ptr_eq(&cast, &root));
}

#[test]
fn model_views_use_standard_upcasts_downcasts_and_typed_projection_errors() {
    use ferroribbon_core::{ObservableObjectImpl, model::*};
    use ferroui_system::component_model::INotifyPropertyChanged;
    use std::{any::Any, rc::Rc};

    let concrete = RibbonButtonModel::new(Some("native".into()), None);
    let weak = Rc::downgrade(&concrete);
    let node: Rc<dyn RibbonNodeModelImpl> = concrete.clone();
    let observable: Rc<dyn ObservableObjectImpl> = node.clone();
    let notify: Rc<dyn INotifyPropertyChanged> = observable;
    let erased: Rc<dyn Any> = node.clone();
    let recovered = erased.downcast::<RibbonButtonModel>().unwrap();
    assert!(Rc::ptr_eq(&concrete, &recovered));
    assert!(std::ptr::addr_eq(
        Rc::as_ptr(&notify),
        Rc::as_ptr(&concrete)
    ));
    let projected = try_cast_node::<dyn RibbonButtonModelImpl>(Some(node.clone()))
        .unwrap()
        .unwrap();
    assert!(std::ptr::addr_eq(
        Rc::as_ptr(&projected),
        Rc::as_ptr(&concrete)
    ));
    assert!(
        try_cast_node::<dyn RibbonSliderModelImpl>(Some(node.clone()))
            .unwrap()
            .is_none()
    );
    let error = try_cast_model::<dyn RibbonNodeModelImpl>(Some(Rc::new(7_i32)))
        .err()
        .unwrap();
    let error: Box<dyn std::error::Error> = Box::new(error);
    assert!(
        error
            .to_string()
            .starts_with("unregistered ribbon model type:")
    );
    assert!(error.source().is_none());
    drop(projected);
    drop(recovered);
    drop(notify);
    drop(node);
    drop(concrete);
    assert!(weak.upgrade().is_none());
}

#[test]
fn consumes_merger_and_shared_plugin_commands_from_independent_package() {
    use ferroribbon_core::{commands::*, model::*};
    use std::{cell::Cell, rc::Rc};
    let host = Rc::new(ConsumerRibbonModel::default());
    let target: Rc<dyn RibbonModelImpl> = host.clone();
    let host_catalog = RibbonCommandCatalog::new();
    target.set_command_catalog(Some(host_catalog.clone()));
    let source = RibbonModel::new();
    let source_catalog = RibbonCommandCatalog::new();
    source.set_command_catalog(Some(source_catalog.clone()));
    let count = Rc::new(Cell::new(0));
    let output = count.clone();
    let command = RibbonRelayCommand::new(move || output.set(output.get() + 1));
    let descriptor =
        RibbonCommandDescriptor::new(Some("plugin.run".into()), Some("Run".into()), Some(command));
    source_catalog.register(Some(descriptor.clone()));
    let button = RibbonButtonModel::new(Some("plugin".into()), Some("Plugin".into()));
    button.set_command_id(Some("plugin.run".into()));
    source.tab_strip_items().add(Some(button.clone()));
    let merger = RibbonModelMerger::merge(Some(target.clone()), Some(source));
    let found = target.find_item(Some("plugin".into())).unwrap();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&found), Rc::as_ptr(&button)));
    assert!(Rc::ptr_eq(
        &host_catalog.find(Some(&"PLUGIN.RUN".into())).unwrap(),
        &descriptor
    ));
    assert!(host_catalog.execute(Some(&"plugin.run".into()), None));
    assert_eq!(count.get(), 1);
    merger.unmerge();
    assert!(target.find_item(Some("plugin".into())).is_none());
    assert!(!host_catalog.execute(Some(&"plugin.run".into()), None));
    assert!(source_catalog.execute(Some(&"plugin.run".into()), None));
    assert_eq!(count.get(), 2);
}

#[test]
fn keytip_scope_returns_original_model_and_releases_it_after_owner_removal() {
    use ferroribbon_core::{keytips::*, model::*};
    use std::rc::Rc;
    let model = RibbonButtonModel::new(Some("paint".into()), Some("Format Painter".into()));
    let weak = Rc::downgrade(&model);
    let requests = RibbonItems::from_values(vec![RibbonKeyTipRequest::new(
        model.label(),
        Some("FP".into()),
    )]);
    let tips = RibbonKeyTipAssigner::assign(Some(&requests), None);
    let scope = RibbonKeyTipScope::<Rc<RibbonButtonModel>>::new();
    scope.add(tips.get(0), Some(model.clone()));
    let navigator = RibbonKeyTipNavigator::new();
    navigator.push(Some(scope.clone()));
    let (result, target) = scope.process(b'f' as u16);
    assert_eq!(result, RibbonKeyTipMatch::Partial);
    assert!(target.is_none());
    let candidates = scope.clone().candidates();
    assert!(Rc::ptr_eq(
        &candidates.iter().next().unwrap().target.unwrap(),
        &model
    ));
    let (result, target) = navigator.current().unwrap().process(b'p' as u16);
    assert_eq!(result, RibbonKeyTipMatch::Complete);
    assert!(Rc::ptr_eq(&target.unwrap(), &model));
    assert!(scope.typed().is_empty());
    let entries = scope.entries();
    assert!(std::rc::Rc::ptr_eq(&entries, &scope.entries()));
    navigator.clear();
    drop(model);
    drop(scope);
    drop(entries);
    assert!(weak.upgrade().is_some()); // lazy Candidates owns the source scope
    let mut iterator = candidates.iter();
    drop(candidates);
    assert!(iterator.next().is_some());
    assert!(iterator.next().is_none()); // active traversal ends, captured source stays alive
    assert!(weak.upgrade().is_some());
    drop(iterator);
    assert!(weak.upgrade().is_none());
}

#[test]
fn search_results_keep_original_command_model_and_shared_index_ownership() {
    use ferroribbon_core::{commands::*, model::*, search::*};
    use std::{cell::Cell, rc::Rc};
    let calls = Rc::new(Cell::new(0));
    let output = calls.clone();
    let command = RibbonRelayCommand::new(move || output.set(output.get() + 1));
    let model = RibbonButtonModel::new(Some("run".into()), Some("Run".into()));
    model.set_command(Some(command));
    let weak = Rc::downgrade(&model);
    let engine = RibbonSearchEngine::new();
    let entry = RibbonSearchEntry::with_options(
        Some("run".into()),
        model.label(),
        RibbonSearchEntryOptions {
            target: Some(RibbonObject::reference(model.clone())),
            ..Default::default()
        },
    );
    engine.add(Some(entry.clone()));
    let results = engine.search_default(Some(&"run".into()));
    let result = results.get(0).unwrap();
    assert!(Rc::ptr_eq(&result.entry().unwrap(), &entry));
    let payload = result
        .entry()
        .unwrap()
        .target()
        .unwrap()
        .payload()
        .downcast::<RibbonButtonModel>()
        .unwrap();
    assert!(Rc::ptr_eq(&payload, &model));
    assert_eq!(calls.get(), 0); // Core search ranks; the control layer owns invocation.
    payload.command().unwrap().execute(None);
    assert_eq!(calls.get(), 1);
    model.set_label(Some("Changed".into()));
    assert_eq!(entry.label().unwrap().to_utf8().unwrap(), "Run"); // immutable index snapshot
    engine.mark_used(Some("run".into()));
    assert_eq!(engine.search_default(None).get(0).unwrap().score(), 100.);
    let list = engine.entries();
    let alias = list.clone();
    assert!(std::rc::Rc::ptr_eq(&alias, &engine.entries()));
    alias.clear();
    assert!(engine.search_default(None).is_empty());
    drop(payload);
    drop(model);
    drop(entry);
    drop(result);
    drop(results);
    assert!(weak.upgrade().is_none());
}

#[test]
fn state_customization_and_json_are_usable_without_controls_or_runtime_reflection() {
    use ferroribbon_core::{RibbonVisibilityMode, state::*};
    use std::rc::Rc;
    let state = RibbonState::new();
    state.set_visibility_mode(RibbonVisibilityMode::PanelTitles);
    state.set_schema_version(0);
    let shared = RibbonItems::from_values(vec![
        None,
        Some("".into()),
        Some("bold".into()),
        Some("bold".into()),
    ]);
    state.set_quick_access_item_ids(Some(shared.clone()));
    state.set_recent_search_ids(Some(shared.clone()));
    let custom = state.customization().unwrap();
    let group = RibbonCustomGroup::new();
    group.set_id(Some("custom.tools".into()));
    group.set_label(None);
    group.set_item_ids(Some(shared.clone()));
    custom.custom_groups().unwrap().add(Some(group.clone()));
    let labels = custom.labels().unwrap();
    labels.insert("home".into(), Some("Start & é".into()));
    state.normalize();
    assert_eq!(state.schema_version(), 0);
    assert!(std::rc::Rc::ptr_eq(
        &shared,
        &state.recent_search_ids().unwrap()
    ));
    assert_eq!(shared.len(), 2);
    assert!(Rc::ptr_eq(
        &custom.custom_groups().unwrap().get(0).unwrap(),
        &group
    ));
    let json = RibbonStateSerializer::serialize(Some(&state), false).unwrap();
    let restored = RibbonStateSerializer::deserialize(Some(&json))
        .unwrap()
        .unwrap();
    assert_eq!(restored.schema_version(), 1);
    assert_eq!(
        restored.visibility_mode(),
        RibbonVisibilityMode::PanelTitles
    );
    assert_eq!(
        restored
            .customization()
            .unwrap()
            .labels()
            .unwrap()
            .get(&"home".into())
            .unwrap()
            .unwrap()
            .to_utf8()
            .unwrap(),
        "Start & é"
    );
    assert_eq!(restored.quick_access_item_ids().unwrap().len(), 2);
    assert!(!Rc::ptr_eq(&state, &restored));
    assert!(
        RibbonStateSerializer::deserialize(Some(&"{\"schemaVersion\":99}".into()))
            .unwrap()
            .is_none()
    );
}

#[test]
fn localization_resources_refresh_and_subscription_cleanup_work_in_an_independent_library() {
    use ferroribbon_core::{localization::*, model::RibbonItemsSource};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let old = RibbonStrings::current();
    let strings =
        RibbonStrings::for_culture(Some(RibbonCulture::new("de-AT".into(), "de".into()))).unwrap();
    assert_eq!(strings.file().unwrap().to_utf8().unwrap(), "Datei");
    let count = Arc::new(AtomicUsize::new(0));
    let output = count.clone();
    let changed = RibbonStrings::current_changed();
    let token = changed.add(Arc::new(move |_| {
        output.fetch_add(1, Ordering::SeqCst);
    }));
    RibbonStrings::set_current(Some(strings.clone())).unwrap();
    let same = strings
        .override_string(Some("File".into()), Some("Start".into()))
        .unwrap();
    assert!(Arc::ptr_eq(&same, &strings));
    RibbonStrings::register(
        Some("de".into()),
        Some(RibbonItemsSource::from_values(vec![(
            Some("Options".into()),
            Some("New options".into()),
        )])),
    )
    .unwrap();
    let refreshed = RibbonStrings::current();
    assert!(!Arc::ptr_eq(&strings, &refreshed));
    assert_eq!(refreshed.file().unwrap().to_utf8().unwrap(), "Start");
    assert_eq!(
        refreshed.options().unwrap().to_utf8().unwrap(),
        "New options"
    );
    assert_eq!(count.load(Ordering::SeqCst), 3);
    assert_eq!(
        refreshed
            .format(
                Some(&"TablePickerFormat".into()),
                Some(RibbonFixedItems::new(vec![3.into(), 4.into()]))
            )
            .unwrap()
            .to_utf8()
            .unwrap(),
        "3x4-Tabelle"
    );
    changed.remove(token);
    RibbonStrings::set_current(Some(old)).unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 3);
}

#[test]
fn state_file_futures_use_native_paths_typed_errors_and_owned_snapshots() {
    use ferroribbon_core::state::{RibbonState, RibbonStateErrorKind, RibbonStateSerializer};
    use futures_executor::block_on;
    use std::path::Path;
    let root = std::env::temp_dir()
        .join(format!("state-core-consumer-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("state.json");
    let state = RibbonState::new();
    state.set_selected_tab_id(Some("before".into()));
    let save = RibbonStateSerializer::save_async_default(Some(&state), Some(&path));
    state.set_selected_tab_id(Some("after".into()));
    block_on(save).unwrap();
    let loaded = block_on(RibbonStateSerializer::load_async_default(Some(&path)))
        .unwrap()
        .unwrap();
    assert_eq!(
        loaded.selected_tab_id().unwrap().to_utf8().unwrap(),
        "before"
    );
    assert!(!root.join("state.json.tmp").exists());
    let error = block_on(RibbonStateSerializer::save_async_default(
        Some(&state),
        Some(Path::new("")),
    ))
    .unwrap_err();
    assert_eq!(error.kind, RibbonStateErrorKind::Argument);
    std::fs::remove_dir_all(root).unwrap();
}
