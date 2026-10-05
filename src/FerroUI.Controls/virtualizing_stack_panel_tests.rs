//! Tests of the virtualizing stack panel.
//!
//! The model items that count the measure and arrange calls of their
//! containers are merged into the plain model items.

use crate::generators::RecycleKey;
use crate::presenters::{ContentPresenter, ItemsPresenter, ScrollContentPresenter};
use crate::primitives::{ScrollBar, ScrollBarVisibility, TemplatedControl, TemplatedControlImpl};
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
    IDataTemplate, ITemplateOf,
};
use crate::test_support::{string_of, test_scope, TestRoot, TestScope};
use crate::{
    Border, Button, Canvas, CanvasImpl, ContentControl, Control, ControlImpl, Decorator, IItemsList, ItemsChangedEventArgs,
    ItemsChangedHandler, ItemsControl, ItemsControlImpl, ItemsSource, ListBox, ListBoxItem, Panel, PanelImpl,
    ScrollViewer, TextBlock, VirtualizingPanelImpl, VirtualizingStackPanel,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{ReflectionBinding, TemplateBinding};
use ferroui_base::input::{IKeyboardDevice, InputElement, InputElementImpl, KeyboardDevice, KeyboardNavigation};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{
    HorizontalAlignment, Layoutable, LayoutableImpl, LayoutableImplExt, Orientation, VerticalAlignment,
};
use ferroui_base::media::{Brushes, Colors, IBrush, SolidColorBrush};
use ferroui_base::styling::{ControlTheme, Selectors, Setter, Style};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_model, instantiate, AnyValue, BoxedValue, FerroLocator, FerroObjectImpl,
    ObjectType, Rect, Ref, Size, StyledElementImpl, Thickness, TypeInfo, Vector, Visual,
    VisualImpl,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

struct ItemWithHeight {
    caption: String,
    height: Cell<f64>,
    measured: Cell<i32>,
    arranged: Cell<i32>,
    property_changed: Event<str>,
}

impl ItemWithHeight {
    fn new(index: i32) -> Rc<Self> {
        Self::with_height(index, 10.0)
    }

    fn with_height(index: i32, height: f64) -> Rc<Self> {
        Model::new_model(Self {
            caption: format!("Item {index}"),
            height: Cell::new(height),
            measured: Cell::new(0),
            arranged: Cell::new(0),
            property_changed: Event::new(),
        })
    }

    fn height(&self) -> f64 {
        self.height.get()
    }

    fn set_height(&self, value: f64) {
        if self.height.get() != value {
            self.height.set(value);
            self.property_changed.raise("Height");
        }
    }
}

impl INotifyPropertyChanged for ItemWithHeight {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(ItemWithHeight, |b| b
    .notify_property_changed()
    .property::<Value<f64>>("Height", |vm| vm.height(), |vm, v| vm.set_height(v)));

struct ItemWithWidth {
    caption: String,
    width: Cell<f64>,
    measured: Cell<i32>,
    arranged: Cell<i32>,
    property_changed: Event<str>,
}

impl ItemWithWidth {
    fn new(index: i32) -> Rc<Self> {
        Self::with_width(index, 10.0)
    }

    fn with_width(index: i32, width: f64) -> Rc<Self> {
        Model::new_model(Self {
            caption: format!("Item {index}"),
            width: Cell::new(width),
            measured: Cell::new(0),
            arranged: Cell::new(0),
            property_changed: Event::new(),
        })
    }

    fn width(&self) -> f64 {
        self.width.get()
    }

    fn set_width(&self, value: f64) {
        if self.width.get() != value {
            self.width.set(value);
            self.property_changed.raise("Width");
        }
    }
}

impl INotifyPropertyChanged for ItemWithWidth {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(ItemWithWidth, |b| b
    .notify_property_changed()
    .property::<Value<f64>>("Width", |vm| vm.width(), |vm, v| vm.set_width(v)));

struct ItemWithIsVisible {
    #[allow(dead_code)]
    caption: String,
    is_visible: Cell<bool>,
    property_changed: Event<str>,
}

impl ItemWithIsVisible {
    fn new(index: i32) -> Rc<Self> {
        Model::new_model(Self {
            caption: format!("Item {index}"),
            is_visible: Cell::new(true),
            property_changed: Event::new(),
        })
    }

    fn is_visible(&self) -> bool {
        self.is_visible.get()
    }

    fn set_is_visible(&self, value: bool) {
        if self.is_visible.get() != value {
            self.is_visible.set(value);
            self.property_changed.raise("IsVisible");
        }
    }
}

impl INotifyPropertyChanged for ItemWithIsVisible {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(ItemWithIsVisible, |b| b
    .notify_property_changed()
    .property::<Value<bool>>("IsVisible", |vm| vm.is_visible(), |vm, v| vm.set_is_visible(v)));

/// The counters of a model item that counts the measure and arrange calls of
/// its container content.
fn with_counters(data: &Option<BoxedValue>, f: impl Fn(&Cell<i32>, &Cell<i32>)) {
    let Some(data) = data else { return };
    let value: &dyn AnyValue = &**data;
    if let Some(item) = value.downcast_ref::<ItemWithHeight>() {
        f(&item.measured, &item.arranged);
    } else if let Some(item) = value.downcast_ref::<ItemWithWidth>() {
        f(&item.measured, &item.arranged);
    }
}

/// A collection that only ever notifies of resets.
struct ResettingCollection {
    items: RefCell<Vec<String>>,
    collection_changed: HandlerList<ItemsChangedHandler>,
}

impl ResettingCollection {
    fn new(items: impl IntoIterator<Item = String>) -> Rc<Self> {
        Rc::new(Self { items: RefCell::new(items.into_iter().collect()), collection_changed: HandlerList::new() })
    }

    fn reset(&self, items: impl IntoIterator<Item = String>) {
        *self.items.borrow_mut() = items.into_iter().collect();
        let e = ItemsChangedEventArgs::RESET;
        for (_, handler) in self.collection_changed.snapshot().iter() {
            handler(&e);
        }
    }
}

impl IItemsList for ResettingCollection {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        Some(Rc::new(self.items.borrow()[index].clone()))
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.collection_changed.add(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.collection_changed.remove(token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Test classes
// ---------------------------------------------------------------------------

/// An items control whose containers cannot be recycled.
#[repr(C)]
struct NonRecyclingItemsControl {
    base: ItemsControl,
}

ferro_class!(NonRecyclingItemsControl: ItemsControl);
ferro_impl_classes!(
    NonRecyclingItemsControl: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl StyledElementImpl for NonRecyclingItemsControl {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        ItemsControl::TYPE
    }
}

impl ItemsControlImpl for NonRecyclingItemsControl {
    fn needs_container_override(_this: &Self, _item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        (true, None)
    }
}

impl NonRecyclingItemsControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: ItemsControl::construct() })
    }
}

/// A virtualizing stack panel that counts its measure and arrange calls.
#[repr(C)]
struct CountingPanel {
    base: VirtualizingStackPanel,
    measured: Cell<i32>,
    arranged: Cell<i32>,
}

ferro_class!(CountingPanel: VirtualizingStackPanel);
ferro_impl_classes!(
    CountingPanel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    VirtualizingPanelImpl
);

impl LayoutableImpl for CountingPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measured.set(this.measured.get() + 1);
        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arranged.set(this.arranged.get() + 1);
        Self::parent_arrange_override(this, final_size)
    }
}

impl CountingPanel {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: VirtualizingStackPanel::construct(),
            measured: Cell::new(0),
            arranged: Cell::new(0),
        })
    }

    fn reset_measure_arrange_counters(&self) {
        // Reset counters.
        self.measured.set(0);
        self.arranged.set(0);
        for item in self.items().iter() {
            with_counters(&item, |measured, arranged| {
                measured.set(0);
                arranged.set(0);
            });
        }
    }
}

/// A canvas that counts its measure and arrange calls on its model item.
#[repr(C)]
struct CountingCanvas {
    base: Canvas,
}

ferro_class!(CountingCanvas: Canvas);
ferro_impl_classes!(
    CountingCanvas: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    CanvasImpl
);

impl LayoutableImpl for CountingCanvas {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        with_counters(&this.data_context(), |measured, _| measured.set(measured.get() + 1));
        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        with_counters(&this.data_context(), |_, arranged| arranged.set(arranged.get() + 1));
        Self::parent_arrange_override(this, final_size)
    }
}

impl CountingCanvas {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Canvas::construct() })
    }
}

/// An items control that wraps every item in a [`ContainerControl`].
#[repr(C)]
struct ItemsControlWithContainer {
    base: ItemsControl,
}

ferro_class!(ItemsControlWithContainer: ItemsControl);
ferro_impl_classes!(
    ItemsControlWithContainer: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl StyledElementImpl for ItemsControlWithContainer {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        ItemsControl::TYPE
    }
}

impl ItemsControlImpl for ItemsControlWithContainer {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        ContainerControl::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<ContainerControl>(item)
    }
}

#[repr(C)]
struct ContainerControl {
    base: ContentControl,
}

ferro_class!(ContainerControl: ContentControl);
ferro_impl_classes!(
    ContainerControl: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    crate::ContentControlImpl
);

impl StyledElementImpl for ContainerControl {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        ContentControl::TYPE
    }
}

impl ContainerControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: ContentControl::construct() })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct Options {
    items: Option<ItemsSource>,
    /// `Some(None)` is an explicitly absent item template.
    item_template: Option<Option<Rc<dyn IDataTemplate>>>,
    styles: Vec<Ref<Style>>,
    orientation: Orientation,
    buffer_factor: f64,
    items_control: Option<Ref<ItemsControl>>,
    panel: Option<Ref<VirtualizingStackPanel>>,
}

struct Target {
    target: Ref<VirtualizingStackPanel>,
    scroll: Ref<ScrollViewer>,
    items_control: Ref<ItemsControl>,
    /// The default items, when the test did not supply its own.
    default_items: Option<Rc<FerroList<String>>>,
    root: Option<Ref<TestRoot>>,
}

impl Target {
    /// The default items.
    fn items(&self) -> Rc<FerroList<String>> {
        self.default_items.clone().expect("the test supplied its own items")
    }

    fn counting_panel(&self) -> Ref<CountingPanel> {
        self.target.cast::<CountingPanel>().expect("the panel is not a counting panel")
    }
}

fn app() -> TestScope {
    let scope = test_scope();
    FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(KeyboardDevice::new());
    scope
}

fn create_target(configure: impl FnOnce(&mut Options)) -> Target {
    let mut styles = Vec::new();
    let mut target = create_unrooted_target(|o| {
        configure(o);
        styles = std::mem::take(&mut o.styles);
    });

    let root = create_root(Some(target.items_control.clone().upcast()), None, &styles);

    root.execute_initial_layout_pass();

    target.root = Some(root);
    target
}

fn create_unrooted_target(configure: impl FnOnce(&mut Options)) -> Target {
    let mut options = Options {
        items: None,
        item_template: None,
        styles: Vec::new(),
        orientation: Orientation::Vertical,
        buffer_factor: 0.0,
        items_control: None,
        panel: None,
    };
    configure(&mut options);

    let target = options.panel.unwrap_or_else(VirtualizingStackPanel::new);
    target.set_orientation(options.orientation);
    target.set_cache_length(options.buffer_factor);

    let mut default_items = None;
    let items = options.items.unwrap_or_else(|| {
        let items = Rc::new(FerroList::from_items((0..100).map(|x| format!("Item {x}"))));
        default_items = Some(items.clone());
        items.into()
    });

    let presenter = ItemsPresenter::new();
    let property = ItemsControl::items_panel_property().as_property();
    presenter.bind_binding(property, &TemplateBinding::new(property));

    let scroll = ScrollViewer::new();
    scroll.set_name(Some("PART_ScrollViewer".to_string()));
    scroll.set_content(Some(Control::boxed(presenter)));

    if options.orientation == Orientation::Horizontal {
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    }

    scroll.set_template(Some(scroll_viewer_template()));

    let items_control = options.items_control.unwrap_or_else(ItemsControl::new);
    items_control.set_items_source(Some(items));
    let template_scroll = scroll.clone();
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<ItemsControl>(move |_, scope| {
        template_scroll.clone().register_in_name_scope(&**scope).upcast()
    });
    items_control.set_template(Some(template));
    let template_target = target.clone();
    let items_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
        FuncTemplate::new(move || Some(template_target.clone().upcast::<Panel>()));
    items_control.set_items_panel(items_panel);
    items_control.set_item_template(options.item_template.unwrap_or_else(|| Some(default_item_template())));

    Target { target, scroll, items_control, default_items, root: None }
}

fn create_root(child: Option<Ref<Control>>, client_size: Option<Size>, styles: &[Ref<Style>]) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_child(child);
    root.set_client_size(client_size.unwrap_or(Size::new(100.0, 100.0)));

    for style in styles {
        root.styles().add(style.clone());
    }

    root
}

fn create_is_visible_binding_style() -> Ref<Style> {
    Style::with_setters(
        Selectors::of_type::<ContentPresenter>(),
        [Setter::new_binding_base(
            Visual::is_visible_property().as_property(),
            ReflectionBinding::new("IsVisible"),
        )],
    )
}

fn default_item_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let canvas = Canvas::new();
            canvas.set_width(100.0);
            canvas.set_height(10.0);
            Some(canvas.upcast())
        },
        false,
    )
}

fn canvas_with_height_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::for_type::<ItemWithHeight>(
        |_, _| {
            let canvas = CountingCanvas::new();
            canvas.set_width(100.0);
            canvas.bind_binding(Layoutable::height_property().as_property(), &ReflectionBinding::new("Height"));
            Some(canvas.upcast())
        },
        false,
    )
}

fn canvas_with_width_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::for_type::<ItemWithWidth>(
        |_, _| {
            let canvas = CountingCanvas::new();
            canvas.set_height(100.0);
            canvas.bind_binding(Layoutable::width_property().as_property(), &ReflectionBinding::new("Width"));
            Some(canvas.upcast())
        },
        false,
    )
}

fn list_box_item_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ListBoxItem>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.set_width(100.0);
        presenter.set_height(10.0);
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ScrollContentPresenter".to_string()));
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

