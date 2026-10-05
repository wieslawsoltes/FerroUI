use crate::generators::RecycleKey;
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{TemplatedControl, TemplatedControlImpl};
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
    IDataTemplate, ITemplateOf,
};
use crate::test_support::{boxed_str, test_scope, TestRoot, TestScope};
use crate::{
    box_item, items_equal, AssignedBinding, Border, Button, Canvas, ContentControl, Control, ControlImpl, ItemsControl,
    ItemsControlImpl, ItemsSource, ListBoxItem, Panel, TextBlock,
};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::controls::NameScope;
use ferroui_base::data::core::plugins::PropertyInfoAccessorFactory;
use ferroui_base::data::core::{ClrPropertyInfo, Maybe, Value};
use ferroui_base::data::model::Model;
use ferroui_base::data::{BindingBase, CompiledBinding, CompiledBindingPathBuilder, ReflectionBinding, TemplateBinding};
use ferroui_base::input::{IKeyboardDevice, InputElement, InputElementImpl, Key, KeyEventArgs, KeyboardDevice};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, IBrush, SolidColorBrush};
use ferroui_base::styling::{ControlTheme, Setter};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_model, instantiate, BoxedValue, FerroLocator, FerroObject, FerroObjectImpl, ObjectType,
    Ref, StyledElement, StyledElementImpl, TypeInfo, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// A model item.
struct Item {
    caption: String,
    value: Option<String>,
}

ferro_model!(Item, |b| b
    .read_only::<Value<String>>("Caption", |item| item.caption.clone())
    .read_only::<Maybe<String>>("Value", |item| item.value.clone()));

impl Item {
    fn new(caption: &str, value: Option<&str>) -> Rc<Item> {
        Model::new_model(Item { caption: caption.to_string(), value: value.map(str::to_string) })
    }
}

#[derive(Default)]
struct Options {
    data_context: Option<BoxedValue>,
    items: Option<Vec<Option<BoxedValue>>>,
    items_source: Option<ItemsSource>,
    item_container_theme: Option<Ref<ControlTheme>>,
    item_template: Option<Rc<dyn IDataTemplate>>,
    items_panel: Option<Rc<dyn ITemplateOf<Option<Ref<Panel>>>>>,
    no_layout: bool,
}

struct Target {
    _scope: TestScope,
    root: Ref<TestRoot>,
    target: Ref<ItemsControl>,
}

impl Deref for Target {
    type Target = Ref<ItemsControl>;

    fn deref(&self) -> &Ref<ItemsControl> {
        &self.target
    }
}

fn start() -> TestScope {
    let scope = test_scope();
    FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(KeyboardDevice::new());
    scope
}

fn create_target(configure: impl FnOnce(&mut Options)) -> Target {
    create_target_of(ItemsControl::new(), configure)
}

fn create_target_of(target: Ref<ItemsControl>, configure: impl FnOnce(&mut Options)) -> Target {
    create_target_in(start(), target, configure)
}

fn create_target_in(scope: TestScope, target: Ref<ItemsControl>, configure: impl FnOnce(&mut Options)) -> Target {
    let mut options = Options::default();
    configure(&mut options);

    target.set_data_context(options.data_context);
    target.set_item_container_theme(options.item_container_theme);
    target.set_item_template(options.item_template);
    target.set_items_source(options.items_source);

    if let Some(items) = options.items {
        for item in items {
            target.items().add(item);
        }
    }

    if let Some(items_panel) = options.items_panel {
        target.set_items_panel(items_panel);
    }

    let root = create_root(&target);

    if !options.no_layout {
        root.execute_initial_layout_pass();
    }

    Target { _scope: scope, root, target }
}

fn create_root(child: &Ref<ItemsControl>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.resources().add(ContentControl::TYPE, theme_resource(create_content_control_theme()));
    root.resources().add(ItemsControl::TYPE, theme_resource(create_items_control_theme()));
    root.set_child(child.clone());
    root
}

fn theme_resource(theme: Ref<ControlTheme>) -> Option<BoxedValue> {
    Some(Rc::new(theme))
}

fn create_content_control_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        ContentControl::TYPE,
        [Setter::new(TemplatedControl::template_property(), Some(create_content_control_template()))],
    )
}

fn create_content_control_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ContentControl>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        for property in [
            ContentControl::content_property().as_property(),
            ContentControl::content_template_property().as_property(),
        ] {
            presenter.bind_binding(property, &TemplateBinding::new(property));
        }
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

