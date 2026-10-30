//! The native WinUI Repeater port through the run-time XAML loader. These tests cross the
//! markup/type-system boundary: layouts and factories remain native FerroUI objects, and
//! templates create ordinary controls.

use crate::{
    FlowLayout, FlowLayoutLineAlignment, ItemsRepeater, ItemsRepeaterScrollHost, LayoutPanel,
    LinedFlowLayout, LinedFlowLayoutItemsJustification, LinedFlowLayoutItemsStretch, RecyclePool,
    RecyclingElementFactory, StackLayout, UniformGridLayout, UniformGridLayoutItemsJustification,
    UniformGridLayoutItemsStretch,
};
use ferroui_base::layout::Orientation;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, FerroLocator, Rect, Ref, Size, Thickness};
use ferroui_controls::{Border, Control, ItemsSource, UserControl};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

/// The scope of a test that loads markup: the types of the crate registered, the run-time
/// loader installed, and a dispatcher and a service locator of the test's own.
struct XamlTestScope {
    locator: Option<std::rc::Rc<dyn IDisposable>>,
    _dispatcher: UnitTestDispatcherScope,
}

fn xaml_test_base() -> XamlTestScope {
    crate::register_types();
    let dispatcher = Dispatcher::unit_test_scope();
    Control::reset_loaded_queue_for_unit_tests();
    let locator = FerroLocator::enter_scope();
    FerroRuntimeXamlLoader::register();
    XamlTestScope { locator: Some(locator), _dispatcher: dispatcher }
}

impl Drop for XamlTestScope {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            Dispatcher::reset_for_unit_tests();
        }
        if let Some(locator) = self.locator.take() {
            locator.dispose();
        }
        Control::reset_loaded_queue_for_unit_tests();
    }
}

fn try_load(xaml: &str) -> Result<BoxedValue, impl std::fmt::Debug> {
    FerroRuntimeXamlLoader::parse(xaml, None)
}

#[track_caller]
fn load(xaml: &str) -> BoxedValue {
    try_load(xaml).unwrap_or_else(|error| panic!("the document loads: {error:?}"))
}

#[track_caller]
fn load_as<T: Clone + 'static>(xaml: &str) -> T {
    FerroRuntimeXamlLoader::parse_as::<T>(xaml, None)
        .unwrap_or_else(|error| panic!("the document loads: {error:?}"))
}

const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

fn realize(repeater: &Ref<ItemsRepeater>, count: i32) {
    repeater.set_items_source(Some(ItemsSource::from_values(0..count)));
    repeater.measure(Size::new(320.0, 240.0));
    repeater.arrange(Rect::new(0.0, 0.0, 320.0, 240.0));
}

#[test]
fn stack_layout_and_implicit_data_template_are_native_objects() {
    let _scope = xaml_test_base();
    let repeater = load_as::<Ref<ItemsRepeater>>(&format!(
        "<ItemsRepeater {NS} HorizontalCacheLength='1.5' VerticalCacheLength='3'>
           <ItemsRepeater.Layout><StackLayout Orientation='Horizontal' Spacing='7' IsVirtualizationEnabled='False'/></ItemsRepeater.Layout>
           <DataTemplate><Border Width='20' Height='16'/></DataTemplate>
         </ItemsRepeater>"
    ));
    let layout = repeater.layout().unwrap().cast::<StackLayout>().unwrap();
    assert_eq!(layout.orientation(), Orientation::Horizontal);
    assert_eq!(layout.spacing(), 7.0);
    assert!(!layout.is_virtualization_enabled());
    assert_eq!(repeater.horizontal_cache_length(), 1.5);
    assert_eq!(repeater.vertical_cache_length(), 3.0);
    realize(&repeater, 4);
    let first = repeater.try_get_element(0).unwrap();
    let second = repeater.try_get_element(1).unwrap();
    assert!(first.is::<Border>());
    assert_eq!(first.bounds().width, 20.0);
    assert_eq!(second.bounds().x, 27.0);
    assert_eq!(repeater.get_element_index(&second), 1);
}