fn scroll_viewer_template_with_scroll_bars() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ScrollContentPresenter".to_string()));
        let presenter = presenter.register_in_name_scope(&**scope);

        let horizontal_scroll_bar = ScrollBar::new();
        horizontal_scroll_bar.set_name(Some("PART_HorizontalScrollBar".to_string()));
        horizontal_scroll_bar.set_orientation(Orientation::Horizontal);
        horizontal_scroll_bar.set_vertical_alignment(VerticalAlignment::Bottom);
        let horizontal_scroll_bar = horizontal_scroll_bar.register_in_name_scope(&**scope);

        let vertical_scroll_bar = ScrollBar::new();
        vertical_scroll_bar.set_name(Some("PART_VerticalScrollBar".to_string()));
        vertical_scroll_bar.set_orientation(Orientation::Vertical);
        vertical_scroll_bar.set_horizontal_alignment(HorizontalAlignment::Right);
        let vertical_scroll_bar = vertical_scroll_bar.register_in_name_scope(&**scope);

        let panel = Panel::new();
        panel.children().add(presenter);
        panel.children().add(horizontal_scroll_bar);
        panel.children().add(vertical_scroll_bar);
        panel.upcast()
    })
}

fn layout(target: &Layoutable) {
    if let Some(layout_manager) = target.get_layout_manager() {
        layout_manager.execute_layout_pass();
    }
}

/// The model items as an items source.
fn models<T: PartialEq + 'static>(items: &[Rc<T>]) -> ItemsSource {
    ItemsSource::from_items(items.iter().map(|item| {
        let item: BoxedValue = item.clone();
        Some(item)
    }))
}

fn control_items(items: &[Ref<Control>]) -> ItemsSource {
    ItemsSource::from_items(items.iter().map(|item| Some(Control::boxed(item.clone()))))
}

fn button(width: f64, height: f64) -> Ref<Control> {
    let button = Button::new();
    button.set_width(width);
    button.set_height(height);
    button.upcast()
}

fn range(first: i32, count: i32) -> Vec<i32> {
    (first..first + count).collect()
}

fn parse_indexes(raw: &str, separator: &str) -> Vec<i32> {
    raw.split(separator).map(|x| x.parse().unwrap()).collect()
}

fn brush(brush: Rc<dyn IBrush>) -> Option<Rc<dyn IBrush>> {
    Some(brush)
}

fn first_realized(target: &VirtualizingStackPanel) -> Ref<Control> {
    target.get_realized_elements().first().cloned().flatten().expect("no realized element")
}

fn last_realized(target: &VirtualizingStackPanel) -> Ref<Control> {
    target.get_realized_elements().last().cloned().flatten().expect("no realized element")
}

fn realized_containers(target: &VirtualizingStackPanel) -> Vec<Ref<Control>> {
    target.get_realized_containers().expect("no realized containers")
}

fn get_realized_indexes(target: &VirtualizingStackPanel, items_control: &ItemsControl) -> Vec<i32> {
    target
        .get_realized_elements()
        .iter()
        .map(|x| match x {
            None => -1,
            Some(x) => items_control.index_from_container(x),
        })
        .collect()
}

fn assert_realized_items(
    target: &Ref<VirtualizingStackPanel>,
    items_control: &Ref<ItemsControl>,
    first_index: i32,
    count: i32,
) {
    for x in realized_containers(target) {
        assert_eq!(x.visual_parent(), Some(target.clone().upcast::<Visual>()));
        assert_eq!(x.parent(), Some(items_control.clone().upcast()));
    }

    let mut child_indexes: Vec<i32> = realized_containers(target)
        .iter()
        .map(|x| items_control.index_from_container(x))
        .filter(|x| *x >= 0)
        .collect();
    child_indexes.sort();
    assert_eq!(range(first_index, count), child_indexes);

    let visible_children = target.children().to_vec().iter().filter(|x| x.is_visible()).count();
    assert_eq!(count as usize, visible_children);
}

fn assert_realized_control_items<TContainer: ObjectType>(
    target: &Ref<VirtualizingStackPanel>,
    items_control: &Ref<ItemsControl>,
    first_index: i32,
    count: i32,
) {
    for x in realized_containers(target) {
        assert_eq!(x.get_type(), TContainer::TYPE);
        assert_eq!(x.visual_parent(), Some(target.clone().upcast::<Visual>()));
        assert_eq!(x.parent(), Some(items_control.clone().upcast()));
    }

    let mut child_indexes: Vec<i32> = realized_containers(target)
        .iter()
        .map(|x| items_control.index_from_container(x))
        .filter(|x| *x >= 0)
        .collect();
    child_indexes.sort();
    assert_eq!(range(first_index, count), child_indexes);
}

fn content_presenter(container: Option<Ref<Control>>) -> Ref<ContentPresenter> {
    let container = container.expect("no container");
    assert_eq!(container.get_type(), ContentPresenter::TYPE);
    container.cast::<ContentPresenter>().unwrap()
}

fn realized_contents(target: &VirtualizingStackPanel) -> Vec<String> {
    target
        .get_realized_elements()
        .into_iter()
        .map(|x| {
            let presenter = x.and_then(|x| x.cast::<ContentPresenter>()).expect("not a content presenter");
            presenter.content().as_ref().and_then(string_of).expect("the content is not a string")
        })
        .collect()
}

fn id(control: &Ref<Control>) -> usize {
    &**control as *const Control as usize
}

/// Declares one test per case of a parameterized test.
macro_rules! theory {
    ($func:ident: $($name:ident($($arg:expr),* $(,)?));+ $(;)?) => {
        $(
            #[test]
            fn $name() {
                $func($($arg),*)
            }
        )+
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

fn creates_initial_items(buffer_factor: f64, expected_count: i32) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);

    assert_eq!(1000.0, t.scroll.extent().height);

    assert_realized_items(&t.target, &t.items_control, 0, expected_count);
}

theory!(creates_initial_items: creates_initial_items_1(0.0, 10); creates_initial_items_2(0.5, 20));

fn initializes_initial_control_items(buffer_factor: f64, expected_count: i32) {
    let _app = app();
    let items: Vec<Ref<Control>> = (0..100).map(|_| button(25.0, 10.0)).collect();
    let t = create_target(|o| {
        o.items = Some(control_items(&items));
        o.item_template = Some(None);
        o.buffer_factor = buffer_factor;
    });

    assert_eq!(1000.0, t.scroll.extent().height);

    assert_realized_control_items::<Button>(&t.target, &t.items_control, 0, expected_count);
}

// Buffer factor of 0.5. Since at start there is no room, the 10 additional
// items are just appended.
theory!(initializes_initial_control_items:
    initializes_initial_control_items_1(0.0, 10);
    initializes_initial_control_items_2(0.5, 20));

fn creates_reassigned_items(buffer_factor: f64, expected_count: i32) {
    let _app = app();
    let t = create_target(|o| {
        o.items = Some(ItemsSource::from_items([]));
        o.buffer_factor = buffer_factor;
    });

    assert!(t.items_control.get_realized_containers().is_empty());

    t.items_control.set_items_source(Some(ItemsSource::from_strs(["foo", "bar"])));
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, 0, expected_count);
}

theory!(creates_reassigned_items: creates_reassigned_items_1(0.0, 2); creates_reassigned_items_2(0.5, 2));

fn scrolls_down_one_item(buffer_factor: f64, expected_first_index: i32, expected_count: i32) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);

    t.scroll.set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, expected_first_index, expected_count);
}

theory!(scrolls_down_one_item: scrolls_down_one_item_1(0.0, 1, 10); scrolls_down_one_item_2(0.5, 0, 20));

fn scrolls_down_more_than_a_page(buffer_factor: f64, expected_first_index: i32, expected_count: i32) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, expected_first_index, expected_count);
}

theory!(scrolls_down_more_than_a_page:
    scrolls_down_more_than_a_page_1(0.0, 20, 10);
    scrolls_down_more_than_a_page_2(0.5, 15, 20));

fn scrolls_down_to_index(buffer_factor: f64, expected_first_index: i32, expected_count: i32) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);

    t.target.scroll_into_view(20);

    assert_realized_items(&t.target, &t.items_control, expected_first_index, expected_count);
}

theory!(scrolls_down_to_index: scrolls_down_to_index_1(0.0, 11, 10); scrolls_down_to_index_2(0.5, 6, 20));

fn scrolls_up_to_index(buffer_factor: f64, first_realized_index: i32, expected_first_index: i32, expected_count: i32) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);

    t.scroll.scroll_to_end();
    layout(&t.target);

    assert_eq!(first_realized_index, t.target.first_realized_index());

    t.target.scroll_into_view(20);

    assert_realized_items(&t.target, &t.items_control, expected_first_index, expected_count);
}

theory!(scrolls_up_to_index: scrolls_up_to_index_1(0.0, 90, 20, 10); scrolls_up_to_index_2(0.5, 80, 15, 20));

fn scroll_to_end_arrives_at_end_when_items_have_different_sizes(orientation: Orientation, buffer_factor: f64) {
    let _app = app();
    let horizontal = orientation == Orientation::Horizontal;
    let size = |x: i32| if x < 60 { 20.0 } else { 50.0 };
    let (items, count) = if horizontal {
        let items: Vec<_> = (0..100).map(|x| ItemWithWidth::with_width(x, size(x))).collect();
        (models(&items), items.len())
    } else {
        let items: Vec<_> = (0..100).map(|x| ItemWithHeight::with_height(x, size(x))).collect();
        (models(&items), items.len())
    };
    let t = create_unrooted_target(|o| {
        o.items = Some(items);
        o.item_template =
            Some(Some(if horizontal { canvas_with_width_template() } else { canvas_with_height_template() }));
        o.orientation = orientation;
        o.buffer_factor = buffer_factor;
    });
    t.scroll.set_template(Some(scroll_viewer_template_with_scroll_bars()));
    create_root(Some(t.items_control.clone().upcast()), None, &[]).execute_initial_layout_pass();

    let scroll_bar = if horizontal { t.scroll.horizontal_scroll_bar() } else { t.scroll.vertical_scroll_bar() };
    if let Some(scroll_bar) = scroll_bar {
        scroll_bar.scroll_to_end();
    }
    layout(&t.target);

    assert_eq!(
        if horizontal {
            t.scroll.extent().width - t.scroll.viewport().width
        } else {
            t.scroll.extent().height - t.scroll.viewport().height
        },
        if horizontal { t.scroll.offset().x } else { t.scroll.offset().y }
    );
    assert_eq!(count as i32 - 1, t.target.last_realized_index());
    assert_eq!(
        if horizontal { t.target.view_port().right() } else { t.target.view_port().bottom() },
        if horizontal { last_realized(&t.target).bounds().right() } else { last_realized(&t.target).bounds().bottom() }
    );
}

theory!(scroll_to_end_arrives_at_end_when_items_have_different_sizes:
    scroll_to_end_arrives_at_end_when_items_have_different_sizes_1(Orientation::Vertical, 0.0);
    scroll_to_end_arrives_at_end_when_items_have_different_sizes_2(Orientation::Vertical, 0.5);
    scroll_to_end_arrives_at_end_when_items_have_different_sizes_3(Orientation::Horizontal, 0.0);
    scroll_to_end_arrives_at_end_when_items_have_different_sizes_4(Orientation::Horizontal, 0.5));

#[test]
fn item_of_content_sized_panel_is_positioned_at_start() {
    let _app = app();

    let items = Rc::new(FerroList::<String>::new());
    let t = create_unrooted_target(|o| o.items = Some(items.clone().into()));

    // Size the control to its content rather than to the viewport.
    t.items_control.set_vertical_alignment(VerticalAlignment::Top);

    let root = create_root(Some(t.items_control.clone().upcast()), None, &[]);
    root.execute_initial_layout_pass();

    items.add("Item 0".to_string());
    layout(&t.target);

    let container = t.target.container_from_index(0).expect("no container");

    assert_eq!(0.0, container.bounds().y);
    assert_eq!(container.bounds().height, t.target.bounds().height);
}

fn scrolling_up_to_index_does_not_create_a_page_of_unrealized_elements(buffer_factor: f64, expected_count: usize) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);

    t.scroll.scroll_to_end();
    layout(&t.target);
    t.target.scroll_into_view(20);

    assert_eq!(expected_count, t.target.children().count());
}

theory!(scrolling_up_to_index_does_not_create_a_page_of_unrealized_elements:
    scrolling_up_to_index_does_not_create_a_page_of_unrealized_elements_1(0.0, 10);
    scrolling_up_to_index_does_not_create_a_page_of_unrealized_elements_2(0.5, 20));

fn creates_elements_on_item_insert_1(
    buffer_factor: f64,
    first_count: usize,
    second_count: usize,
    indexes_raw: &str,
    third_count: i32,
) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();

    assert_eq!(first_count, t.target.get_realized_elements().len());

    items.insert(0, "new".to_string());

    assert_eq!(second_count, t.target.get_realized_elements().len());

    let indexes = get_realized_indexes(&t.target, &t.items_control);

    // Blank space inserted in realized elements and subsequent indexes
    // updated.
    assert_eq!(parse_indexes(indexes_raw, ", "), indexes);

    let mut elements = t.target.get_realized_elements();
    layout(&t.target);

    let indexes = get_realized_indexes(&t.target, &t.items_control);

    // After layout an element for the new element is created.
    assert_eq!(range(0, third_count), indexes);

    // But apart from the new element and the removed last element, all
    // existing elements should be the same.
    elements[0] = t.target.get_realized_elements()[0].clone();
    elements.pop();
    assert_eq!(elements, t.target.get_realized_elements());
}

theory!(creates_elements_on_item_insert_1:
    creates_elements_on_item_insert_1_1(0.0, 10, 11, "-1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10", 10);
    creates_elements_on_item_insert_1_2(
        0.5,
        20,
        21,
        "-1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20",
        20,
    ));