fn create_items_control_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        ItemsControl::TYPE,
        [Setter::new(TemplatedControl::template_property(), Some(create_items_control_template()))],
    )
}

fn create_items_control_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ItemsControl>(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));
        let border = Border::new();
        border.set_background(Some(SolidColorBrush::from_uint32(0xffffffff).into()));
        border.set_child(presenter.register_in_name_scope(&**scope));
        border.upcast()
    })
}

fn layout(target: &Target) {
    target.root.layout_manager().execute_layout_pass();
}

fn get_container(target: &ItemsControl, index: usize) -> Ref<ContentPresenter> {
    get_container_of::<ContentPresenter>(target, index)
}

fn get_container_of<T: ObjectType>(target: &ItemsControl, index: usize) -> Ref<T> {
    let container = target.get_realized_containers()[index].clone();
    assert_eq!(container.get_type(), T::TYPE);
    container.cast::<T>().unwrap()
}

fn strs(values: &[&str]) -> Option<ItemsSource> {
    Some(ItemsSource::from_strs(values.iter().copied()))
}

fn ints(values: &[i32]) -> Option<ItemsSource> {
    Some(ItemsSource::from_values(values.iter().copied()))
}

fn boxed_items<T: ferroui_base::PropertyValue>(values: &[T]) -> Option<Vec<Option<BoxedValue>>> {
    Some(values.iter().map(box_item).collect())
}

fn controls(values: &[&Ref<Control>]) -> Option<Vec<Option<BoxedValue>>> {
    Some(values.iter().map(|c| Some(Control::boxed((*c).clone()))).collect())
}

fn logical(children: &[&Ref<Control>]) -> Vec<Ref<StyledElement>> {
    children.iter().map(|c| (*c).clone().upcast()).collect()
}

fn as_object(target: &ItemsControl) -> Option<Ref<FerroObject>> {
    Some(target.to_ref().upcast())
}

fn brush(brush: Rc<dyn IBrush>) -> Option<Rc<dyn IBrush>> {
    Some(brush)
}

fn background_theme(target_type: &'static TypeInfo, background: Rc<dyn IBrush>) -> Ref<ControlTheme> {
    ControlTheme::with_setters(target_type, [Setter::new(ContentPresenter::background_property(), brush(background))])
}

fn track_logical_children(
    target: &ItemsControl,
    predicate: impl Fn(NotifyCollectionChangedAction) -> bool + 'static,
) -> Rc<Cell<bool>> {
    let called = Rc::new(Cell::new(false));
    let result = called.clone();
    StyledElement::logical_children(target).add_collection_changed(Rc::new(
        move |e: &NotifyCollectionChangedEventArgs<'_, Ref<StyledElement>>| called.set(predicate(e.action)),
    ));
    result
}

fn button() -> Ref<Control> {
    Button::new().upcast()
}

fn raise_key_down(target: &ItemsControl, key: Key) {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = key;
    target.raise_event(&args);
}

/// Whether an untyped value is an [`Item`].
fn is_item(value: &BoxedValue) -> bool {
    let value: &dyn ferroui_base::AnyValue = &**value;
    value.is::<Item>()
}

fn canvas_template() -> Option<Rc<dyn IDataTemplate>> {
    Some(FuncDataTemplate::for_type::<String>(|_, _| Some(Canvas::new().upcast()), false))
}

#[test]
fn setting_items_source_should_populate_items() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));

    let source = target.items_source().unwrap();
    assert!(!source.ptr_eq(&ItemsSource::from_items(target.items().to_vec())));
    let expected = source.to_vec();
    let actual = target.items().to_vec();
    assert_eq!(expected.len(), actual.len());
    assert!(expected.iter().zip(actual.iter()).all(|(a, b)| items_equal(a, b)));
}

#[test]
#[should_panic(expected = "Items collection must be empty before using ItemsSource.")]
fn cannot_set_items_source_with_items_present() {
    let target = create_target(|_| {});
    target.items().add(boxed_str("foo"));

    target.set_items_source(strs(&["baz"]));
}

#[test]
#[should_panic(expected = "Operation is not valid while ItemsSource is in use.")]
fn cannot_modify_items_when_items_source_set() {
    let target = create_target(|o| o.items_source = strs(&[]));

    target.items().add(boxed_str("foo"));
}