#[test]
fn uniform_grid_layout_properties_and_enumerations_load() {
    let _scope = xaml_test_base();
    let layout = load_as::<Ref<UniformGridLayout>>(&format!(
        "<UniformGridLayout {NS} Orientation='Vertical' MinItemWidth='48' MinItemHeight='24'
          MinRowSpacing='3' MinColumnSpacing='5' MaximumRowsOrColumns='4'
          ItemsJustification='SpaceEvenly' ItemsStretch='Uniform'/>"
    ));
    assert_eq!(layout.orientation(), Orientation::Vertical);
    assert_eq!(layout.min_item_width(), 48.0);
    assert_eq!(layout.min_item_height(), 24.0);
    assert_eq!(layout.min_row_spacing(), 3.0);
    assert_eq!(layout.min_column_spacing(), 5.0);
    assert_eq!(layout.maximum_rows_or_columns(), 4);
    assert_eq!(
        layout.items_justification(),
        UniformGridLayoutItemsJustification::SpaceEvenly
    );
    assert_eq!(
        layout.items_stretch(),
        UniformGridLayoutItemsStretch::Uniform
    );
}

#[test]
fn flow_layout_properties_and_enumerations_load() {
    let _scope = xaml_test_base();
    let layout = load_as::<Ref<FlowLayout>>(&format!(
        "<FlowLayout {NS} Orientation='Horizontal' LineSpacing='6' MinItemSpacing='9' LineAlignment='SpaceBetween'/>"
    ));
    assert_eq!(layout.orientation(), Orientation::Horizontal);
    assert_eq!(layout.line_spacing(), 6.0);
    assert_eq!(layout.min_item_spacing(), 9.0);
    assert_eq!(
        layout.line_alignment(),
        FlowLayoutLineAlignment::SpaceBetween
    );
}

#[test]
fn lined_flow_layout_properties_and_enumerations_load() {
    let _scope = xaml_test_base();
    let layout = load_as::<Ref<LinedFlowLayout>>(&format!(
        "<LinedFlowLayout {NS} LineHeight='72' LineSpacing='8' MinItemSpacing='4'
          ItemsJustification='Center' ItemsStretch='Fill'/>"
    ));
    assert_eq!(layout.line_height(), 72.0);
    assert_eq!(layout.line_spacing(), 8.0);
    assert_eq!(layout.min_item_spacing(), 4.0);
    assert_eq!(
        layout.items_justification(),
        LinedFlowLayoutItemsJustification::Center
    );
    assert_eq!(layout.items_stretch(), LinedFlowLayoutItemsStretch::Fill);
}

#[test]
fn recycling_factory_populates_its_nullable_native_dictionary() {
    let _scope = xaml_test_base();
    let factory = load_as::<Ref<RecyclingElementFactory>>(&format!(
        "<RecyclingElementFactory {NS}>
           <DataTemplate x:Key='narrow'><Border Width='24' Height='16'/></DataTemplate>
           <DataTemplate x:Key='wide'><Border Width='48' Height='16'/></DataTemplate>
         </RecyclingElementFactory>"
    ));
    assert_eq!(factory.templates().count(), 2);
    let narrow = factory
        .templates()
        .try_get_value(&"narrow".to_owned())
        .unwrap();
    let wide = factory
        .templates()
        .try_get_value(&"wide".to_owned())
        .unwrap();
    assert_eq!(narrow.build(&None).unwrap().width(), 24.0);
    assert_eq!(wide.build(&None).unwrap().width(), 48.0);
}

#[test]
fn explicit_factory_templates_property_populates_existing_dictionary() {
    let _scope = xaml_test_base();
    let factory = load_as::<Ref<RecyclingElementFactory>>(&format!(
        "<RecyclingElementFactory {NS}>
           <RecyclingElementFactory.Templates>
             <DataTemplate x:Key='item'><Border Width='24' Height='16'/></DataTemplate>
           </RecyclingElementFactory.Templates>
         </RecyclingElementFactory>"
    ));
    assert_eq!(factory.templates().count(), 1);
}