fn creates_elements_on_item_insert_2(
    buffer_factor: f64,
    first_count: usize,
    second_count: usize,
    indexes_raw: &str,
    third_count: i32,
) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();

    assert_eq!(first_count, t.target.get_realized_elements().len());

    items.insert(2, "new".to_string());

    assert_eq!(second_count, t.target.get_realized_elements().len());

    let indexes = get_realized_indexes(&t.target, &t.items_control);

    // Blank space inserted in realized elements and subsequent indexes
    // updated.
    assert_eq!(parse_indexes(indexes_raw, ", "), indexes);

    let mut elements = t.target.get_realized_elements();
    layout(&t.target);

    let indexes = get_realized_indexes(&t.target, &t.items_control);

    // After layout an element for the new element is created.
    assert_eq!(range(0, third_count), indexes);

    // But apart from the new element and the removed last element, all
    // existing elements should be the same.
    elements[2] = t.target.get_realized_elements()[2].clone();
    elements.pop();
    assert_eq!(elements, t.target.get_realized_elements());
}

theory!(creates_elements_on_item_insert_2:
    creates_elements_on_item_insert_2_1(0.0, 10, 11, "0, 1, -1, 3, 4, 5, 6, 7, 8, 9, 10", 10);
    creates_elements_on_item_insert_2_2(
        0.5,
        20,
        21,
        "0, 1, -1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20",
        20,
    ));

fn updates_elements_on_item_moved(buffer_factor: f64) {
    // Arrange

    let _app = app();

    let actual_items = Rc::new(FerroList::from_items((0..100).map(|x| format!("Item {x}"))));

    let t = create_target(|o| {
        o.items = Some(actual_items.clone().into());
        o.buffer_factor = buffer_factor;
    });

    let expected_realized_element_contents = [1, 2, 0, 3, 4, 5, 6, 7, 8, 9].map(|x| format!("Item {x}"));

    // Act

    actual_items.move_item(0, 2);
    layout(&t.target);

    // Assert

    let actual_realized_element_contents = realized_contents(&t.target);

    for expected in expected_realized_element_contents.iter() {
        assert!(actual_realized_element_contents.contains(expected), "{expected} is not realized");
    }
}

theory!(updates_elements_on_item_moved: updates_elements_on_item_moved_1(0.0); updates_elements_on_item_moved_2(0.5));

fn updates_elements_on_item_range_moved(buffer_factor: f64) {
    // Arrange

    let _app = app();

    let actual_items = Rc::new(FerroList::from_items((0..100).map(|x| format!("Item {x}"))));

    let t = create_target(|o| {
        o.items = Some(actual_items.clone().into());
        o.buffer_factor = buffer_factor;
    });

    let expected_realized_element_contents = [2, 0, 1, 3, 4, 5, 6, 7, 8, 9].map(|x| format!("Item {x}"));

    // Act

    actual_items.move_range(0, 2, 3);
    layout(&t.target);

    // Assert

    let actual_realized_element_contents = realized_contents(&t.target);

    for expected in expected_realized_element_contents.iter() {
        assert!(actual_realized_element_contents.contains(expected), "{expected} is not realized");
    }
}

theory!(updates_elements_on_item_range_moved:
    updates_elements_on_item_range_moved_1(0.0);
    updates_elements_on_item_range_moved_2(0.5));

fn updates_elements_on_item_remove(buffer_factor: f64, first_count: i32, second_count: i32) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();

    assert_eq!(first_count as usize, t.target.get_realized_elements().len());

    let to_recycle = t.target.get_realized_elements()[2].clone();
    items.remove_at(2);

    let indexes = get_realized_indexes(&t.target, &t.items_control);

    // Item removed from realized elements and subsequent row indexes updated.
    assert_eq!(range(0, second_count), indexes);

    let mut elements = t.target.get_realized_elements();
    layout(&t.target);

    let indexes = get_realized_indexes(&t.target, &t.items_control);

    // After layout an element for the newly visible last row is created and
    // indexes updated.
    assert_eq!(range(0, first_count), indexes);

    // And the removed row should now have been recycled as the last row.
    elements.push(to_recycle);
    assert_eq!(elements, t.target.get_realized_elements());
}

theory!(updates_elements_on_item_remove:
    updates_elements_on_item_remove_1(0.0, 10, 9);
    updates_elements_on_item_remove_2(0.5, 20, 19));

fn updates_elements_on_item_replace(buffer_factor: f64, first_count: i32, indexes_raw: &str) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();

    assert_eq!(first_count as usize, t.target.get_realized_elements().len());

    let to_replace = t.target.get_realized_elements()[2].clone();
    items.set(2, "new".to_string());

    // Container being replaced should have been recycled.
    assert!(!t.target.get_realized_elements().contains(&to_replace));
    assert!(!to_replace.unwrap().is_visible());

    let indexes = get_realized_indexes(&t.target, &t.items_control);

    // Item removed from realized elements at old position and space inserted
    // at new position.
    assert_eq!(parse_indexes(indexes_raw, ", "), indexes);

    layout(&t.target);

    let indexes = get_realized_indexes(&t.target, &t.items_control);

    // After layout the missing container should have been created.
    assert_eq!(range(0, first_count), indexes);
}

theory!(updates_elements_on_item_replace:
    updates_elements_on_item_replace_1(0.0, 10, "0, 1, -1, 3, 4, 5, 6, 7, 8, 9");
    updates_elements_on_item_replace_2(
        0.5,
        20,
        "0, 1, -1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19",
    ));

fn updates_elements_on_item_move(buffer_factor: f64, first_count: i32, indexes_raw: &str) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();

    assert_eq!(first_count as usize, t.target.get_realized_elements().len());

    let to_move = t.target.get_realized_elements()[2].clone();
    items.move_item(2, 6);

    // Container being moved should have been recycled.
    assert!(!t.target.get_realized_elements().contains(&to_move));
    assert!(!to_move.unwrap().is_visible());

    let indexes = get_realized_indexes(&t.target, &t.items_control);

    // Item removed from realized elements at old position and space inserted
    // at new position.
    assert_eq!(parse_indexes(indexes_raw, ", "), indexes);

    layout(&t.target);

    let indexes = get_realized_indexes(&t.target, &t.items_control);

    // After layout the missing container should have been created.
    assert_eq!(range(0, first_count), indexes);
}

theory!(updates_elements_on_item_move:
    updates_elements_on_item_move_1(0.0, 10, "0, 1, 2, 3, 4, 5, -1, 7, 8, 9");
    updates_elements_on_item_move_2(
        0.5,
        20,
        "0, 1, 2, 3, 4, 5, -1, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19",
    ));

fn removes_control_items_from_panel_on_item_remove(buffer_factor: f64) {
    let _app = app();
    let items = Rc::new(FerroList::from_items((0..100).map(|_| button(25.0, 10.0))));
    let t = create_target(|o| {
        o.items = Some(items.clone().into());
        o.item_template = Some(None);
        o.buffer_factor = buffer_factor;
    });

    assert_eq!(1000.0, t.scroll.extent().height);

    let removed = items.get(1);
    items.remove_at(1);

    assert!(removed.parent().is_none());
    assert!(removed.visual_parent().is_none());
}

theory!(removes_control_items_from_panel_on_item_remove:
    removes_control_items_from_panel_on_item_remove_1(0.0);
    removes_control_items_from_panel_on_item_remove_2(0.5));

fn does_not_recycle_focused_element(buffer_factor: f64) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);

    let focused = first_realized(&t.target);
    focused.set_focusable(true);
    focused.focus();
    assert!(first_realized(&t.target).is_keyboard_focus_within());

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    for x in t.target.get_realized_elements() {
        assert!(!x.unwrap().is_keyboard_focus_within());
    }
}

theory!(does_not_recycle_focused_element:
    does_not_recycle_focused_element_1(0.0);
    does_not_recycle_focused_element_2(0.5));

fn removing_item_of_focused_element_clears_focus(buffer_factor: f64) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();

    let focused = first_realized(&t.target);
    focused.set_focusable(true);
    focused.focus();
    assert!(focused.is_keyboard_focus_within());
    assert_eq!(
        Some(focused.clone().upcast::<InputElement>()),
        KeyboardNavigation::get_tab_once_active_element(&t.items_control)
    );

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    items.remove_at(0);

    for x in t.target.get_realized_elements() {
        assert!(!x.unwrap().is_keyboard_focus_within());
    }
    for x in t.target.get_realized_elements() {
        assert_ne!(Some(focused.clone()), x);
    }
}

theory!(removing_item_of_focused_element_clears_focus:
    removing_item_of_focused_element_clears_focus_1(0.0);
    removing_item_of_focused_element_clears_focus_2(0.5));

fn scrolling_back_to_focused_element_uses_correct_element(buffer_factor: f64) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);

    let focused = first_realized(&t.target);
    focused.set_focusable(true);
    focused.focus();
    assert!(focused.is_keyboard_focus_within());

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    t.scroll.set_offset(Vector::new(0.0, 0.0));
    layout(&t.target);

    assert_eq!(focused, first_realized(&t.target));
}

theory!(scrolling_back_to_focused_element_uses_correct_element:
    scrolling_back_to_focused_element_uses_correct_element_1(0.0);
    scrolling_back_to_focused_element_uses_correct_element_2(0.5));

fn focused_element_outside_realized_range_is_not_arranged_in_viewport(buffer_factor: f64) {
    let _app = app();

    // Item 0 is much taller than the others, so the average-based position
    // estimated for it once it has left the realized range lands inside the
    // viewport (#17935).
    let items: Vec<_> =
        (0..100).map(|x| ItemWithHeight::with_height(x, if x == 0 { 100.0 } else { 10.0 })).collect();
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_height_template()));
        o.buffer_factor = buffer_factor;
    });

    let focused = first_realized(&t.target);
    focused.set_focusable(true);
    focused.focus();
    assert!(focused.is_keyboard_focus_within());

    t.scroll.set_offset(Vector::new(0.0, 160.0));
    layout(&t.target);

    let viewport =
        Rect::new(t.scroll.offset().x, t.scroll.offset().y, t.scroll.viewport().width, t.scroll.viewport().height);
    assert!(focused.is_keyboard_focus_within());
    assert!(!focused.bounds().intersects(viewport));

    t.scroll.set_offset(Vector::new(0.0, 0.0));
    layout(&t.target);

    assert_eq!(focused, first_realized(&t.target));
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), focused.bounds());
}

theory!(focused_element_outside_realized_range_is_not_arranged_in_viewport:
    focused_element_outside_realized_range_is_not_arranged_in_viewport_1(0.0);
    focused_element_outside_realized_range_is_not_arranged_in_viewport_2(0.5));

fn focusing_another_element_recycles_original_focus_element(buffer_factor: f64) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);

    let original_focused = first_realized(&t.target);
    original_focused.set_focusable(true);
    original_focused.focus();

    t.scroll.set_offset(Vector::new(0.0, 500.0));
    layout(&t.target);

    let new_focused = first_realized(&t.target);
    new_focused.set_focusable(true);
    new_focused.focus();

    assert!(!original_focused.is_visible());
}

theory!(focusing_another_element_recycles_original_focus_element:
    focusing_another_element_recycles_original_focus_element_1(0.0);
    focusing_another_element_recycles_original_focus_element_2(0.5));

fn focused_element_losing_focus_does_not_reset_selection(buffer_factor: f64) {
    let _app = app();
    let t = create_target(|o| {
        o.items_control = Some(ListBox::new().upcast());
        o.styles = vec![Style::with_setters(
            Selectors::of_type::<ListBoxItem>(),
            [Setter::new(TemplatedControl::template_property(), Some(list_box_item_template()))],
        )];
        o.buffer_factor = buffer_factor;
    });
    let list_box = t.items_control.cast::<ListBox>().unwrap();

    list_box.set_selected_index(0);

    let selected_container = first_realized(&t.target);
    selected_container.set_focusable(true);
    selected_container.focus();

    t.scroll.set_offset(Vector::new(0.0, 500.0));
    layout(&t.target);

    let new_focused = first_realized(&t.target);
    new_focused.set_focusable(true);
    new_focused.focus();

    assert_eq!(0, list_box.selected_index());
}

theory!(focused_element_losing_focus_does_not_reset_selection:
    focused_element_losing_focus_does_not_reset_selection_1(0.0);
    focused_element_losing_focus_does_not_reset_selection_2(0.5));

fn removing_range_when_scrolled_to_end_updates_viewport(
    buffer_factor: f64,
    first_index: i32,
    second_index: i32,
    count: i32,
) {
    let _app = app();
    let items = Rc::new(FerroList::from_items((0..100).map(|x| format!("Item {x}"))));
    let t = create_target(|o| {
        o.items = Some(items.clone().into());
        o.buffer_factor = buffer_factor;
    });

    t.scroll.set_offset(Vector::new(0.0, 900.0));
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, first_index, count);

    items.remove_range(0, 80);
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, second_index, count);
    assert_eq!(Vector::new(0.0, 100.0), t.scroll.offset());
}

theory!(removing_range_when_scrolled_to_end_updates_viewport:
    removing_range_when_scrolled_to_end_updates_viewport_1(0.0, 90, 10, 10);
    removing_range_when_scrolled_to_end_updates_viewport_2(0.5, 80, 0, 20));

fn removing_range_to_have_less_than_a_page_of_items_when_scrolled_to_end_updates_viewport(
    buffer_factor: f64,
    first_index: i32,
    count: i32,
) {
    let _app = app();
    let items = Rc::new(FerroList::from_items((0..100).map(|x| format!("Item {x}"))));
    let t = create_target(|o| {
        o.items = Some(items.clone().into());
        o.buffer_factor = buffer_factor;
    });

    t.scroll.set_offset(Vector::new(0.0, 900.0));
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, first_index, count);

    items.remove_range(0, 95);
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, 0, 5);
    assert_eq!(Vector::new(0.0, 0.0), t.scroll.offset());
}

theory!(removing_range_to_have_less_than_a_page_of_items_when_scrolled_to_end_updates_viewport:
    removing_range_to_have_less_than_a_page_of_items_when_scrolled_to_end_updates_viewport_1(0.0, 90, 10);
    removing_range_to_have_less_than_a_page_of_items_when_scrolled_to_end_updates_viewport_2(0.5, 80, 20));