#[test]
fn should_use_item_template_to_create_control() {
    let target = create_target(|o| {
        o.items_source = strs(&["Foo"]);
        o.item_template = canvas_template();
    });
    let container = get_container(&target, 0);

    assert!(container.child().unwrap().is::<Canvas>());
}

#[test]
fn item_template_can_be_changed() {
    let target = create_target(|o| {
        o.items_source = strs(&["Foo"]);
        o.item_template = canvas_template();
    });
    let container = get_container(&target, 0);

    assert!(container.child().unwrap().is::<Canvas>());

    target.set_item_template(Some(FuncDataTemplate::for_type::<String>(
        |_, _| Some(Border::new().upcast()),
        false,
    )));
    layout(&target);

    let container = get_container(&target, 0);

    assert!(container.child().unwrap().is::<Border>());
}

#[test]
fn panel_should_have_templated_parent_set_to_items_control() {
    let target = create_target(|o| o.items_source = strs(&["Foo"]));

    assert_eq!(as_object(&target), target.items_panel_root().unwrap().templated_parent());
}

#[test]
fn panel_should_have_items_host_set_to_true() {
    let target = create_target(|o| o.items_source = strs(&["Foo"]));

    assert!(target.items_panel_root().unwrap().is_items_host());
}

#[test]
fn container_should_have_templated_parent_set_to_null() {
    let target = create_target(|o| o.items_source = strs(&["Foo"]));

    let container = get_container(&target, 0);

    assert!(container.templated_parent().is_none());
}

#[test]
fn container_should_have_theme_set_to_item_container_theme() {
    let theme = ControlTheme::with_target_type(ContentPresenter::TYPE);
    let target = create_target(|o| {
        o.items_source = strs(&["Foo"]);
        o.item_container_theme = Some(theme.clone());
    });

    let container = get_container(&target, 0);

    assert_eq!(container.theme(), Some(theme));
}

#[test]
fn container_should_have_theme_set_to_item_container_theme_with_base_target_type() {
    let theme = ControlTheme::with_target_type(Control::TYPE);
    let target = create_target(|o| {
        o.items_source = strs(&["Foo"]);
        o.item_container_theme = Some(theme.clone());
    });

    let container = get_container(&target, 0);

    assert_eq!(container.theme(), Some(theme));
}

#[test]
fn item_container_theme_can_be_changed() {
    let theme1 = background_theme(ContentPresenter::TYPE, Brushes::red());
    let theme2 = background_theme(ContentPresenter::TYPE, Brushes::green());

    let target = create_target(|o| {
        o.items_source = strs(&["Foo"]);
        o.item_container_theme = Some(theme1.clone());
    });

    let container = get_container(&target, 0);

    assert_eq!(container.theme(), Some(theme1));
    assert_eq!(container.background(), brush(Brushes::red()));

    target.set_item_container_theme(Some(theme2.clone()));

    let container = get_container(&target, 0);
    assert_eq!(container.theme(), Some(theme2));
    assert_eq!(container.background(), brush(Brushes::green()));
}

#[test]
fn item_container_theme_can_be_cleared() {
    let theme = background_theme(ContentPresenter::TYPE, Brushes::red());

    let target = create_target(|o| {
        o.items_source = strs(&["Foo"]);
        o.item_container_theme = Some(theme.clone());
    });

    let container = get_container(&target, 0);

    assert_eq!(container.theme(), Some(theme));
    assert_eq!(container.background(), brush(Brushes::red()));

    target.set_item_container_theme(None);

    let container = get_container(&target, 0);
    assert!(container.theme().is_none());
    assert!(container.background().is_none());
}

#[test]
fn item_container_theme_should_not_override_local_value_theme() {
    let theme1 = background_theme(ContentPresenter::TYPE, Brushes::red());
    let theme2 = background_theme(Control::TYPE, Brushes::green());

    let first = ContentPresenter::new();
    let second = ContentPresenter::new();
    second.set_theme(theme2.clone());

    let items = ItemsSource::from_items([Some(Control::boxed(first)), Some(Control::boxed(second))]);

    let target = create_target(|o| {
        o.items_source = Some(items);
        o.item_container_theme = Some(theme1.clone());
    });

    assert_eq!(Some(theme1), get_container(&target, 0).theme());
    assert_eq!(Some(theme2.clone()), get_container(&target, 1).theme());

    target.set_item_container_theme(None);

    assert!(get_container(&target, 0).theme().is_none());
    assert_eq!(Some(theme2), get_container(&target, 1).theme());
}