#[test]
fn resource_factory_and_recycle_pool_are_shared_native_instances() {
    let _scope = xaml_test_base();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.Resources>
             <RecyclePool x:Key='pool'/>
             <RecyclingElementFactory x:Key='factory' RecyclePool='{{StaticResource pool}}'>
               <DataTemplate x:Key='item'><Border Width='36' Height='18'/></DataTemplate>
             </RecyclingElementFactory>
           </UserControl.Resources>
           <ItemsRepeater Name='items' ItemTemplate='{{StaticResource factory}}'>
             <ItemsRepeater.Layout><StackLayout Spacing='2'/></ItemsRepeater.Layout>
           </ItemsRepeater>
         </UserControl>"
    ));
    let repeater = root.find_control::<ItemsRepeater>("items").unwrap();
    realize(&repeater, 3);
    assert_eq!(repeater.try_get_element(1).unwrap().bounds().y, 20.0);
    let factory_value = repeater.item_template().unwrap();
    let factory = ferroui_base::metadata::from_markup_value::<Ref<RecyclingElementFactory>>(&Some(
        factory_value,
    ))
    .unwrap();
    assert!(factory.recycle_pool().is_some());
}

#[test]
fn a_shared_layout_keeps_host_specific_state() {
    let _scope = xaml_test_base();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.Resources><StackLayout x:Key='layout' Spacing='3'/></UserControl.Resources>
           <StackPanel>
             <ItemsRepeater Name='first' Layout='{{StaticResource layout}}'><DataTemplate><Border Width='20' Height='10'/></DataTemplate></ItemsRepeater>
             <ItemsRepeater Name='second' Layout='{{StaticResource layout}}'><DataTemplate><Border Width='20' Height='30'/></DataTemplate></ItemsRepeater>
           </StackPanel>
         </UserControl>"
    ));
    let first = root.find_control::<ItemsRepeater>("first").unwrap();
    let second = root.find_control::<ItemsRepeater>("second").unwrap();
    assert_eq!(first.layout(), second.layout());
    realize(&first, 3);
    realize(&second, 3);
    assert_eq!(first.try_get_element(1).unwrap().bounds().y, 13.0);
    assert_eq!(second.try_get_element(1).unwrap().bounds().y, 33.0);
}