fn resetting_collection_to_have_less_items_when_scrolled_to_end_updates_viewport(
    buffer_factor: f64,
    first_index: i32,
    second_index: i32,
    count: i32,
) {
    let _app = app();
    let items = ResettingCollection::new((0..100).map(|x| format!("Item {x}")));
    let t = create_target(|o| {
        o.items = Some(ItemsSource::new(items.clone()));
        o.buffer_factor = buffer_factor;
    });

    t.scroll.set_offset(Vector::new(0.0, 900.0));
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, first_index, count);

    items.reset((0..20).map(|x| format!("Item {x}")));
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, second_index, count);
    assert_eq!(Vector::new(0.0, 100.0), t.scroll.offset());
}

theory!(resetting_collection_to_have_less_items_when_scrolled_to_end_updates_viewport:
    resetting_collection_to_have_less_items_when_scrolled_to_end_updates_viewport_1(0.0, 90, 10, 10);
    resetting_collection_to_have_less_items_when_scrolled_to_end_updates_viewport_2(0.5, 80, 0, 20));

fn resetting_collection_to_have_less_than_a_page_of_items_when_scrolled_to_end_updates_viewport(
    buffer_factor: f64,
    first_index: i32,
    count: i32,
) {
    let _app = app();
    let items = ResettingCollection::new((0..100).map(|x| format!("Item {x}")));
    let t = create_target(|o| {
        o.items = Some(ItemsSource::new(items.clone()));
        o.buffer_factor = buffer_factor;
    });

    t.scroll.set_offset(Vector::new(0.0, 900.0));
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, first_index, count);

    items.reset((0..5).map(|x| format!("Item {x}")));
    layout(&t.target);

    assert_realized_items(&t.target, &t.items_control, 0, 5);
    assert_eq!(Vector::new(0.0, 0.0), t.scroll.offset());
}

theory!(resetting_collection_to_have_less_than_a_page_of_items_when_scrolled_to_end_updates_viewport:
    resetting_collection_to_have_less_than_a_page_of_items_when_scrolled_to_end_updates_viewport_1(0.0, 90, 10);
    resetting_collection_to_have_less_than_a_page_of_items_when_scrolled_to_end_updates_viewport_2(0.5, 80, 20));

#[test]
fn shrinking_viewport_then_growing_back_triggers_remeasure() {
    // Regression test for a stale extended viewport comparison in the
    // effective viewport changed handler.
    //
    // When the viewport shrinks (e.g., a popup shrinks during filtering), the
    // handler doesn't trigger a measure (the smaller viewport is within the
    // old extended viewport). The extended viewport comparison baseline is
    // NOT updated. When the viewport later grows back, the handler compares
    // against the stale large extended viewport, concludes "no significant
    // change", and skips the measure. This prevents item realization when the
    // only measure trigger is the effective viewport change.
    //
    // The fix uses a separate last known extended viewport that is always
    // updated, so the comparison correctly detects viewport growth after a
    // shrink.
    //
    // Key: the scroll host passes infinite height for vertical scroll, so the
    // panel's measure override is NOT called from the layout cascade when
    // only the root size changes. The effective viewport change is the sole
    // measure trigger.
    let _app = app();

    let t = create_unrooted_target(|o| {
        o.items = Some(ItemsSource::from_values((0..20).map(|x| format!("Item {x}"))));
        o.panel = Some(CountingPanel::new().upcast());
        o.buffer_factor = 0.0;
    });
    let root = create_root(Some(t.items_control.clone().upcast()), Some(Size::new(100.0, 100.0)), &[]);

    root.execute_initial_layout_pass();

    // Initial state: viewport 0-100, 10 items visible, extended viewport =
    // (0,0,100,100).
    assert_realized_items(&t.target, &t.items_control, 0, 10);

    // Shrink viewport (simulates popup shrinking when items are filtered).
    // The panel's measure override is NOT called (the scroll host passes
    // infinite height). The effective viewport changes to the small viewport
    // but no measure is needed because the small viewport is within the old
    // extended viewport.
    root.set_client_size(Size::new(100.0, 10.0));
    root.invalidate_measure();
    layout(&t.target);

    // Reset counters after shrink.
    t.counting_panel().reset_measure_arrange_counters();

    // Grow viewport back (simulates popup growing when filter is removed).
    // The panel's measure override is NOT called from the layout cascade
    // (same infinite constraint). The effective viewport change is the ONLY
    // path to trigger a remeasure.
    root.set_client_size(Size::new(100.0, 100.0));
    root.invalidate_measure();
    layout(&t.target);

    // Without fix: the handler compares the new viewport (0-100) against the
    // stale extended viewport (0-100, never updated during shrink). Sees no
    // change. No remeasure triggered. Measure count = 0.
    //
    // With fix: compares against the last known extended viewport (0-10,
    // updated during shrink). Detects that viewport grew past it (100 > 10).
    // Measure invalidated. Measure count >= 1.
    assert!(
        t.counting_panel().measured.get() >= 1,
        "Panel should be re-measured when viewport grows back after a previous shrink. \
         The effective viewport changed handler must detect viewport growth by comparing against \
         the last known extended viewport, not the stale extended viewport."
    );
}

fn nth_child_selector_works(buffer_factor: f64, count: usize, indexes_raw: &str) {
    let _app = app();

    let style = Style::with_setters(
        Selectors::of_type::<ContentPresenter>().nth_child(5, 0),
        [Setter::new(ContentPresenter::background_property(), brush(Brushes::red()))],
    );

    let t = create_target(|o| {
        o.styles = vec![style];
        o.buffer_factor = buffer_factor;
    });
    let realized: Vec<_> = realized_containers(&t.target).into_iter().map(|x| content_presenter(Some(x))).collect();

    assert_eq!(count, realized.len());

    for (i, container) in realized.iter().enumerate() {
        let index = t.target.index_from_container(&container.clone().upcast());
        let red_indexes = parse_indexes(indexes_raw, ",");
        let expected_background = if red_indexes.contains(&(i as i32)) { brush(Brushes::red()) } else { None };

        assert_eq!(i as i32, index);
        assert_eq!(expected_background, container.background());
    }
}

theory!(nth_child_selector_works:
    nth_child_selector_works_1(0.0, 10, "4,9");
    nth_child_selector_works_2(0.5, 20, "4,9,14,19"));

fn canvas_child_background(container: &ContentPresenter) -> Option<Rc<dyn IBrush>> {
    container.child().and_then(|child| child.cast::<Canvas>()).expect("the child is not a canvas").background()
}

// Issue 12838
fn nth_child_selector_works_for_item_template_children(buffer_factor: f64, count: usize, indexes_raw: &str) {
    let _app = app();

    let style = Style::with_setters(
        Selectors::of_type::<ContentPresenter>().nth_child(5, 0).child().of_type::<Canvas>(),
        [Setter::new(Panel::background_property(), brush(Brushes::red()))],
    );

    let t = create_target(|o| {
        o.styles = vec![style];
        o.buffer_factor = buffer_factor;
    });
    let realized: Vec<_> = realized_containers(&t.target).into_iter().map(|x| content_presenter(Some(x))).collect();

    assert_eq!(count, realized.len());

    for (i, container) in realized.iter().enumerate() {
        let index = t.target.index_from_container(&container.clone().upcast());
        let red_indexes = parse_indexes(indexes_raw, ",");
        let expected_background = if red_indexes.contains(&(i as i32)) { brush(Brushes::red()) } else { None };

        assert_eq!(i as i32, index);
        assert_eq!(expected_background, canvas_child_background(container));
    }
}

theory!(nth_child_selector_works_for_item_template_children:
    nth_child_selector_works_for_item_template_children_1(0.0, 10, "4,9");
    nth_child_selector_works_for_item_template_children_2(0.5, 20, "4,9,14,19"));

fn nth_last_child_selector_works(buffer_factor: f64, count: usize, indexes_raw: &str) {
    let _app = app();

    let style = Style::with_setters(
        Selectors::of_type::<ContentPresenter>().nth_last_child(5, 0),
        [Setter::new(ContentPresenter::background_property(), brush(Brushes::red()))],
    );

    let t = create_target(|o| {
        o.styles = vec![style];
        o.buffer_factor = buffer_factor;
    });
    let realized: Vec<_> = realized_containers(&t.target).into_iter().map(|x| content_presenter(Some(x))).collect();

    assert_eq!(count, realized.len());

    for (i, container) in realized.iter().enumerate() {
        let index = t.target.index_from_container(&container.clone().upcast());
        let red_indexes = parse_indexes(indexes_raw, ",");
        let expected_background = if red_indexes.contains(&(i as i32)) { brush(Brushes::red()) } else { None };

        assert_eq!(i as i32, index);
        assert_eq!(expected_background, container.background());
    }
}

theory!(nth_last_child_selector_works:
    nth_last_child_selector_works_1(0.0, 10, "0,5");
    nth_last_child_selector_works_2(0.5, 20, "0,5,10,15"));

// Issue 12838
fn nth_last_child_selector_works_for_item_template_children(buffer_factor: f64, count: usize, indexes_raw: &str) {
    let _app = app();

    let style = Style::with_setters(
        Selectors::of_type::<ContentPresenter>().nth_last_child(5, 0).child().of_type::<Canvas>(),
        [Setter::new(Panel::background_property(), brush(Brushes::red()))],
    );

    let t = create_target(|o| {
        o.styles = vec![style];
        o.buffer_factor = buffer_factor;
    });
    let realized: Vec<_> = realized_containers(&t.target).into_iter().map(|x| content_presenter(Some(x))).collect();

    assert_eq!(count, realized.len());

    for (i, container) in realized.iter().enumerate() {
        let index = t.target.index_from_container(&container.clone().upcast());
        let red_indexes = parse_indexes(indexes_raw, ",");
        let expected_background = if red_indexes.contains(&(i as i32)) { brush(Brushes::red()) } else { None };

        assert_eq!(i as i32, index);
        assert_eq!(expected_background, canvas_child_background(container));
    }
}

theory!(nth_last_child_selector_works_for_item_template_children:
    nth_last_child_selector_works_for_item_template_children_1(0.0, 10, "0,5");
    nth_last_child_selector_works_for_item_template_children_2(0.5, 20, "0,5,10,15"));

fn container_prepared_is_raised_when_scrolling(buffer_factor: f64, expected_raised: i32) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let raised = Rc::new(Cell::new(0));

    let counter = raised.clone();
    t.items_control.container_prepared(move |_| counter.set(counter.get() + 1));

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    assert_eq!(expected_raised, raised.get());
}

theory!(container_prepared_is_raised_when_scrolling:
    container_prepared_is_raised_when_scrolling_1(0.0, 10);
    container_prepared_is_raised_when_scrolling_2(0.5, 15));

fn container_clearing_is_raised_when_scrolling(buffer_factor: f64, expected_raised: i32) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let raised = Rc::new(Cell::new(0));

    let counter = raised.clone();
    t.items_control.container_clearing(move |_| counter.set(counter.get() + 1));

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    assert_eq!(expected_raised, raised.get());
}

theory!(container_clearing_is_raised_when_scrolling:
    container_clearing_is_raised_when_scrolling_1(0.0, 10);
    container_clearing_is_raised_when_scrolling_2(0.5, 15));

fn container_index_changed_is_raised_on_insert(buffer_factor: f64, expected_raised: i32) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();
    let raised = Rc::new(Cell::new(0));
    let index = Rc::new(Cell::new(1));

    let (counter, expected_index) = (raised.clone(), index.clone());
    t.items_control.container_index_changed(move |e| {
        counter.set(counter.get() + 1);
        assert_eq!(expected_index.get(), e.old_index());
        expected_index.set(expected_index.get() + 1);
        assert_eq!(expected_index.get(), e.new_index());
    });

    items.insert(index.get() as usize, "new".to_string());

    assert_eq!(expected_raised, raised.get());
}

theory!(container_index_changed_is_raised_on_insert:
    container_index_changed_is_raised_on_insert_1(0.0, 9);
    container_index_changed_is_raised_on_insert_2(0.5, 19));

fn container_index_changed_is_raised_when_item_inserted_before_realized_elements(
    buffer_factor: f64,
    expected_raised: i32,
    index: i32,
) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();
    let raised = Rc::new(Cell::new(0));
    let index = Rc::new(Cell::new(index));

    let (counter, expected_index) = (raised.clone(), index.clone());
    t.items_control.container_index_changed(move |e| {
        counter.set(counter.get() + 1);
        assert_eq!(expected_index.get(), e.old_index());
        expected_index.set(expected_index.get() + 1);
        assert_eq!(expected_index.get(), e.new_index());
    });

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    items.insert(10, "new".to_string());

    assert_eq!(expected_raised, raised.get());
}

theory!(container_index_changed_is_raised_when_item_inserted_before_realized_elements:
    container_index_changed_is_raised_when_item_inserted_before_realized_elements_1(0.0, 10, 20);
    container_index_changed_is_raised_when_item_inserted_before_realized_elements_2(0.5, 20, 15));

fn container_index_changed_is_raised_on_remove(buffer_factor: f64, expected_raised: i32) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();
    let raised = Rc::new(Cell::new(0));
    let index = Rc::new(Cell::new(1));

    let (counter, expected_index) = (raised.clone(), index.clone());
    t.items_control.container_index_changed(move |e| {
        counter.set(counter.get() + 1);
        assert_eq!(expected_index.get() + 1, e.old_index());
        assert_eq!(expected_index.get(), e.new_index());
        expected_index.set(expected_index.get() + 1);
    });

    items.remove_at(index.get() as usize);

    assert_eq!(expected_raised, raised.get());
}

theory!(container_index_changed_is_raised_on_remove:
    container_index_changed_is_raised_on_remove_1(0.0, 8);
    container_index_changed_is_raised_on_remove_2(0.5, 18));

fn container_index_changed_is_raised_when_item_removed_before_realized_elements(
    buffer_factor: f64,
    expected_raised: i32,
    index: i32,
) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();
    let raised = Rc::new(Cell::new(0));
    let index = Rc::new(Cell::new(index));

    let (counter, expected_index) = (raised.clone(), index.clone());
    t.items_control.container_index_changed(move |e| {
        assert_eq!(expected_index.get(), e.old_index());
        assert_eq!(expected_index.get() - 1, e.new_index());
        expected_index.set(expected_index.get() + 1);
        counter.set(counter.get() + 1);
    });

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    items.remove_at(10);

    assert_eq!(expected_raised, raised.get());
}