#[test]
fn container_should_have_logical_parent_set_to_items_control() {
    let _scope = start();
    let target = ItemsControl::new();
    let root = create_root(&target);
    let templated_parent = Button::new();

    target.set_templated_parent(templated_parent.upcast::<FerroObject>());
    target.set_template(Some(create_items_control_template()));
    target.set_items_source(strs(&["Foo"]));

    root.execute_initial_layout_pass();

    let container = get_container(&target, 0);

    assert_eq!(container.parent(), Some(target.clone().upcast()));
}

#[test]
fn control_item_should_be_logical_child_before_apply_template() {
    let child = Control::new();
    let target = create_target(|o| {
        o.items = controls(&[&child]);
        o.no_layout = true;
    });

    assert!(!target.is_measure_valid());
    assert_eq!(target.visual_children().count(), 0);
    assert_eq!(child.parent(), Some(target.clone().upcast()));
    assert_eq!(target.logical_children().to_vec(), logical(&[&child]));
}

#[test]
fn control_item_should_be_logical_child_after_layout() {
    let child = Control::new();
    let target = create_target(|o| o.items = controls(&[&child]));

    assert!(target.is_measure_valid());
    assert_eq!(target.visual_children().count(), 1);
    assert_eq!(child.parent(), Some(target.clone().upcast()));
    assert_eq!(target.logical_children().to_vec(), logical(&[&child]));
}

#[test]
fn added_container_should_have_logical_parent_set_to_items_control() {
    let items = Rc::new(FerroList::<Ref<Control>>::new());
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    let item: Ref<Control> = Border::new().upcast();
    items.add(item.clone());

    assert_eq!(item.parent(), Some(target.clone().upcast()));
}

#[test]
fn control_item_can_be_removed_from_logical_children_before_apply_template() {
    let child = Control::new();
    let target = create_target(|o| {
        o.items = controls(&[&child]);
        o.no_layout = true;
    });

    assert!(!target.is_measure_valid());
    assert_eq!(target.visual_children().count(), 0);
    assert_eq!(target.logical_children().count(), 1);

    target.items().remove_at(0);

    assert!(child.parent().is_none());
    assert!(target.logical_children().is_empty());
}

#[test]
fn clearing_items_should_clear_child_controls_parent_before_apply_template() {
    let child = Control::new();
    let target = create_target(|o| {
        o.items = controls(&[&child]);
        o.no_layout = true;
    });

    assert!(!target.is_measure_valid());
    assert_eq!(target.visual_children().count(), 0);
    assert_eq!(target.logical_children().count(), 1);

    target.items().clear();

    assert!(child.parent().is_none());
}

#[test]
fn assigning_items_source_should_not_fire_logical_children_collection_changed_before_apply_template() {
    let child = Control::new();
    let target = create_target(|o| {
        o.items_source = Some(ItemsSource::from_items([Some(Control::boxed(child.clone()))]));
        o.no_layout = true;
    });
    let called = track_logical_children(&target, |_| true);

    let list = Rc::new(FerroList::from_items([child]));
    target.set_items_source(Some(list.into()));

    assert!(!called.get());
}

#[test]
fn removing_items_source_items_should_not_fire_logical_children_collection_changed_before_apply_template() {
    let items = Rc::new(FerroList::from_items(["Foo".to_string(), "Bar".to_string()]));
    let target = create_target(|o| {
        o.items_source = Some(items.clone().into());
        o.no_layout = true;
    });
    let called = track_logical_children(&target, |_| true);

    items.remove(&"Bar".to_string());

    assert!(!called.get());
}

#[test]
fn changing_items_source_should_not_fire_logical_children_collection_changed_before_apply_template() {
    let child = Control::new();
    let target = create_target(|o| {
        o.items_source = Some(ItemsSource::from_items([Some(Control::boxed(child.clone()))]));
        o.no_layout = true;
    });
    let called = track_logical_children(&target, |_| true);

    let list = Rc::new(FerroList::<Ref<Control>>::new());
    target.set_items_source(Some(list.clone().into()));
    list.add(child);

    assert!(!called.get());
}

#[test]
fn clearing_items_should_clear_child_controls_parent() {
    let child = Control::new();
    let target = create_target(|o| o.items = controls(&[&child]));

    target.items().clear();

    assert!(child.parent().is_none());
}