#[test]
fn layout_panel_uses_children_as_its_content() {
    let _scope = xaml_test_base();
    let panel = load_as::<Ref<LayoutPanel>>(&format!(
        "<LayoutPanel {NS} Padding='4' BorderThickness='2' CornerRadius='3'>
           <LayoutPanel.Layout><StackLayout Orientation='Horizontal' Spacing='5'/></LayoutPanel.Layout>
           <Border Width='20' Height='10'/><Border Width='30' Height='10'/>
         </LayoutPanel>"
    ));
    assert_eq!(panel.children().count(), 2);
    assert_eq!(panel.padding(), Thickness::uniform(4.0));
    panel.measure(Size::new(100.0, 50.0));
    panel.arrange(Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(panel.children().get(0).bounds().x, 6.0);
    assert_eq!(panel.children().get(1).bounds().x, 31.0);
}

#[test]
fn scroll_host_accepts_native_scroll_viewer_content() {
    let _scope = xaml_test_base();
    let host = load_as::<Ref<ItemsRepeaterScrollHost>>(&format!(
        "<ItemsRepeaterScrollHost {NS} HorizontalAnchorRatio='0.25' VerticalAnchorRatio='0.75'>
           <ScrollViewer><ItemsRepeater/></ScrollViewer>
         </ItemsRepeaterScrollHost>"
    ));
    assert_eq!(host.horizontal_anchor_ratio(), 0.25);
    assert_eq!(host.vertical_anchor_ratio(), 0.75);
    let viewer = host.scroll_viewer().unwrap();
    assert!(Control::from_boxed(&viewer.content().unwrap())
        .unwrap()
        .is::<ItemsRepeater>());
}

#[test]
fn native_controls_in_a_markup_items_source_are_realized_without_wrappers() {
    let _scope = xaml_test_base();
    let repeater = load_as::<Ref<ItemsRepeater>>(&format!(
        "<ItemsRepeater {NS} xmlns:col='using:System.Collections'>
           <ItemsRepeater.ItemsSource><col:ArrayList>
             <Border Name='original' Width='32' Height='18'/>
           </col:ArrayList></ItemsRepeater.ItemsSource>
         </ItemsRepeater>"
    ));
    repeater.measure(Size::new(320.0, 240.0));
    repeater.arrange(Rect::new(0.0, 0.0, 320.0, 240.0));
    let original = repeater.items_source_view().unwrap().get_at(0).unwrap();
    let control = Control::from_boxed(&original).unwrap();
    assert_eq!(repeater.try_get_element(0), Some(control));
}

#[test]
fn recycle_pool_attached_reuse_key_round_trips_in_markup() {
    let _scope = xaml_test_base();
    let border =
        load_as::<Ref<Border>>(&format!("<Border {NS} RecyclePool.ReuseKey='wide-item'/>"));
    assert_eq!(RecyclePool::get_reuse_key(&border.upcast()), "wide-item");
}

#[test]
fn selection_service_loads_native_source_and_index_path() {
    use crate::selection::{IndexPath, SelectionModel};
    use std::rc::Rc;

    let _scope = xaml_test_base();
    let model = load_as::<Rc<SelectionModel>>(&format!(
        "<r:SelectionModel {NS} xmlns:r='using:FerroUI.Controls.Repeater' xmlns:col='using:System.Collections' SingleSelect='False'>
           <r:SelectionModel.Source><col:ArrayList><x:String>first</x:String><x:String>second</x:String></col:ArrayList></r:SelectionModel.Source>
         </r:SelectionModel>"
    ));
    // An `IndexPath` is a value: markup constructs it through a converter only, so the
    // selection is made in code (no `x:Arguments` for value types).
    model.select(1).unwrap();
    assert!(!model.single_select());
    assert_eq!(model.source().unwrap().count(), 2);
    assert_eq!(model.selected_index(), Some(IndexPath::create_from(1)));
    assert_eq!(
        model
            .selected_item()
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("second")
    );
}

#[test]
fn compiled_selection_view_bindings_track_native_selection_changes() {
    use crate::selection::SelectionModel;
    use ferroui_controls::{StackPanel, TextBlock};
    use std::rc::Rc;

    let _scope = xaml_test_base();
    let panel = load_as::<Ref<StackPanel>>(&format!(
        "<StackPanel {NS} xmlns:r='using:FerroUI.Controls.Repeater' x:DataType='r:SelectionModel'>
           <TextBlock Name='selectedItemsCount' Text='{{CompiledBinding SelectedItems.Count}}'/>
           <TextBlock Name='selectedIndicesCount' Text='{{CompiledBinding SelectedIndices.Count}}'/>
         </StackPanel>"
    ));
    let model = SelectionModel::new();
    model.set_single_select(false);
    model.set_source(Some(ItemsSource::from_values([
        "first".to_owned(),
        "second".to_owned(),
    ])));
    model.select(1).unwrap();
    panel.set_data_context(Some(Rc::new(model.clone())));
    let assert_counts = |expected: &str| {
        for name in ["selectedItemsCount", "selectedIndicesCount"] {
            assert_eq!(
                panel
                    .find_control::<TextBlock>(name)
                    .unwrap()
                    .text()
                    .as_deref(),
                Some(expected),
                "{name}"
            );
        }
    };
    assert_counts("1");
    model.select(0).unwrap();
    assert_counts("2");
    model.deselect(1).unwrap();
    assert_counts("1");
}

#[test]
fn repeater_conversions_are_registered_on_subsequent_ui_threads() {
    // The process-wide metadata registration can have happened on a thread
    // that has already exited. Each new UI thread still needs its conversions.
    crate::register_types();
    std::thread::spawn(|| {
        use crate::selection::SelectionModel;
        use std::rc::Rc;

        let _scope = xaml_test_base();
        let factory = load_as::<Ref<RecyclingElementFactory>>(&format!(
            "<RecyclingElementFactory {NS}><DataTemplate x:Key='item'><Border Width='19'/></DataTemplate></RecyclingElementFactory>"
        ));
        assert_eq!(factory.templates().count(), 1);
        assert_eq!(factory.templates().try_get_value(&"item".to_owned()).unwrap().build(&None).unwrap().width(), 19.0);

        let model = SelectionModel::new();
        model.set_source(Some(ItemsSource::from_values(["first".to_owned(), "second".to_owned()])));
        model.select(1).unwrap();
        let repeater = load_as::<Ref<ItemsRepeater>>(&format!(
            "<ItemsRepeater {NS} ItemsSource='{{Binding SelectedItems}}'><DataTemplate><Border Width='20' Height='10'/></DataTemplate></ItemsRepeater>"
        ));
        repeater.set_data_context(Some(Rc::new(model.clone())));
        repeater.measure(Size::new(100.0, 100.0));
        let view = repeater.items_source_view().unwrap();
        assert_eq!(view.count(), 1);
        assert_eq!(view.get_at(0).unwrap().downcast_ref::<String>().map(String::as_str), Some("second"));
        let selected = load_as::<Ref<ferroui_controls::TextBlock>>(&format!(
            "<TextBlock {NS} Text='{{Binding SelectedIndex}}'/>"
        ));
        selected.set_data_context(Some(Rc::new(model)));
        assert_eq!(selected.text().as_deref(), Some("R.1"));
    }).join().unwrap();
}

#[test]
fn items_source_view_uses_its_native_class_name_and_preserves_binding_identity() {
    use ferroui_base::data::core::ValueTypes;
    use ferroui_controls::ItemsSourceView;
    use std::rc::Rc;

    let _scope = xaml_test_base();
    let view = ItemsSourceView::new(ItemsSource::from_values(["first".to_owned()]));
    let boxed: ferroui_base::BoxedValue = Rc::new(view.clone());
    assert_eq!(
        ValueTypes::to_display_string(Some(&boxed)),
        "FerroUI.Controls.ItemsSourceView"
    );
    for source in [
        boxed,
        Rc::new(Some(view.clone())) as ferroui_base::BoxedValue,
    ] {
        let repeater = load_as::<Ref<ItemsRepeater>>(&format!(
            "<ItemsRepeater {NS} ItemsSource='{{Binding}}'><DataTemplate><Border Width='20' Height='10'/></DataTemplate></ItemsRepeater>"
        ));
        repeater.set_data_context(Some(source));
        repeater.measure(Size::new(100.0, 100.0));
        assert!(Rc::ptr_eq(&repeater.items_source_view().unwrap(), &view));
    }
    assert_eq!(view.count(), 1);
}

#[test]
fn repeater_runtime_types_have_native_markup_names() {
    use ferroui_base::data::core::ValueTypes;

    let _scope = xaml_test_base();
    for name in [
        "ItemsRepeater",
        "ItemsRepeaterScrollHost",
        "LayoutPanel",
        "ElementFactory",
        "RecyclingElementFactory",
        "RecyclePool",
        "VirtualizingLayout",
        "NonVirtualizingLayout",
        "VirtualizingLayoutContext",
        "NonVirtualizingLayoutContext",
        "StackLayout",
        "StackLayoutState",
        "UniformGridLayout",
        "UniformGridLayoutState",
        "FlowLayout",
        "FlowLayoutState",
        "LinedFlowLayout",
        "ItemCollectionTransitionProvider",
        "LinedFlowLayoutItemCollectionTransitionProvider",
    ] {
        let value = load(&format!("<{name} {NS}/>"));
        let object = ValueTypes::as_object(&*value).unwrap();
        assert_eq!(
            object.get_type().full_name(),
            format!("FerroUI.Controls.{name}")
        );
        assert_eq!(
            ValueTypes::to_display_string(Some(&value)),
            format!("FerroUI.Controls.{name}")
        );
    }
}

#[test]
fn repeater_service_metadata_has_native_names() {
    use ferroui_base::metadata::MarkupType;
    use crate::selection::*;
    use crate::*;
    use std::any::TypeId;
    use std::rc::Rc;

    let _scope = xaml_test_base();
    macro_rules! name {
        ($handle:ty, $expected:expr) => {
            assert_eq!(
                MarkupType::find_by_handle(TypeId::of::<$handle>())
                    .unwrap()
                    .full_name(),
                $expected
            );
        };
    }
    name!(IndexPath, "FerroUI.Controls.Repeater.IndexPath");
    name!(
        Rc<SelectionModel>,
        "FerroUI.Controls.Repeater.SelectionModel"
    );
    name!(
        SelectionModelSelectionChangedEventArgs,
        "FerroUI.Controls.Repeater.SelectionModelSelectionChangedEventArgs"
    );
    name!(
        Rc<SelectionModelChildrenRequestedEventArgs>,
        "FerroUI.Controls.Repeater.SelectionModelChildrenRequestedEventArgs"
    );
    name!(
        ElementFactoryGetArgs,
        "FerroUI.Controls.Repeater.ElementFactoryGetArgs"
    );
    name!(
        ElementFactoryRecycleArgs,
        "FerroUI.Controls.Repeater.ElementFactoryRecycleArgs"
    );
    name!(
        SelectTemplateEventArgs,
        "FerroUI.Controls.Repeater.SelectTemplateEventArgs"
    );
    name!(
        ItemsRepeaterElementPreparedEventArgs,
        "FerroUI.Controls.ItemsRepeaterElementPreparedEventArgs"
    );
    name!(
        ItemsRepeaterElementClearingEventArgs,
        "FerroUI.Controls.ItemsRepeaterElementClearingEventArgs"
    );
    name!(
        ItemsRepeaterElementIndexChangedEventArgs,
        "FerroUI.Controls.ItemsRepeaterElementIndexChangedEventArgs"
    );
    name!(
        LinedFlowLayoutItemsInfoRequestedEventArgs,
        "FerroUI.Controls.LinedFlowLayoutItemsInfoRequestedEventArgs"
    );
}

#[test]
fn repeater_static_helpers_resolve_by_their_declared_markup_namespace() {
    use ferroui_base::metadata::MarkupType;
    use crate::{LayoutsTestHooks, RepeaterBindingHelper, RepeaterTestHooks};
    use ferroui_markup_xaml_loader::FerroXamlIlRuntimeCompiler;
    use xamlx::type_system::IXamlTypeSystem;

    let _scope = xaml_test_base();
    let system = FerroXamlIlRuntimeCompiler::type_system();
    for type_info in [LayoutsTestHooks::TYPE, RepeaterBindingHelper::TYPE, RepeaterTestHooks::TYPE] {
        let metadata = MarkupType::find_by_type_info(type_info).expect("linked static metadata");
        let projected = system.type_of_class(type_info);
        assert_eq!(projected.full_name(), metadata.full_name());
        let by_name = system.find_type(&metadata.full_name()).expect("the declared markup name");
        assert!(by_name.equals(&*projected));
        assert!(system.find_type(&format!("FerroUI.Controls.{}", type_info.name())).is_none());
    }
}

#[test]
fn repeater_markup_activation_requires_native_constructor_arguments() {
    use ferroui_base::data::core::ValueTypes;

    let _scope = xaml_test_base();
    assert!(try_load(&format!("<ItemsSourceView {NS}/>")).is_err());
    assert!(try_load(&format!(
        "<p:RepeaterAutomationPeer {NS} xmlns:p='using:FerroUI.Automation.Peers'/>"
    ))
    .is_err());
    assert_eq!(
        <crate::RepeaterAutomationPeer as ferroui_base::StaticType>::TYPE.full_name(),
        "FerroUI.Automation.Peers.RepeaterAutomationPeer"
    );
    let model = load(&format!(
        "<r:SelectionModel {NS} xmlns:r='using:FerroUI.Controls.Repeater'/>"
    ));
    assert_eq!(
        ValueTypes::to_display_string(Some(&model)),
        "FerroUI.Controls.Repeater.SelectionModel"
    );
}