theory!(container_index_changed_is_raised_when_item_removed_before_realized_elements:
    container_index_changed_is_raised_when_item_removed_before_realized_elements_1(0.0, 10, 20);
    container_index_changed_is_raised_when_item_removed_before_realized_elements_2(0.5, 20, 15));

fn fires_correct_container_lifecycle_events_on_replace(buffer_factor: f64) {
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();
    let events = Rc::new(RefCell::new(Vec::<String>::new()));

    let sink = events.clone();
    t.items_control.container_prepared(move |e| {
        sink.borrow_mut().push(format!("Prepared #{} = {}", id(e.container()), e.index()));
    });
    let sink = events.clone();
    t.items_control.container_clearing(move |e| {
        sink.borrow_mut().push(format!("Clearing #{}", id(e.container())));
    });
    let sink = events.clone();
    t.items_control.container_index_changed(move |e| {
        sink.borrow_mut().push(format!("IndexChanged #{} {} -> {}", id(e.container()), e.old_index(), e.new_index()));
    });

    let to_replace = t.target.get_realized_elements()[2].clone().unwrap();
    items.set(2, "New Item".to_string());

    assert_eq!(vec![format!("Clearing #{}", id(&to_replace))], *events.borrow());
    events.borrow_mut().clear();

    t.items_control.update_layout();

    assert_eq!(vec![format!("Prepared #{} = 2", id(&to_replace))], *events.borrow());
    events.borrow_mut().clear();
}

theory!(fires_correct_container_lifecycle_events_on_replace:
    fires_correct_container_lifecycle_events_on_replace_1(0.0);
    fires_correct_container_lifecycle_events_on_replace_2(0.5));

fn scrolling_down_with_larger_element_does_not_cause_jump_and_arrives_at_end(buffer_factor: f64) {
    let _app = app();

    let items: Vec<_> = (0..1000).map(ItemWithHeight::new).collect();
    items[20].set_height(200.0);

    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_height_template()));
        o.buffer_factor = buffer_factor;
    });

    let mut index = t.target.first_realized_index();

    // Scroll down to the larger element.
    while t.target.last_realized_index() < items.len() as i32 - 1 {
        t.scroll.line_down();
        layout(&t.target);

        assert!(
            t.target.first_realized_index() >= index,
            "{} is not greater or equal to {}",
            t.target.first_realized_index(),
            index
        );

        if t.scroll.offset().y + t.scroll.viewport().height == t.scroll.extent().height {
            assert_eq!(items.len() as i32 - 1, t.target.last_realized_index());
        }

        index = t.target.first_realized_index();
    }
}

theory!(scrolling_down_with_larger_element_does_not_cause_jump_and_arrives_at_end:
    scrolling_down_with_larger_element_does_not_cause_jump_and_arrives_at_end_1(0.0);
    scrolling_down_with_larger_element_does_not_cause_jump_and_arrives_at_end_2(0.5));

fn scrolling_up_to_larger_element_does_not_cause_jump(buffer_factor: f64) {
    let _app = app();

    let items: Vec<_> = (0..100).map(ItemWithHeight::new).collect();
    items[20].set_height(200.0);

    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_height_template()));
        o.buffer_factor = buffer_factor;
    });

    // Scroll past the larger element.
    t.scroll.set_offset(Vector::new(0.0, 600.0));
    layout(&t.target);

    // Precondition checks
    assert!(t.target.first_realized_index() > 20);

    let mut index = t.target.first_realized_index();

    // Scroll up to the top.
    while t.scroll.offset().y > 0.0 {
        t.scroll.line_up();
        layout(&t.target);

        assert!(
            t.target.first_realized_index() <= index,
            "{} is not less than {}",
            t.target.first_realized_index(),
            index
        );
        index = t.target.first_realized_index();
    }
}

theory!(scrolling_up_to_larger_element_does_not_cause_jump:
    scrolling_up_to_larger_element_does_not_cause_jump_1(0.0);
    scrolling_up_to_larger_element_does_not_cause_jump_2(0.5));

fn scrolling_up_to_smaller_element_does_not_cause_jump(buffer_factor: f64) {
    let _app = app();

    let items: Vec<_> = (0..100).map(|x| ItemWithHeight::with_height(x, 30.0)).collect();
    items[20].set_height(25.0);

    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_height_template()));
        o.buffer_factor = buffer_factor;
    });

    let additional_items_count = if buffer_factor == 0.0 {
        1.0
    } else {
        // Buffer factor of 0.5 and 7 visible items => will be rounded up to 4
        // => when we scroll up and are near the _extended_ viewport, 4
        // additional items will be inserted above the current viewport.
        (t.target.children().count() as f64 * t.target.cache_length()).round()
    };

    // Scroll past the larger element.
    t.scroll.set_offset(Vector::new(0.0, 25.0 * items[0].height()));
    layout(&t.target);

    // Precondition checks
    assert!(t.target.first_realized_index() > 20);

    let mut index = t.target.first_realized_index();

    // Scroll up to the top.
    while t.scroll.offset().y > 0.0 {
        t.scroll.set_offset(t.scroll.offset() - Vector::new(0.0, 5.0));
        layout(&t.target);

        assert!(
            t.target.first_realized_index() <= index,
            "{} is not less than {}",
            t.target.first_realized_index(),
            index
        );
        assert!(
            (index - t.target.first_realized_index()) as f64 <= additional_items_count,
            "FirstIndex changed from {} to {}",
            index,
            t.target.first_realized_index()
        );

        index = t.target.first_realized_index();
    }
}

theory!(scrolling_up_to_smaller_element_does_not_cause_jump:
    scrolling_up_to_smaller_element_does_not_cause_jump_1(0.0);
    scrolling_up_to_smaller_element_does_not_cause_jump_2(0.5));

fn does_not_throw_when_estimating_viewport_with_ancestor_margin(buffer_factor: f64) {
    // Issue #11272
    let _app = app();
    let t = create_unrooted_target(|o| o.buffer_factor = buffer_factor);
    let container = Decorator::new();
    container.set_margin(Thickness::new(100.0, 100.0, 100.0, 100.0));
    let root = TestRoot::with_child(container.clone());

    root.execute_initial_layout_pass();

    container.set_child(t.items_control.clone());

    root.layout_manager().execute_layout_pass();
}

theory!(does_not_throw_when_estimating_viewport_with_ancestor_margin:
    does_not_throw_when_estimating_viewport_with_ancestor_margin_1(0.0);
    does_not_throw_when_estimating_viewport_with_ancestor_margin_2(0.5));

fn supports_null_recycle_key_when_scrolling(buffer_factor: f64, offset: f64) {
    let _app = app();
    let t = create_unrooted_target(|o| {
        o.items_control = Some(NonRecyclingItemsControl::new().upcast());
        o.buffer_factor = buffer_factor;
    });
    let root = create_root(Some(t.items_control.clone().upcast()), None, &[]);

    root.execute_initial_layout_pass();

    let first_item = t.items_control.container_from_index(0).unwrap();
    t.scroll.set_offset(Vector::new(0.0, offset));

    layout(&t.items_control);

    assert!(first_item.parent().is_none());
    assert!(first_item.visual_parent().is_none());
    assert!(!t.items_control.items_panel_root().unwrap().children().contains(&first_item));
}

theory!(supports_null_recycle_key_when_scrolling:
    supports_null_recycle_key_when_scrolling_1(0.0, 20.0);
    supports_null_recycle_key_when_scrolling_2(0.5, 200.0));

fn supports_null_recycle_key_when_clearing_items(buffer_factor: f64) {
    let _app = app();
    let t = create_unrooted_target(|o| {
        o.items_control = Some(NonRecyclingItemsControl::new().upcast());
        o.buffer_factor = buffer_factor;
    });
    let root = create_root(Some(t.items_control.clone().upcast()), None, &[]);

    root.execute_initial_layout_pass();

    let first_item = t.items_control.container_from_index(0).unwrap();
    t.items_control.set_items_source(None);

    layout(&t.items_control);

    assert!(first_item.parent().is_none());
    assert!(first_item.visual_parent().is_none());
    assert!(t.items_control.items_panel_root().unwrap().children().is_empty());
}

theory!(supports_null_recycle_key_when_clearing_items:
    supports_null_recycle_key_when_clearing_items_1(0.0);
    supports_null_recycle_key_when_clearing_items_2(0.5));

fn scroll_into_view_on_effectively_invisible_panel_does_not_create_ghost_elements(buffer_factor: f64) {
    let _app = app();
    let items = || Some(ItemsSource::from_strs(["foo", "bar", "baz"]));
    let t = create_unrooted_target(|o| {
        o.items = items();
        o.buffer_factor = buffer_factor;
    });
    let container = Decorator::new();
    container.set_margin(Thickness::new(100.0, 100.0, 100.0, 100.0));
    container.set_child(t.items_control.clone());
    let root = TestRoot::with_child(container.clone());

    root.execute_initial_layout_pass();

    // Clear the items and do a layout to recycle all elements.
    t.items_control.set_items_source(None);
    root.layout_manager().execute_layout_pass();

    // Should have no realized elements and no unrealized elements.
    assert_eq!(0, t.target.get_realized_elements().len());
    assert_eq!(0, t.target.children().count());

    // Make the panel effectively invisible and set items.
    container.set_is_visible(false);
    t.items_control.set_items_source(items());

    // Try to scroll into view while effectively invisible.
    t.target.scroll_into_view(0);

    // Make the panel visible and layout.
    container.set_is_visible(true);
    root.layout_manager().execute_layout_pass();

    // Should have 3 realized elements and no unrealized elements.
    assert_eq!(3, t.target.get_realized_elements().len());
    assert_eq!(3, t.target.children().count());
}

theory!(scroll_into_view_on_effectively_invisible_panel_does_not_create_ghost_elements:
    scroll_into_view_on_effectively_invisible_panel_does_not_create_ghost_elements_1(0.0);
    scroll_into_view_on_effectively_invisible_panel_does_not_create_ghost_elements_2(0.5));

// Issue 10968
fn does_not_realize_items_if_self_outside_viewport(buffer_factor: f64) {
    let _app = app();
    let t = create_unrooted_target(|o| o.buffer_factor = buffer_factor);
    t.items_control.set_margin(Thickness::new(0.0, 200.0, 0.0, 0.0));

    let scroll_content_presenter = ScrollContentPresenter::new();
    scroll_content_presenter.set_width(100.0);
    scroll_content_presenter.set_height(100.0);
    scroll_content_presenter.set_content(Some(Control::boxed(t.items_control.clone())));

    let root = create_root(Some(scroll_content_presenter.clone().upcast()), None, &[]);
    root.execute_initial_layout_pass();
    assert_eq!(1, t.target.visual_children().count());

    scroll_content_presenter.set_content(None);
    root.layout_manager().execute_layout_pass();

    scroll_content_presenter.set_content(Some(Control::boxed(t.items_control.clone())));
    root.layout_manager().execute_layout_pass();

    assert_eq!(1, t.target.visual_children().count());
}

theory!(does_not_realize_items_if_self_outside_viewport:
    does_not_realize_items_if_self_outside_viewport_1(0.0);
    does_not_realize_items_if_self_outside_viewport_2(0.5));

fn alternating_backgrounds_should_be_correct_after_scrolling(
    buffer_factor: f64,
    first_index1: i32,
    last_index1: i32,
    first_index2: i32,
    last_index2: i32,
) {
    // Issue #12381.
    fn assert_colors(target: &VirtualizingStackPanel) {
        for container in realized_containers(target) {
            assert_eq!(container.get_type(), ListBoxItem::TYPE);
        }

        for i in target.first_realized_index()..=target.last_realized_index() {
            let container = target.container_from_index(i).expect("no container");
            assert_eq!(container.get_type(), ListBoxItem::TYPE);
            let container = container.cast::<ListBoxItem>().unwrap();
            let expected_background = if i % 2 == 0 { Colors::GREEN } else { Colors::RED };
            let background = container.background().expect("no background");
            let brush = background.as_solid_color_brush().expect("the background is not a solid color brush");

            assert_eq!(expected_background, brush.color());
        }
    }

    let _app = app();
    let styles = [
        Style::with_setters(
            Selectors::of_type::<ListBoxItem>(),
            [Setter::new(TemplatedControl::background_property(), brush(Brushes::white()))],
        ),
        Style::with_setters(
            Selectors::of_type::<ListBoxItem>().nth_child(2, 1),
            [Setter::new(TemplatedControl::background_property(), brush(Brushes::green()))],
        ),
        Style::with_setters(
            Selectors::of_type::<ListBoxItem>().nth_child(2, 0),
            [Setter::new(TemplatedControl::background_property(), brush(Brushes::red()))],
        ),
    ];
    let t = create_unrooted_target(|o| {
        o.items_control = Some(ListBox::new().upcast());
        o.buffer_factor = buffer_factor;
    });

    // We need to display an odd number of items to reproduce the issue.
    let root = create_root(Some(t.items_control.clone().upcast()), Some(Size::new(100.0, 90.0)), &styles);
    root.execute_initial_layout_pass();

    for container in realized_containers(&t.target) {
        assert_eq!(container.get_type(), ListBoxItem::TYPE);
    }

    assert_eq!(first_index1, t.target.first_realized_index());
    assert_eq!(last_index1, t.target.last_realized_index());
    assert_colors(&t.target);

    t.scroll.set_offset(Vector::new(0.0, 10.0));
    t.target.update_layout();

    assert_eq!(first_index2, t.target.first_realized_index());
    assert_eq!(last_index2, t.target.last_realized_index());
    assert_colors(&t.target);
}

theory!(alternating_backgrounds_should_be_correct_after_scrolling:
    alternating_backgrounds_should_be_correct_after_scrolling_1(0.0, 0, 8, 1, 9);
    alternating_backgrounds_should_be_correct_after_scrolling_2(0.5, 0, 17, 0, 17));