#[test]
fn adding_control_item_should_make_control_appear_in_logical_children() {
    let child = Control::new();
    let target = create_target(|o| {
        o.items = controls(&[&child]);
        o.no_layout = true;
    });

    // Should appear both before and after applying template.
    assert_eq!(target.logical_children().to_vec(), logical(&[&child]));

    layout(&target);

    assert_eq!(target.logical_children().to_vec(), logical(&[&child]));
}

#[test]
fn adding_string_item_should_make_content_presenter_appear_in_logical_children() {
    let target = create_target(|o| o.items_source = strs(&["Foo "]));

    assert_eq!(target.logical_children().count(), 1);
    assert!(target.logical_children().get(0).is::<ContentPresenter>());
}

#[test]
fn adding_items_should_fire_logical_children_collection_changed() {
    let target = create_target(|_| {});

    target.set_template(Some(create_items_control_template()));
    target.apply_template();

    let called = track_logical_children(&target, |action| action == NotifyCollectionChangedAction::Add);

    let child = Control::new();
    target.items().add(Some(Control::boxed(child)));

    assert!(called.get());
}

#[test]
fn clearing_items_should_fire_logical_children_collection_changed() {
    let child = Control::new();
    let target = create_target(|o| o.items = controls(&[&child]));

    let called = track_logical_children(&target, |action| action == NotifyCollectionChangedAction::Remove);

    target.items().clear();

    assert!(called.get());
}

#[test]
fn logical_children_should_not_change_instance_when_template_changed() {
    let target = create_target(|_| {});
    let before = StyledElement::logical_children(&target) as *const _;

    target.set_template(None);
    target.set_template(Some(create_items_control_template()));
    layout(&target);

    let after = StyledElement::logical_children(&target) as *const _;

    assert!(std::ptr::eq(before, after));
}

#[test]
fn control_item_should_be_removed_from_logical_children() {
    let item: Ref<Control> = Border::new().upcast();

    let items = Rc::new(FerroList::<Ref<Control>>::new());
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    items.add(item.clone());
    items.remove(&item);

    assert!(target.logical_children().is_empty());
}

#[test]
fn should_clear_containers_when_items_presenter_changes() {
    let target = create_target(|o| o.items_source = strs(&["foo", "bar"]));
    let panel = target.presenter().unwrap().panel().unwrap();

    assert_eq!(panel.children().count(), 2);

    target.set_template(Some(create_items_control_template()));
    target.apply_template();

    assert!(panel.children().is_empty());
}

#[test]
fn empty_class_should_initially_be_applied() {
    let target = create_target(|o| o.no_layout = true);

    assert!(target.classes().contains(":empty"));
}

#[test]
fn empty_class_should_be_cleared_when_items_added() {
    let target = create_target(|o| {
        o.items = boxed_items(&[1, 2, 3]);
        o.no_layout = true;
    });

    assert!(!target.classes().contains(":empty"));
}

#[test]
fn empty_class_should_be_cleared_when_items_source_items_added() {
    let target = create_target(|o| {
        o.items_source = ints(&[1, 2, 3]);
        o.no_layout = true;
    });

    assert!(!target.classes().contains(":empty"));
}

#[test]
fn empty_class_should_be_set_when_items_source_collection_cleared() {
    let target = create_target(|o| o.items_source = ints(&[1, 2, 3]));

    target.set_items_source(ints(&[]));

    assert!(target.classes().contains(":empty"));
}

#[test]
fn item_count_should_be_set_when_items_source_set() {
    let target = create_target(|o| o.items_source = ints(&[1, 2, 3]));

    assert_eq!(target.item_count(), 3);
}

#[test]
fn item_count_should_be_set_when_items_changed() {
    let target = create_target(|o| o.items = boxed_items(&[1, 2, 3]));

    target.items().add(box_item(&4));

    assert_eq!(target.item_count(), 4);

    target.items().clear();

    assert_eq!(target.item_count(), 0);
}

fn int_list(values: &[i32]) -> Rc<FerroList<i32>> {
    Rc::new(FerroList::from_items(values.iter().copied()))
}

#[test]
fn item_count_should_be_set_when_items_source_items_changed() {
    let items = int_list(&[1, 2, 3]);
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    items.add(4);

    assert_eq!(target.item_count(), 4);

    items.clear();

    assert_eq!(target.item_count(), 0);
}

#[test]
fn empty_class_should_be_set_when_items_collection_cleared() {
    let items = int_list(&[1, 2, 3]);
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    items.clear();

    assert!(target.classes().contains(":empty"));
}

#[test]
fn empty_class_should_not_be_set_when_items_source_collection_count_increases() {
    let items = int_list(&[]);
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    items.add(1);

    assert!(!target.classes().contains(":empty"));
}

#[test]
fn single_item_class_should_be_set_when_items_source_collection_count_increases_to_one() {
    let items = int_list(&[]);
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    items.add(1);

    assert!(target.classes().contains(":singleitem"));
}

#[test]
fn empty_class_should_not_be_set_when_items_source_collection_cleared() {
    let items = int_list(&[1, 2, 3]);
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    items.clear();

    assert!(!target.classes().contains(":singleitem"));
}

#[test]
fn single_item_class_should_not_be_set_when_items_collection_count_increases_beyond_one() {
    let items = int_list(&[1]);
    let target = create_target(|o| o.items_source = Some(items.clone().into()));

    items.add(2);

    assert!(!target.classes().contains(":singleitem"));
}

#[test]
fn data_contexts_should_be_correctly_set() {
    let text_block = TextBlock::new();
    text_block.set_text(Some("Baz"));
    let list_box_item = ListBoxItem::new();
    list_box_item.set_content(boxed_str("Qux"));

    let items: Vec<Option<BoxedValue>> = vec![
        boxed_str("Foo"),
        Some(Item::new("Bar", None)),
        Some(Control::boxed(text_block)),
        Some(Control::boxed(list_box_item)),
    ];
    let data_template: Rc<dyn IDataTemplate> = FuncDataTemplate::new(
        |data| data.is_some_and(is_item),
        |x, _| {
            let button = Button::new();
            button.set_content(x.clone());
            Some(button.upcast())
        },
        false,
    );
    let target = create_target(|o| {
        o.data_context = boxed_str("Base");
        o.items_source = Some(ItemsSource::from_items(items.clone()));
        o.item_template = Some(data_template);
    });
    let panel = target.items_panel_root().unwrap();
    let data_contexts: Vec<Option<BoxedValue>> = panel
        .children()
        .to_vec()
        .iter()
        .map(|x| {
            if let Some(presenter) = x.downcast_ref::<ContentPresenter>() {
                presenter.update_child();
            }
            x.data_context()
        })
        .collect();

    let expected = [items[0].clone(), items[1].clone(), boxed_str("Base"), boxed_str("Base")];
    assert_eq!(data_contexts.len(), expected.len());
    for (actual, expected) in data_contexts.iter().zip(expected.iter()) {
        assert!(items_equal(actual, expected));
    }
}

#[test]
fn control_item_should_not_be_name_scope() {
    let text_block: Ref<Control> = TextBlock::new().upcast();
    let target = create_target(|o| o.items_source = Some(ItemsSource::from_items([Some(Control::boxed(text_block))])));
    let item = target.logical_children().get(0);

    assert!(NameScope::get_name_scope(&item).is_none());
}

#[test]
fn focuses_next_item_on_key_down() {
    let items = ItemsSource::from_items([Some(Control::boxed(button())), Some(Control::boxed(button()))]);

    let target = create_target(|o| o.items_source = Some(items));
    get_container_of::<Button>(&target, 0).focus();

    raise_key_down(&target, Key::Down);

    let panel = target.items_panel_root().unwrap();
    let focus_manager = ferroui_base::input::FocusManager::get_focus_manager(&target).unwrap();

    assert_eq!(Some(panel.children().get(1).upcast()), focus_manager.get_focused_element());
}

#[test]
fn does_not_focus_non_focusable_item_on_key_down() {
    let non_focusable = button();
    non_focusable.set_focusable(false);
    let items = ItemsSource::from_items([
        Some(Control::boxed(button())),
        Some(Control::boxed(non_focusable)),
        Some(Control::boxed(button())),
    ]);

    let target = create_target(|o| o.items_source = Some(items));
    get_container_of::<Button>(&target, 0).focus();

    raise_key_down(&target, Key::Down);

    let panel = target.items_panel_root().unwrap();
    let focus_manager = ferroui_base::input::FocusManager::get_focus_manager(&target).unwrap();

    assert_eq!(Some(panel.children().get(2).upcast()), focus_manager.get_focused_element());
}