fn inserting_item_before_viewport_preserves_first_realized_index(buffer_factor: f64, first_index: i32) {
    // Issue #12744
    let _app = app();
    let t = create_target(|o| o.buffer_factor = buffer_factor);
    let items = t.items();

    // Scroll down 20 items.
    t.scroll.set_offset(Vector::new(0.0, 200.0));
    t.target.update_layout();
    assert_eq!(first_index, t.target.first_realized_index());

    // Insert an item at the beginning.
    items.insert(0, "New Item".to_string());
    t.target.update_layout();

    // The first realized index should still be 20 as the scroll should be
    // unchanged.
    assert_eq!(first_index, t.target.first_realized_index());
    assert_eq!(Vector::new(0.0, 200.0), t.scroll.offset());
}

theory!(inserting_item_before_viewport_preserves_first_realized_index:
    inserting_item_before_viewport_preserves_first_realized_index_1(0.0, 20);
    inserting_item_before_viewport_preserves_first_realized_index_2(0.5, 15));

fn can_bind_item_is_visible(buffer_factor: f64) {
    let _app = app();
    let style = create_is_visible_binding_style();
    let items: Vec<_> = (0..100).map(ItemWithIsVisible::new).collect();
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.styles = vec![style];
        o.buffer_factor = buffer_factor;
    });
    let container = t.target.container_from_index(2).unwrap();

    assert!(container.is_visible());
    assert_eq!(20.0, container.bounds().top());

    items[2].set_is_visible(false);
    layout(&t.target);

    assert!(!container.is_visible());

    // Next container should be in correct position.
    assert_eq!(20.0, t.target.container_from_index(3).unwrap().bounds().top());
}

theory!(can_bind_item_is_visible: can_bind_item_is_visible_1(0.0); can_bind_item_is_visible_2(0.5));

fn is_visible_binding_persists_after_scrolling(buffer_factor: f64) {
    let _app = app();
    let style = create_is_visible_binding_style();
    let items: Vec<_> = (0..100).map(ItemWithIsVisible::new).collect();
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.styles = vec![style];
        o.buffer_factor = buffer_factor;
    });
    let container = t.target.container_from_index(2).unwrap();

    assert!(container.is_visible());
    assert_eq!(20.0, container.bounds().top());

    items[2].set_is_visible(false);
    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    t.scroll.set_offset(Vector::new(0.0, 0.0));
    layout(&t.target);

    let container = t.target.container_from_index(2).unwrap();
    assert!(!container.is_visible());
}

theory!(is_visible_binding_persists_after_scrolling:
    is_visible_binding_persists_after_scrolling_1(0.0);
    is_visible_binding_persists_after_scrolling_2(0.5));

fn recycling_a_hidden_control_shows_it(buffer_factor: f64) {
    let _app = app();
    let style = create_is_visible_binding_style();
    let items_list: Vec<_> = (0..3).map(ItemWithIsVisible::new).collect();
    let items = Rc::new(FerroList::<BoxedValue>::from_items(items_list.iter().map(|item| {
        let item: BoxedValue = item.clone();
        item
    })));
    let t = create_target(|o| {
        o.items = Some(items.clone().into());
        o.styles = vec![style];
        o.buffer_factor = buffer_factor;
    });
    let container = t.target.container_from_index(2).unwrap();

    assert!(container.is_visible());
    assert_eq!(20.0, container.bounds().top());

    items_list[2].set_is_visible(false);
    layout(&t.target);

    assert!(!container.is_visible());

    items.remove_at(2);
    items.add(ItemWithIsVisible::new(3));
    layout(&t.target);

    assert!(container.is_visible());
}

theory!(recycling_a_hidden_control_shows_it:
    recycling_a_hidden_control_shows_it_1(0.0);
    recycling_a_hidden_control_shows_it_2(0.5));

fn scroll_into_view_with_target_rect_outside_viewport_should_scroll_to_item(buffer_factor: f64) {
    let _app = app();
    let items: Vec<_> = (0..101).map(|x| ItemWithHeight::with_height(x, (x * 100 + 1) as f64)).collect();
    let item_template = FuncDataTemplate::for_type::<ItemWithHeight>(
        |_, _| {
            let border = Border::new();
            border.set_height(10.0);
            border.bind_binding(Layoutable::width_property().as_property(), &ReflectionBinding::new("Height"));
            Some(border.upcast())
        },
        false,
    );
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(item_template));
        o.styles = vec![Style::with_setters(
            Selectors::of_type::<ScrollViewer>(),
            [Setter::new(ScrollViewer::horizontal_scroll_bar_visibility_property(), ScrollBarVisibility::Visible)],
        )];
        o.buffer_factor = buffer_factor;
    });
    let _subscription = t.items_control.container_prepared(|ev| {
        ev.container().add_handler(Control::request_bring_into_view_event(), |_, e| {
            let data_context = e.target_object.as_ref().unwrap().data_context().unwrap();
            let value: &dyn AnyValue = &*data_context;
            let data_context = value.downcast_ref::<ItemWithHeight>().unwrap();
            e.set_target_rect(Rect::new(data_context.height() - 50.0, 0.0, 50.0, 10.0));
        });
    });

    t.target.scroll_into_view(100);

    assert_eq!(9901.0, t.scroll.offset().x);
}

theory!(scroll_into_view_with_target_rect_outside_viewport_should_scroll_to_item:
    scroll_into_view_with_target_rect_outside_viewport_should_scroll_to_item_1(0.0);
    scroll_into_view_with_target_rect_outside_viewport_should_scroll_to_item_2(0.5));

fn scroll_into_view_correctly_scrolls_down_to_a_page_of_smaller_items(buffer_factor: f64, first_index: i32, count: i32) {
    let _app = app();

    // First 10 items have height of 20, next 10 have height of 10.
    let items: Vec<_> = (0..20).map(|x| ItemWithHeight::with_height(x, (((29 - x) / 10) * 10) as f64)).collect();
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_height_template()));
        o.buffer_factor = buffer_factor;
    });

    // Scroll the last item into view.
    t.target.scroll_into_view(19);

    // At the time of the scroll, the average item height is 20, so the
    // requested item should be placed at 380 (19 * 20) which therefore
    // results in an extent of 390 to accommodate the item height of 10. This
    // is obviously not a perfect answer, but it's the best we can do without
    // knowing the actual item heights.
    let container = content_presenter(t.target.container_from_index(19));
    assert_eq!(Rect::new(0.0, 380.0, 100.0, 10.0), container.bounds());
    assert_eq!(Size::new(100.0, 100.0), t.scroll.viewport());
    assert_eq!(Size::new(100.0, 390.0), t.scroll.extent());
    assert_eq!(Vector::new(0.0, 290.0), t.scroll.offset());

    // Items 10-19 should be visible.
    assert_realized_items(&t.target, &t.items_control, first_index, count);
}

theory!(scroll_into_view_correctly_scrolls_down_to_a_page_of_smaller_items:
    scroll_into_view_correctly_scrolls_down_to_a_page_of_smaller_items_1(0.0, 10, 10);
    scroll_into_view_correctly_scrolls_down_to_a_page_of_smaller_items_2(0.5, 5, 15));

fn scroll_into_view_correctly_scrolls_down_to_a_page_of_larger_items(
    buffer_factor: f64,
    first_index: i32,
    count: i32,
    y: f64,
    extent_height: f64,
    offset: f64,
) {
    let _app = app();

    // First 10 items have height of 10, next 10 have height of 20.
    let items: Vec<_> = (0..20).map(|x| ItemWithHeight::with_height(x, (((x / 10) + 1) * 10) as f64)).collect();
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_height_template()));
        o.buffer_factor = buffer_factor;
    });

    // Scroll the last item into view.
    t.target.scroll_into_view(19);

    // At the time of the scroll, the average item height is 10, so the
    // requested item should be placed at 190 (19 * 10) which therefore
    // results in an extent of 210 to accommodate the item height of 20. This
    // is obviously not a perfect answer, but it's the best we can do without
    // knowing the actual item heights.
    let container = content_presenter(t.target.container_from_index(19));
    assert_eq!(Rect::new(0.0, y, 100.0, 20.0), container.bounds());
    assert_eq!(Size::new(100.0, 100.0), t.scroll.viewport());
    assert_eq!(Size::new(100.0, extent_height), t.scroll.extent());
    assert_eq!(Vector::new(0.0, offset), t.scroll.offset());

    // Items 15-19 should be visible.
    assert_realized_items(&t.target, &t.items_control, first_index, count);
}

theory!(scroll_into_view_correctly_scrolls_down_to_a_page_of_larger_items:
    scroll_into_view_correctly_scrolls_down_to_a_page_of_larger_items_1(0.0, 15, 5, 190.0, 210.0, 110.0);
    scroll_into_view_correctly_scrolls_down_to_a_page_of_larger_items_2(0.5, 10, 10, 253.0, 273.0, 173.0));

fn scroll_into_view_correctly_scrolls_right_to_a_page_of_smaller_items(buffer_factor: f64, first_index: i32, count: i32) {
    let _app = app();

    // First 10 items have width of 20, next 10 have width of 10.
    let items: Vec<_> = (0..20).map(|x| ItemWithWidth::with_width(x, (((29 - x) / 10) * 10) as f64)).collect();
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_width_template()));
        o.orientation = Orientation::Horizontal;
        o.buffer_factor = buffer_factor;
    });

    // Scroll the last item into view.
    t.target.scroll_into_view(19);

    // At the time of the scroll, the average item width is 20, so the
    // requested item should be placed at 380 (19 * 20) which therefore
    // results in an extent of 390 to accommodate the item width of 10. This
    // is obviously not a perfect answer, but it's the best we can do without
    // knowing the actual item widths.
    let container = content_presenter(t.target.container_from_index(19));
    assert_eq!(Rect::new(380.0, 0.0, 10.0, 100.0), container.bounds());
    assert_eq!(Size::new(100.0, 100.0), t.scroll.viewport());
    assert_eq!(Size::new(390.0, 100.0), t.scroll.extent());
    assert_eq!(Vector::new(290.0, 0.0), t.scroll.offset());

    // Items 10-19 should be visible.
    assert_realized_items(&t.target, &t.items_control, first_index, count);
}

theory!(scroll_into_view_correctly_scrolls_right_to_a_page_of_smaller_items:
    scroll_into_view_correctly_scrolls_right_to_a_page_of_smaller_items_1(0.0, 10, 10);
    scroll_into_view_correctly_scrolls_right_to_a_page_of_smaller_items_2(0.5, 5, 15));

fn scroll_into_view_correctly_scrolls_right_to_a_page_of_larger_items(
    buffer_factor: f64,
    first_index: i32,
    count: i32,
    x: f64,
    extent_width: f64,
    offset: f64,
) {
    let _app = app();

    // First 10 items have width of 10, next 10 have width of 20.
    let items: Vec<_> = (0..20).map(|x| ItemWithWidth::with_width(x, (((x / 10) + 1) * 10) as f64)).collect();
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_width_template()));
        o.orientation = Orientation::Horizontal;
        o.buffer_factor = buffer_factor;
    });

    // Scroll the last item into view.
    t.target.scroll_into_view(19);

    // At the time of the scroll, the average item width is 10, so the
    // requested item should be placed at 190 (19 * 10) which therefore
    // results in an extent of 210 to accommodate the item width of 20. This
    // is obviously not a perfect answer, but it's the best we can do without
    // knowing the actual item widths.
    let container = content_presenter(t.target.container_from_index(19));
    assert_eq!(Rect::new(x, 0.0, 20.0, 100.0), container.bounds());
    assert_eq!(Size::new(100.0, 100.0), t.scroll.viewport());
    assert_eq!(Size::new(extent_width, 100.0), t.scroll.extent());
    assert_eq!(Vector::new(offset, 0.0), t.scroll.offset());

    // Items 15-19 should be visible.
    assert_realized_items(&t.target, &t.items_control, first_index, count);
}

theory!(scroll_into_view_correctly_scrolls_right_to_a_page_of_larger_items:
    scroll_into_view_correctly_scrolls_right_to_a_page_of_larger_items_1(0.0, 15, 5, 190.0, 210.0, 110.0);
    scroll_into_view_correctly_scrolls_right_to_a_page_of_larger_items_2(0.5, 10, 10, 253.0, 273.0, 173.0));

fn extent_and_offset_should_be_updated_when_containers_resize(
    buffer_factor: f64,
    first_index_1: i32,
    last_index_1: i32,
    first_index_2: i32,
    last_index_2: i32,
) {
    let _app = app();

    // All containers start off with a height of 50 (2 containers fit in
    // viewport).
    let items: Vec<_> = (0..20).map(|x| ItemWithHeight::with_height(x, 50.0)).collect();
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_height_template()));
        o.buffer_factor = buffer_factor;
    });

    // Scroll to the 5th item (containers 4 and 5 should be visible).
    t.target.scroll_into_view(5);
    assert_eq!(first_index_1, t.target.first_realized_index());
    assert_eq!(last_index_1, t.target.last_realized_index());

    // The extent should be 500 (10 * 50) and the offset should be 200 (4 *
    // 50).
    let container = content_presenter(t.target.container_from_index(5));
    assert_eq!(Rect::new(0.0, 250.0, 100.0, 50.0), container.bounds());
    assert_eq!(Size::new(100.0, 100.0), t.scroll.viewport());
    assert_eq!(Size::new(100.0, 1000.0), t.scroll.extent());
    assert_eq!(Vector::new(0.0, 200.0), t.scroll.offset());

    // Update the height of all items to 25 and run a layout pass.
    for item in &items {
        item.set_height(25.0);
    }
    t.target.update_layout();

    // The extent should be updated to reflect the new heights. The offset
    // should be unchanged but the first realized index should be updated to
    // 8 (200 / 25).
    assert_eq!(Size::new(100.0, 100.0), t.scroll.viewport());
    assert_eq!(Size::new(100.0, 500.0), t.scroll.extent());
    assert_eq!(Vector::new(0.0, 200.0), t.scroll.offset());
    assert_eq!(first_index_2, t.target.first_realized_index());
    assert_eq!(last_index_2, t.target.last_realized_index());
}

theory!(extent_and_offset_should_be_updated_when_containers_resize:
    extent_and_offset_should_be_updated_when_containers_resize_1(0.0, 4, 5, 8, 11);
    extent_and_offset_should_be_updated_when_containers_resize_2(0.5, 3, 6, 6, 13));