#[test]
fn detaching_then_reattaching_to_logical_tree_twice_does_not_throw() {
    // Issue 3487
    let target = create_target(|o| {
        o.items_source = strs(&["foo", "bar"]);
        o.item_template = canvas_template();
    });

    let root = target.root.clone();

    root.set_child(None::<Ref<Control>>);
    root.set_child(target.target.clone());

    root.layout_manager().execute_layout_pass();

    root.set_child(None::<Ref<Control>>);
    root.set_child(target.target.clone());
}

fn binding(path: &str) -> Option<AssignedBinding> {
    let binding: Rc<dyn BindingBase> = ReflectionBinding::new(path);
    Some(AssignedBinding::new(binding))
}

/// A binding to the length of a string. Strings have no properties that a
/// binding path could name, so the path is built from a description of the
/// property (the form in which markup compiles the binding of the
/// reference test).
fn length_binding() -> Option<AssignedBinding> {
    let path = CompiledBindingPathBuilder::new()
        .property(
            Rc::new(ClrPropertyInfo::read_only::<String, Value<i32>>("Length", |s| s.chars().count() as i32)),
            PropertyInfoAccessorFactory::create_plain_property_accessor(),
        )
        .build();
    let binding: Rc<dyn BindingBase> = CompiledBinding::new(path);
    Some(AssignedBinding::new(binding))
}

fn text_block_child(container: &ContentPresenter) -> Ref<TextBlock> {
    let child = container.child().unwrap();
    assert_eq!(child.get_type(), TextBlock::TYPE);
    child.cast::<TextBlock>().unwrap()
}

#[test]
fn should_use_display_member_binding() {
    let scope = start();
    let target = ItemsControl::new();
    target.set_display_member_binding(length_binding());
    let target = create_target_in(scope, target, |o| o.items_source = strs(&["Foo"]));

    let container = get_container(&target, 0);
    let text_block = text_block_child(&container);

    assert_eq!(text_block.text(), Some("3".to_string()));
}

#[test]
fn display_member_binding_can_be_changed() {
    let scope = start();
    let target = ItemsControl::new();
    target.set_display_member_binding(binding("Value"));
    let target = create_target_in(scope, target, |o| {
        o.items_source = Some(ItemsSource::from_items([Some(Item::new("Foo", Some("Bar")) as BoxedValue)]))
    });

    let container = get_container(&target, 0);
    let text_block = text_block_child(&container);

    assert_eq!(text_block.text(), Some("Bar".to_string()));

    target.set_display_member_binding(binding("Caption"));
    layout(&target);

    let container = get_container(&target, 0);
    let text_block = text_block_child(&container);

    assert_eq!(text_block.text(), Some("Foo".to_string()));
}

#[test]
#[should_panic(expected = "Cannot set both DisplayMemberBinding and ItemTemplate.")]
fn cannot_set_both_display_member_binding_and_item_template_1() {
    let target = create_target(|_| {});
    target.set_display_member_binding(length_binding());

    target.set_item_template(canvas_template());
}

#[test]
#[should_panic(expected = "Cannot set both DisplayMemberBinding and ItemTemplate.")]
fn cannot_set_both_display_member_binding_and_item_template_2() {
    let target = create_target(|o| o.item_template = canvas_template());

    target.set_display_member_binding(length_binding());
}

fn track_prepared(target: &ItemsControl) -> Rc<RefCell<Vec<Ref<Control>>>> {
    let result = Rc::new(RefCell::new(Vec::new()));
    let index = Cell::new(0);
    let sink = result.clone();
    target.container_prepared(move |e| {
        assert_eq!(index.get(), e.index());
        index.set(index.get() + 1);
        sink.borrow_mut().push(e.container().clone());
    });
    result
}

#[test]
fn container_prepared_is_raised_for_each_control_item_container() {
    let target = create_target(|_| {});
    let result = track_prepared(&target);

    target.items().add(Some(Control::boxed(button())));
    target.items().add(Some(Control::boxed(button())));
    target.items().add(Some(Control::boxed(button())));

    assert_eq!(result.borrow().len(), 3);
    assert_eq!(target.get_realized_containers(), *result.borrow());
}

#[test]
fn container_prepared_is_raised_for_each_item_container() {
    let target = create_target(|_| {});
    let result = track_prepared(&target);

    target.items().add(boxed_str("Foo"));
    target.items().add(boxed_str("Bar"));
    target.items().add(boxed_str("Baz"));

    assert_eq!(result.borrow().len(), 3);
    assert_eq!(target.get_realized_containers(), *result.borrow());
}