fn focused_container_is_positioned_correctly_when_container_size_change_causes_it_to_be_moved_out_of_visible_viewport(
    buffer_factor: f64,
    first_index_1: i32,
    last_index_1: i32,
    first_index_2: i32,
    last_index_2: i32,
) {
    let _app = app();

    // All containers start off with a height of 50 (2 containers fit in
    // viewport).
    let items: Vec<_> = (0..20).map(|x| ItemWithHeight::with_height(x, 50.0)).collect();
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_height_template()));
        o.buffer_factor = buffer_factor;
    });

    // Scroll to the 5th item (containers 4 and 5 should be visible).
    t.target.scroll_into_view(5);
    assert_eq!(first_index_1, t.target.first_realized_index());
    assert_eq!(last_index_1, t.target.last_realized_index());

    // Focus the 5th item.
    let container = content_presenter(t.target.container_from_index(5));
    container.set_focusable(true);
    container.focus();

    // Update the height of all items to 25 and run a layout pass.
    for item in &items {
        item.set_height(25.0);
    }
    t.target.update_layout();

    // The focused container should now be outside the realized range.
    assert_eq!(first_index_2, t.target.first_realized_index());
    assert_eq!(last_index_2, t.target.last_realized_index());

    // The container should still exist and be positioned outside the visible
    // viewport.
    let container = content_presenter(t.target.container_from_index(5));
    assert_eq!(Rect::new(0.0, 125.0, 100.0, 25.0), container.bounds());
}

theory!(
    focused_container_is_positioned_correctly_when_container_size_change_causes_it_to_be_moved_out_of_visible_viewport:
    focused_container_is_positioned_correctly_when_container_size_change_causes_it_to_be_moved_out_of_visible_viewport_1(
        0.0, 4, 5, 8, 11,
    );
    focused_container_is_positioned_correctly_when_container_size_change_causes_it_to_be_moved_out_of_visible_viewport_2(
        0.5, 3, 6, 6, 13,
    ));

#[test]
fn focused_container_is_positioned_outside_viewport_when_scrolled_past_items_with_different_heights() {
    let _app = app();

    let items: Vec<_> = (0..20).map(|x| ItemWithHeight::with_height(x, if x < 10 { 10.0 } else { 50.0 })).collect();

    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_height_template()));
    });

    let focused = content_presenter(t.target.container_from_index(5));
    focused.set_focusable(true);
    focused.focus();

    t.target.scroll_into_view(15);
    layout(&t.target);

    assert!(t.target.first_realized_index() > 5);

    let first_realized = content_presenter(t.target.container_from_index(t.target.first_realized_index()));
    let focused = content_presenter(t.target.container_from_index(5));

    // The focused container's position is estimated, as it's outside the
    // realized range. The estimate must never place it before the panel
    // origin...
    assert!(focused.bounds().top() >= 0.0);

    // ...must keep it above the realized range rather than overlapping it...
    assert!(focused.bounds().bottom() <= first_realized.bounds().top());

    // ...and must keep it out of the viewport, so it can't appear as a ghost
    // item.
    assert!(focused.bounds().bottom() <= t.scroll.offset().y);
}

#[allow(clippy::too_many_arguments)]
fn focused_container_is_positioned_correctly_when_container_size_change_causes_it_to_be_moved_into_visible_viewport(
    buffer_factor: f64,
    first_index_1: i32,
    last_index_1: i32,
    first_index_2: i32,
    last_index_2: i32,
    first_index_3: i32,
    last_index_3: i32,
) {
    let _app = app();

    // All containers start off with a height of 25 (4 containers fit in
    // viewport).
    let items: Vec<_> = (0..20).map(|x| ItemWithHeight::with_height(x, 25.0)).collect();
    let t = create_target(|o| {
        o.items = Some(models(&items));
        o.item_template = Some(Some(canvas_with_height_template()));
        o.buffer_factor = buffer_factor;
    });

    // Scroll to the 5th item (containers 4-7 should be visible).
    t.target.scroll_into_view(7);
    assert_eq!(first_index_1, t.target.first_realized_index());
    assert_eq!(last_index_1, t.target.last_realized_index());

    // Focus the 7th item.
    let container = content_presenter(t.target.container_from_index(7));
    container.set_focusable(true);
    container.focus();

    // Scroll up to the 3rd item (containers 3-6 should still be visible).
    t.target.scroll_into_view(3);
    assert_eq!(first_index_2, t.target.first_realized_index());
    assert_eq!(last_index_2, t.target.last_realized_index());

    // Update the height of all items to 20 and run a layout pass.
    for item in &items {
        item.set_height(20.0);
    }
    t.target.update_layout();

    // The focused container should now be inside the realized range.
    assert_eq!(first_index_3, t.target.first_realized_index());
    assert_eq!(last_index_3, t.target.last_realized_index());

    // The container should be positioned correctly.
    let container = content_presenter(t.target.container_from_index(7));
    assert_eq!(Rect::new(0.0, 140.0, 100.0, 20.0), container.bounds());
}

theory!(
    focused_container_is_positioned_correctly_when_container_size_change_causes_it_to_be_moved_into_visible_viewport:
    focused_container_is_positioned_correctly_when_container_size_change_causes_it_to_be_moved_into_visible_viewport_1(
        0.0, 4, 7, 3, 6, 3, 7,
    );
    focused_container_is_positioned_correctly_when_container_size_change_causes_it_to_be_moved_into_visible_viewport_2(
        0.5, 0, 7, 0, 7, 0, 9,
    ));

fn scroll_into_view_with_variable_size_items_keeps_target_in_viewport(target_index: i32, orientation: Orientation) {
    let _app = app();

    let first_half_size = if target_index < 60 { 20.0 } else { 40.0 };
    let second_half_size = if target_index < 60 { 40.0 } else { 20.0 };
    let horizontal = orientation == Orientation::Horizontal;
    let size = |x: i32| if x < 50 { first_half_size } else { second_half_size };
    let items = if horizontal {
        models(&(0..100).map(|x| ItemWithWidth::with_width(x, size(x))).collect::<Vec<_>>())
    } else {
        models(&(0..100).map(|x| ItemWithHeight::with_height(x, size(x))).collect::<Vec<_>>())
    };
    let item_template = if horizontal { canvas_with_width_template() } else { canvas_with_height_template() };
    let t = create_target(|o| {
        o.items = Some(items);
        o.item_template = Some(Some(item_template));
        o.orientation = orientation;
    });

    t.target.scroll_into_view(60);
    t.target.scroll_into_view(target_index);

    let container = content_presenter(t.target.container_from_index(target_index));
    let message = format!(
        "Bounds={:?}, Offset={:?}, Viewport={:?}, Extent={:?}",
        container.bounds(),
        t.scroll.offset(),
        t.scroll.viewport(),
        t.scroll.extent()
    );

    let container_start = if horizontal { container.bounds().left() } else { container.bounds().top() };
    let container_end = if horizontal { container.bounds().right() } else { container.bounds().bottom() };
    let viewport_start = if horizontal { t.scroll.offset().x } else { t.scroll.offset().y };
    let viewport_end =
        viewport_start + if horizontal { t.scroll.viewport().width } else { t.scroll.viewport().height };

    assert!(container_start > 0.0, "{message}");
    assert!(container_start >= viewport_start, "{message}");
    assert!(container_end <= viewport_end, "{message}");
}

theory!(scroll_into_view_with_variable_size_items_keeps_target_in_viewport:
    scroll_into_view_with_variable_size_items_keeps_target_in_viewport_1(25, Orientation::Vertical);
    scroll_into_view_with_variable_size_items_keeps_target_in_viewport_2(99, Orientation::Vertical);
    scroll_into_view_with_variable_size_items_keeps_target_in_viewport_3(25, Orientation::Horizontal);
    scroll_into_view_with_variable_size_items_keeps_target_in_viewport_4(99, Orientation::Horizontal));

fn create_counting_target(orientation: Orientation) -> (Target, Vec<Rc<ItemWithHeight>>, Vec<Rc<ItemWithWidth>>) {
    let horizontal = orientation == Orientation::Horizontal;
    let heights: Vec<_> = if horizontal { Vec::new() } else { (0..100).map(ItemWithHeight::new).collect() };
    let widths: Vec<_> = if horizontal { (0..100).map(ItemWithWidth::new).collect() } else { Vec::new() };
    let items = if horizontal { models(&widths) } else { models(&heights) };

    let t = create_target(|o| {
        o.items = Some(items);
        o.item_template =
            Some(Some(if horizontal { canvas_with_width_template() } else { canvas_with_height_template() }));
        o.orientation = orientation;
        o.panel = Some(CountingPanel::new().upcast());
        o.buffer_factor = 0.5;
    });

    (t, heights, widths)
}

#[test]
fn when_vertical_calculates_view_port_at_start_of_list() {
    // Arrange
    let _app = app();

    // Act
    let (t, _, _) = create_counting_target(Orientation::Vertical);

    // Assert
    assert_eq!(0.0, t.target.view_port().top());
    assert_eq!(100.0, t.target.view_port().bottom());

    assert_eq!(0.0, t.target.last_measured_extended_view_port().top());
    assert_eq!(200.0, t.target.last_measured_extended_view_port().bottom());
}

#[test]
fn when_vertical_calculates_view_port_at_end_of_list() {
    // Arrange
    let _app = app();

    let (t, _, _) = create_counting_target(Orientation::Vertical);

    // Act
    t.scroll.set_offset(Vector::new(0.0, 910.0)); // scroll to end
    layout(&t.target);

    // Assert
    assert_eq!(900.0, t.target.view_port().top());
    assert_eq!(1000.0, t.target.view_port().bottom());

    assert_eq!(800.0, t.target.last_measured_extended_view_port().top());
    assert_eq!(1000.0, t.target.last_measured_extended_view_port().bottom());
}

#[test]
fn when_vertical_calculates_view_port_in_middle_of_list() {
    // Arrange
    let _app = app();

    let (t, _, _) = create_counting_target(Orientation::Vertical);

    // Act
    t.scroll.set_offset(Vector::new(0.0, 500.0)); // scroll to end
    layout(&t.target);

    // Assert
    assert_eq!(500.0, t.target.view_port().top());
    assert_eq!(600.0, t.target.view_port().bottom());

    assert_eq!(450.0, t.target.last_measured_extended_view_port().top());
    assert_eq!(650.0, t.target.last_measured_extended_view_port().bottom());
}

#[test]
fn when_horizontal_calculates_view_port_at_start_of_list() {
    // Arrange
    let _app = app();

    // Act
    let (t, _, _) = create_counting_target(Orientation::Horizontal);

    // Assert
    assert_eq!(0.0, t.target.view_port().left());
    assert_eq!(100.0, t.target.view_port().right());

    assert_eq!(0.0, t.target.last_measured_extended_view_port().left());
    assert_eq!(200.0, t.target.last_measured_extended_view_port().right());
}

#[test]
fn when_horizontal_calculates_view_port_at_end_of_list() {
    // Arrange
    let _app = app();

    let (t, _, _) = create_counting_target(Orientation::Horizontal);

    // Act
    t.scroll.set_offset(Vector::new(900.0, 0.0)); // scroll to end
    layout(&t.target);

    // Assert
    assert_eq!(900.0, t.target.view_port().left());
    assert_eq!(1000.0, t.target.view_port().right());

    assert_eq!(800.0, t.target.last_measured_extended_view_port().left());
    assert_eq!(1000.0, t.target.last_measured_extended_view_port().right());
}

#[test]
fn when_horizontal_calculates_view_port_in_middle_of_list() {
    // Arrange
    let _app = app();

    let (t, _, _) = create_counting_target(Orientation::Horizontal);

    // Act
    t.scroll.set_offset(Vector::new(500.0, 0.0)); // scroll to end
    layout(&t.target);

    // Assert
    assert_eq!(500.0, t.target.view_port().left());
    assert_eq!(600.0, t.target.view_port().right());

    assert_eq!(450.0, t.target.last_measured_extended_view_port().left());
    assert_eq!(650.0, t.target.last_measured_extended_view_port().right());
}

/// The caption and the measure and arrange counts of the model items of the
/// panel's orientation.
fn counts(heights: &[Rc<ItemWithHeight>], widths: &[Rc<ItemWithWidth>]) -> Vec<(String, i32, i32)> {
    if widths.is_empty() {
        heights.iter().map(|x| (x.caption.clone(), x.measured.get(), x.arranged.get())).collect()
    } else {
        widths.iter().map(|x| (x.caption.clone(), x.measured.get(), x.arranged.get())).collect()
    }
}

/// The offset moved by `delta` along the panel's orientation.
fn scrolled(t: &Target, orientation: Orientation, delta: f64) -> Vector {
    if orientation == Orientation::Horizontal {
        Vector::new(t.scroll.offset().x + delta, 0.0)
    } else {
        Vector::new(0.0, t.scroll.offset().y + delta)
    }
}

fn along(orientation: Orientation, value: f64) -> Vector {
    if orientation == Orientation::Horizontal {
        Vector::new(value, 0.0)
    } else {
        Vector::new(0.0, value)
    }
}

fn scrolling_forward_does_not_measure_or_arrange_until_extended_view_port_bounds_are_reached(
    orientation: Orientation,
) {
    let _app = app();

    let (t, heights, widths) = create_counting_target(orientation);
    let panel = t.counting_panel();

    assert!(
        t.target.last_realized_index() == 19,
        "Should show 20 items but last realized index was {}",
        t.target.last_realized_index()
    );

    // Reset counters.
    panel.reset_measure_arrange_counters();
    // Shows 20 items, each is 10 high. Visible are 10 => need to scroll down
    // 100px until the next 5 (visible * buffer factor) additional items are
    // added. Until then no measure-arrange call should happen.

    let mut count = 0;
    // Scroll down until the extended viewport bounds are reached.
    while t.target.last_realized_index() < 20 {
        t.scroll.set_offset(scrolled(&t, orientation, 5.0));
        layout(&t.target);
        count += 1;
        if count > 1000 {
            panic!("infinite scroll detected");
        }
    }

    // Assert
    assert!(panel.measured.get() == 1, "should be measured only once");
    assert!(panel.arranged.get() == 1, "should be arranged only once");

    // The first 5 additional items will be reused when scrolling down, but
    // the remaining 10 visible + 5 additional not touched at all.
    let items = counts(&heights, &widths);
    for (caption, measured, arranged) in items.iter().skip(5 /* additional items */).take(15) {
        assert!(*measured == 0, "{caption} should not be measured but was {measured} times");
        assert!(*arranged == 0, "{caption} should not be arranged but was {arranged} times");
    }

    for (caption, measured, arranged) in items.iter().skip(20).take(5) {
        assert!(*measured == 1, "{caption} should be measured but was {measured} times");
        assert!(*arranged == 1, "{caption} should be measured but was {arranged} times");
    }
}