#[test]
fn container_prepared_is_raised_for_each_items_source_item_container_on_layout() {
    let items = Rc::new(FerroList::<String>::new());
    let target = create_target(|o| o.items_source = Some(items.clone().into()));
    let result = track_prepared(&target);

    items.add_range(["Foo".to_string(), "Bar".to_string(), "Baz".to_string()]);

    assert_eq!(result.borrow().len(), 3);
    assert_eq!(target.get_realized_containers(), *result.borrow());
}

#[test]
fn container_index_changed_is_raised_when_item_added() {
    let target = create_target(|o| o.items = boxed_items(&["Foo".to_string(), "Bar".to_string(), "Baz".to_string()]));
    let result = Rc::new(RefCell::new(Vec::new()));
    let index = Cell::new(1);
    let sink = result.clone();

    target.container_index_changed(move |e| {
        assert_eq!(index.get(), e.old_index());
        index.set(index.get() + 1);
        assert_eq!(index.get(), e.new_index());
        sink.borrow_mut().push(e.container().clone());
    });

    target.items().insert(1, boxed_str("Qux"));

    assert_eq!(result.borrow().len(), 2);
    assert_eq!(target.get_realized_containers()[2..], *result.borrow());
}

#[test]
fn container_clearing_is_raised_when_item_removed() {
    let target = create_target(|o| o.items = boxed_items(&["Foo".to_string(), "Bar".to_string(), "Baz".to_string()]));
    let expected = target.container_from_index(1).unwrap();
    let raised = Rc::new(Cell::new(0));
    let counter = raised.clone();

    target.container_clearing(move |e| {
        assert_eq!(&expected, e.container());
        counter.set(counter.get() + 1);
    });

    target.items().remove_at(1);

    assert_eq!(raised.get(), 1);
}

#[test]
fn container_clearing_is_raised_when_items_source_is_cleared() {
    let items_source =
        Rc::new(FerroList::from_items(["Foo".to_string(), "Bar".to_string(), "Baz".to_string()]));
    let target = create_target(|o| o.items_source = Some(items_source.clone().into()));

    let expected_containers: Vec<Ref<Control>> =
        items_source.to_vec().iter().map(|x| target.container_from_item(&box_item(x)).unwrap()).collect();
    let actual_containers = Rc::new(RefCell::new(Vec::new()));
    let raised = Rc::new(Cell::new(0));
    let (sink, counter) = (actual_containers.clone(), raised.clone());

    target.container_clearing(move |e| {
        sink.borrow_mut().push(e.container().clone());
        counter.set(counter.get() + 1);
    });

    items_source.clear();

    assert_eq!(raised.get(), 3);
    assert_eq!(expected_containers, *actual_containers.borrow());
}

#[test]
fn item_is_own_container_content_should_not_be_cleared_when_removed() {
    // Issue #11128.
    let item = ContentPresenter::new();
    item.set_content(boxed_str("foo"));
    let item_control: Ref<Control> = item.clone().upcast();
    let target = create_target(|o| o.items = controls(&[&item_control]));

    target.items().remove_at(0);

    assert!(items_equal(&item.content(), &boxed_str("foo")));
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

#[test]
fn control_items_that_are_not_containers_are_wrapped_in_containers() {
    // The non-virtualizing part of the reference test for issue #10825:
    // the items are controls but not of the container type.
    let items: Vec<Ref<Control>> = (0..3).map(|_| TextBlock::new().upcast()).collect();
    let item_refs: Vec<&Ref<Control>> = items.iter().collect();
    let target = create_target_of(
        instantiate(ItemsControlWithContainer { base: ItemsControl::construct() }).upcast(),
        |o| o.items = controls(&item_refs),
    );

    assert_eq!(target.get_realized_containers().len(), 3);
    for (index, item) in items.iter().enumerate() {
        let container = get_container_of::<ContainerControl>(&target, index);
        assert!(items_equal(&container.content(), &Some(Control::boxed(item.clone()))));
        assert_eq!(target.index_from_container(&container.clone().upcast()), index as i32);
    }
}

#[test]
fn items_panel_template_creates_the_panel() {
    let items_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| Some(Canvas::new().upcast::<Panel>()));
    let target = create_target(|o| {
        o.items_source = strs(&["Foo"]);
        o.items_panel = Some(items_panel);
    });

    assert!(target.items_panel_root().unwrap().is::<Canvas>());
}