#[test]
fn scrolling_down_does_not_measure_or_arrange_until_extended_view_port_bounds_are_reached() {
    scrolling_forward_does_not_measure_or_arrange_until_extended_view_port_bounds_are_reached(Orientation::Vertical);
}

#[test]
fn scrolling_right_does_not_measure_or_arrange_until_extended_view_port_bounds_are_reached() {
    scrolling_forward_does_not_measure_or_arrange_until_extended_view_port_bounds_are_reached(
        Orientation::Horizontal,
    );
}

fn scrolling_back_does_not_measure_or_arrange_until_extended_view_port_bounds_are_reached(orientation: Orientation) {
    let _app = app();

    let (t, heights, widths) = create_counting_target(orientation);
    let panel = t.counting_panel();

    // Scroll a bit down so we are not near the start of the list.
    t.scroll.set_offset(along(orientation, 200.0));
    layout(&t.target);

    assert!(
        t.target.first_realized_index() == 15,
        "Should show items from 20 to 30 (so 15 to 35 including additional items) but first realized index was {}",
        t.target.first_realized_index()
    );

    // Reset counters.
    panel.reset_measure_arrange_counters();
    // Shows 20 items, each is 10 high. Visible are 10 => need to scroll down
    // 100px until the next 5 (visible * buffer factor) additional items are
    // added. Until then no measure-arrange call should happen.

    let initial_first_realized_index = t.target.first_realized_index() as usize;

    let mut count = 0;
    // Scroll down until the extended viewport bounds are reached.
    while t.target.first_realized_index() >= 15 {
        t.scroll.set_offset(scrolled(&t, orientation, -5.0));
        layout(&t.target);
        count += 1;
        if count > 1000 {
            panic!("infinite scroll detected");
        }
    }

    // Assert
    assert!(panel.measured.get() == 1, "should be measured only once");
    assert!(panel.arranged.get() == 1, "should be arranged only once");

    // The last 5 additional items will be reused when scrolling up, but the
    // remaining 10 visible + 5 additional not touched at all.
    let items = counts(&heights, &widths);
    for (caption, measured, arranged) in items.iter().skip(initial_first_realized_index + 1).take(15) {
        assert!(*measured == 0, "{caption} should not be measured but was {measured} times");
        assert!(*arranged == 0, "{caption} should not be arranged but was {arranged} times");
    }

    // Now that we scrolled up to index 19, items 18, 17, 16, 15 and 14 should
    // be the "additional" ones.
    for (caption, measured, arranged) in items.iter().skip(initial_first_realized_index - 6).take(6) {
        assert!(*measured == 1, "{caption} should be measured but was {measured} times");
        assert!(*arranged == 1, "{caption} should be measured but was {arranged} times");
    }
}

#[test]
fn scrolling_up_does_not_measure_or_arrange_until_extended_view_port_bounds_are_reached() {
    scrolling_back_does_not_measure_or_arrange_until_extended_view_port_bounds_are_reached(Orientation::Vertical);
}

#[test]
fn scrolling_left_does_not_measure_or_arrange_until_extended_view_port_bounds_are_reached() {
    scrolling_back_does_not_measure_or_arrange_until_extended_view_port_bounds_are_reached(Orientation::Horizontal);
}

fn scrolling_forward_to_end_of_list_only_measures_once_when_last_item_is_reached(orientation: Orientation) {
    let _app = app();

    let (t, heights, widths) = create_counting_target(orientation);
    let panel = t.counting_panel();

    // Scroll a bit down so we are near the end of the list.
    t.scroll.set_offset(along(orientation, 800.0)); // so we render 75 to 95 with a buffer size of 5
    layout(&t.target);

    assert!(
        t.target.last_realized_index() == 94,
        "Should show 20 items but last realized index was {}",
        t.target.last_realized_index()
    );

    // Reset counters.
    panel.reset_measure_arrange_counters();
    // Shows 20 items, each is 10 high. Visible are 10 => need to scroll down
    // 100px until the next 5 (visible * buffer factor) additional items are
    // added. Until then no measure-arrange call should happen.

    let initial_last_realized_index = t.target.last_realized_index() as usize;

    let mut count = 0;
    // Scroll down until we reached the very last item.
    while t.target.last_realized_index() < 99 {
        t.scroll.set_offset(scrolled(&t, orientation, 5.0));
        layout(&t.target);
        count += 1;
        if count > 1000 {
            panic!("infinite scroll detected");
        }
    }

    // Assert
    assert!(panel.measured.get() == 1, "should be measured only once even though we are at the end of the list");
    assert!(panel.arranged.get() == 1, "should be arranged only once even though we are at the end of the list");

    // The first 5 additional items will be reused when scrolling down, but
    // the remaining 10 visible + 5 additional not touched at all.
    let items = counts(&heights, &widths);
    for (caption, measured, arranged) in items.iter().skip(initial_last_realized_index + 1 - 15).take(15) {
        assert!(*measured == 0, "{caption} should not be measured but was {measured} times");
        assert!(*arranged == 0, "{caption} should not be arranged but was {arranged} times");
    }

    for (caption, measured, arranged) in items.iter().skip(initial_last_realized_index + 1).take(5) {
        assert!(*measured == 1, "{caption} should be measured but was {measured} times");
        assert!(*arranged == 1, "{caption} should be measured but was {arranged} times");
    }
}

#[test]
fn scrolling_down_to_end_of_list_only_measures_once_when_last_item_is_reached() {
    scrolling_forward_to_end_of_list_only_measures_once_when_last_item_is_reached(Orientation::Vertical);
}

#[test]
fn scrolling_right_to_end_of_list_only_measures_once_when_last_item_is_reached() {
    scrolling_forward_to_end_of_list_only_measures_once_when_last_item_is_reached(Orientation::Horizontal);
}

fn scrolling_back_to_start_of_list_only_measures_once_when_first_item_is_reached(orientation: Orientation) {
    let _app = app();

    let (t, heights, widths) = create_counting_target(orientation);
    let panel = t.counting_panel();

    // Scroll a bit down so we are not near the start of the list.
    t.scroll.set_offset(along(orientation, 105.0));
    layout(&t.target);

    assert!(
        t.target.first_realized_index() == 5,
        "Should show items from 10 to 20 (so 5 to 25 including additional items) but first realized index was {}",
        t.target.first_realized_index()
    );

    // Reset counters.
    panel.reset_measure_arrange_counters();
    // Shows 20 items, each is 10 high. Visible are 10 => need to scroll down
    // 100px until the next 5 (visible * buffer factor) additional items are
    // added. Until then no measure-arrange call should happen.

    let mut count = 0;
    // Scroll down until the extended viewport bounds are reached.
    while t.target.first_realized_index() > 0 {
        t.scroll.set_offset(scrolled(&t, orientation, -5.0));
        layout(&t.target);
        count += 1;
        if count > 1000 {
            panic!("infinite scroll detected");
        }
    }

    // Assert
    assert!(panel.measured.get() == 1, "should be measured only once even though we are at the start of the list");
    assert!(panel.arranged.get() == 1, "should be arranged only once even though we are at the start of the list");

    // The last 5 additional items will be reused when scrolling up, but the
    // remaining 10 visible + 5 additional not touched at all.
    let items = counts(&heights, &widths);
    for (caption, measured, arranged) in items.iter().take(20) {
        assert!(*measured == 1, "{caption} should be measured but was {measured} times");
        assert!(*arranged == 1, "{caption} should be arranged but was {arranged} times");
    }

    // Now that we scrolled up to index 19, items 18, 17, 16, 15 and 14 should
    // be the "additional" ones.
    for (caption, measured, arranged) in items.iter().skip(20) {
        assert!(*measured == 0, "{caption} should not be measured but was {measured} times");
        assert!(*arranged == 0, "{caption} should not be measured but was {arranged} times");
    }
}

#[test]
fn scrolling_up_to_start_of_list_only_measures_once_when_first_item_is_reached() {
    scrolling_back_to_start_of_list_only_measures_once_when_first_item_is_reached(Orientation::Vertical);
}

#[test]
fn scrolling_left_to_start_of_list_only_measures_once_when_first_item_is_reached() {
    scrolling_back_to_start_of_list_only_measures_once_when_first_item_is_reached(Orientation::Horizontal);
}

// ---------------------------------------------------------------------------
// Items control tests that need a virtualizing panel
// ---------------------------------------------------------------------------

fn theme_resource(theme: Ref<ControlTheme>) -> Option<BoxedValue> {
    Some(Rc::new(theme))
}

fn create_content_control_theme() -> Ref<ControlTheme> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<ContentControl>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        for property in [
            ContentControl::content_property().as_property(),
            ContentControl::content_template_property().as_property(),
        ] {
            presenter.bind_binding(property, &TemplateBinding::new(property));
        }
        presenter.register_in_name_scope(&**scope).upcast()
    });
    ControlTheme::with_setters(ContentControl::TYPE, [Setter::new(TemplatedControl::template_property(), Some(template))])
}

fn create_items_control_theme() -> Ref<ControlTheme> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<ItemsControl>(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));
        let border = Border::new();
        border.set_background(Some(SolidColorBrush::from_uint32(0xffffffff).into()));
        border.set_child(presenter.register_in_name_scope(&**scope));
        border.upcast()
    });
    ControlTheme::with_setters(ItemsControl::TYPE, [Setter::new(TemplatedControl::template_property(), Some(template))])
}

fn create_scroll_viewer_theme() -> Ref<ControlTheme> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));

        let scroll_bar = ScrollBar::new();
        scroll_bar.set_name(Some("verticalScrollBar".to_string()));

        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.children().add(scroll_bar);
        panel.upcast()
    });
    ControlTheme::with_setters(ScrollViewer::TYPE, [Setter::new(TemplatedControl::template_property(), Some(template))])
}

fn create_themed_root(child: Ref<Control>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.resources().add(ScrollViewer::TYPE, theme_resource(create_scroll_viewer_theme()));
    root.resources().add(ContentControl::TYPE, theme_resource(create_content_control_theme()));
    root.resources().add(ItemsControl::TYPE, theme_resource(create_items_control_theme()));
    root.set_child(child);
    root
}

fn virtualizing_items_panel() -> Rc<dyn ITemplateOf<Option<Ref<Panel>>>> {
    FuncTemplate::new(|| Some(VirtualizingStackPanel::new().upcast::<Panel>()))
}

fn background_theme(background: Rc<dyn IBrush>) -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        ContentPresenter::TYPE,
        [Setter::new(ContentPresenter::background_property(), brush(background))],
    )
}

#[test]
fn item_container_theme_can_be_changed_virtualizing() {
    let _app = app();

    let theme1 = background_theme(Brushes::red());
    let theme2 = background_theme(Brushes::green());

    let target = ItemsControl::new();
    target.set_item_container_theme(Some(theme1.clone()));
    target.set_items_source(Some(ItemsSource::from_strs(["Foo"])));
    target.set_items_panel(virtualizing_items_panel());
    let root = create_themed_root(target.clone().upcast());
    root.execute_initial_layout_pass();

    let container = content_presenter(target.get_realized_containers().first().cloned());

    assert_eq!(container.theme(), Some(theme1));
    assert_eq!(container.background(), brush(Brushes::red()));

    target.set_item_container_theme(Some(theme2.clone()));
    layout(&target);

    let container = content_presenter(target.get_realized_containers().first().cloned());
    assert_eq!(container.theme(), Some(theme2));
    assert_eq!(container.background(), brush(Brushes::green()));
}

#[test]
fn control_item_should_be_removed_from_logical_children_virtualizing() {
    let _app = app();
    let item: Ref<Control> = Border::new().upcast();

    let items = Rc::new(FerroList::<Ref<Control>>::new());
    let target = ItemsControl::new();
    target.set_items_source(Some(items.clone().into()));
    target.set_items_panel(virtualizing_items_panel());
    let root = create_themed_root(target.clone().upcast());
    root.execute_initial_layout_pass();

    items.add(item.clone());
    layout(&target);

    items.remove(&item);

    assert!(target.logical_children().is_empty());
}

#[test]
fn handles_recycling_control_items_inside_containers() {
    // Issue #10825
    let _app = app();

    // The items must be controls but not of the container type.
    let items: Vec<Ref<Control>> = (0..100)
        .map(|x| {
            let text_block = TextBlock::new();
            text_block.set_text(Some(&format!("Item {x}")));
            text_block.set_width(100.0);
            text_block.set_height(100.0);
            text_block.upcast()
        })
        .collect();

    // Virtualization is required.
    let items_panel = virtualizing_items_panel();

    // Create an items control which uses containers, and provide a scroll
    // host.
    let target: Ref<ItemsControl> = instantiate(ItemsControlWithContainer { base: ItemsControl::construct() }).upcast();
    for item in &items {
        target.items().add(Some(Control::boxed(item.clone())));
    }
    target.set_items_panel(items_panel);

    let scroll = ScrollViewer::new();
    scroll.set_content(Some(Control::boxed(target.clone())));
    let root = create_themed_root(scroll.clone().upcast());
    root.execute_initial_layout_pass();

    assert_eq!(10, target.get_realized_containers().len());

    // Scroll so that half a container is visible: an extra container is
    // generated.
    scroll.set_offset(Vector::new(0.0, 2050.0));
    layout(&target);

    // Scroll so that the extra container is no longer needed and recycled.
    scroll.set_offset(Vector::new(0.0, 2100.0));
    layout(&target);

    // Scroll back: issue #10825 triggered.
    scroll.set_offset(Vector::new(0.0, 2000.0));
    layout(&target);
}
