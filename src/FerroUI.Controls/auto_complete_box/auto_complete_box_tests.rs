//! Port of the reference `AutoCompleteBoxTests`.
//!
//! The tests of the reference that show a window do so with the simple
//! theme applied, which gives the text box and the list box of the template
//! of the tests their templates. [`start_styled_window`] adds control themes
//! with those templates to the styles of the application.

use super::{
    AutoCompleteAsyncPopulator, AutoCompleteBox, AutoCompleteFilterMode, AutoCompleteFilterPredicate,
    AutoCompleteSelector, PopulatedEventArgs,
};
use crate::primitives::Popup;
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::test_support::{test_scope, TestRoot};
use crate::test_support_buttons::focus_scope;
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    Application, ContextMenu, Control, DataValidationErrors, ItemsSource, ListBox, Panel, StackPanel, TextBox, Window,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{
    BindingBase, BindingError, BindingErrorType, BindingMode, BindingNotification, BindingPriority, CompiledBinding,
    CompiledBindingPathBuilder, ReflectionBinding,
};
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::reactive::Observable;
use ferroui_base::threading::{CancellationToken, Dispatcher, OperationCanceledError};
use ferroui_base::utilities::{CompareOptions, CultureInfo, ICompareRules, ICultureDataProvider};
use crate::utils::{
    ISelectionAdapter, SelectingItemsControlSelectionAdapter, SelectingItemsControlSelectionAdapterOverrides,
};
use crate::{AssignedBinding, ControlImpl, SelectionChangedEventArgs};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_model, instantiate, AnyValue, BoxedValue, FerroLocator,
    FerroObject, FerroObjectExtensions, FerroObjectImpl, LocatorExtensions, Ref, Size, StyledElementImpl, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

// --- helpers ----------------------------------------------------------------

/// A running unit test application, with the text services of the tests
/// registered under the services of the application.
struct AppScope {
    // Dropped in this order: the application first.
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

/// Starts an application with the services of the tests that show windows
/// (the `Services` of the reference) and the control themes of the parts of
/// the template of the tests.
fn start_styled_window() -> AppScope {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(TestServices::styled_window().with_render_interface(render_interface));
    crate::testing::add_autocomplete_themes(&Application::current().expect("the application is running").styles());
    AppScope { _app: app, _text: text }
}

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

fn run_test(test: impl FnOnce(&Ref<AutoCompleteBox>, &Ref<TextBox>)) {
    let _app = start_styled_window();
    let control = create_control();
    control.set_items_source(Some(create_simple_string_array()));
    let text_box = get_text_box(&control);
    let window = Window::new();
    window.set_content(Some(Control::boxed(control.clone())));
    window.apply_styling();
    window.apply_template();
    window.presenter().expect("the window has a presenter").apply_template();
    run_jobs();
    test(&control, &text_box);
}

fn create_control() -> Ref<AutoCompleteBox> {
    let auto_complete_box = AutoCompleteBox::new();
    auto_complete_box.set_template(Some(create_template()));

    auto_complete_box.apply_template();
    auto_complete_box
}

fn get_text_box(control: &Ref<AutoCompleteBox>) -> Ref<TextBox> {
    control
        .get_visual_descendants()
        .find_map(|visual| visual.cast::<TextBox>())
        .expect("the template of the control has a text box")
}

fn create_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<AutoCompleteBox>(|control, scope| {
        let text_box = TextBox::new();
        text_box.set_name(Some("PART_TextBox".to_string()));
        text_box.bind_indexer(
            &!TextBox::caret_index_property().bind(),
            &control.indexer(&!AutoCompleteBox::caret_index_property().bind()),
        );
        let text_box = text_box.register_in_name_scope(&**scope);

        let list_box = ListBox::new();
        list_box.set_name(Some("PART_SelectingItemsControl".to_string()));
        let list_box = list_box.register_in_name_scope(&**scope);

        let popup = Popup::new();
        popup.set_name(Some("PART_Popup".to_string()));
        popup.set_placement_target(control);
        let popup = popup.register_in_name_scope(&**scope);

        let panel = Panel::new();
        panel.children().add(text_box);
        panel.children().add(popup);
        panel.children().add(list_box);

        panel.upcast()
    })
}

/// Retrieves a defined predicate filter through a new auto-complete box
/// control instance.
fn get_filter(mode: AutoCompleteFilterMode) -> Option<AutoCompleteFilterPredicate<Option<String>>> {
    let control = AutoCompleteBox::new();
    control.set_filter_mode(mode);
    control.text_filter()
}

/// The filter of a mode that has one, as a function of the search text and
/// the value.
fn get_not_null_filter(mode: AutoCompleteFilterMode) -> impl Fn(&str, &str) -> bool {
    let filter = get_filter(mode).expect("the mode has a filter");
    move |search, value| filter.invoke(Some(search), &Some(value.to_string()))
}

fn text_of(item: &Option<BoxedValue>) -> String {
    ValueTypes::to_display_string(item.as_ref())
}

fn raise_key_down(target: &InputElement, key: Key) {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = key;
    target.raise_event(&args);
}

fn validation_error_observable(exception: &BindingError) -> Rc<dyn ferroui_base::reactive::IObservable<BoxedValue>> {
    Observable::single_value(Rc::new(BindingNotification::with_error(
        exception.clone(),
        BindingErrorType::DataValidationError,
    )) as BoxedValue)
}

/// Asserts that the data validation errors of the control are exactly
/// `exception`.
fn assert_errors_are(target: &Control, exception: &BindingError) {
    let errors = DataValidationErrors::get_errors(target).map(|errors| errors.to_vec()).unwrap_or_default();
    assert_eq!(1, errors.len());
    let error = errors[0].clone();
    let error: &dyn AnyValue = &*error;
    assert!(error.downcast_ref::<BindingError>() == Some(exception));
}

/// The `TestContextMenu` of the reference: a context menu that says it is
/// open.
fn test_context_menu() -> Ref<ContextMenu> {
    let menu = ContextMenu::new();
    menu.set_is_open(true);
    menu
}

/// The view model of the tests of a bound text.
struct AutoCompleteBoxViewModel {
    text_value: RefCell<Option<String>>,
    property_changed: Event<str>,
}

impl AutoCompleteBoxViewModel {
    fn new() -> Rc<Self> {
        let view_model = Model::new_model(Self { text_value: RefCell::new(None), property_changed: Event::new() });
        view_model.set_text_value(Some("foo".to_string()));
        view_model
    }

    fn text_value(&self) -> Option<String> {
        self.text_value.borrow().clone()
    }

    fn set_text_value(&self, value: Option<String>) {
        if *self.text_value.borrow() == value {
            return;
        }
        *self.text_value.borrow_mut() = value;
        self.property_changed.raise("TextValue");
    }

    fn update_text_value_twice(&self) {
        self.set_text_value(Some("foo".to_string()));
        self.set_text_value(Some("bar".to_string()));
    }
}

impl INotifyPropertyChanged for AutoCompleteBoxViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(AutoCompleteBoxViewModel, |b| b.notify_property_changed().property::<ferroui_base::data::core::Maybe<
    String,
>>("TextValue", |o| o.text_value(), |o, v| o.set_text_value(v)));

/// Binds the text of the control both ways to the text value of the view
/// model.
fn bind_text(control: &AutoCompleteBox, view_model: &Rc<AutoCompleteBoxViewModel>) {
    let path = CompiledBindingPathBuilder::new()
        .typed_property::<AutoCompleteBoxViewModel, Option<String>>(
            "TextValue",
            |vm| vm.text_value(),
            Some(Rc::new(|vm: &AutoCompleteBoxViewModel, value| vm.set_text_value(value))),
        )
        .build();
    let binding = CompiledBinding::new(path)
        .with_mode(BindingMode::TwoWay)
        .with_source(Some(view_model.clone() as BoxedValue));
    control.bind_indexer(&AutoCompleteBox::text_property().bind(), &binding);
}

// --- tests ------------------------------------------------------------------

#[test]
fn search_filters() {
    let _scope = test_scope();

    assert!(get_not_null_filter(AutoCompleteFilterMode::Contains)("am", "name"));
    assert!(get_not_null_filter(AutoCompleteFilterMode::Contains)("AME", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::Contains)("hello", "name"));

    assert!(get_not_null_filter(AutoCompleteFilterMode::ContainsCaseSensitive)("na", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::ContainsCaseSensitive)("AME", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::ContainsCaseSensitive)("hello", "name"));

    assert!(get_filter(AutoCompleteFilterMode::Custom).is_none());
    assert!(get_filter(AutoCompleteFilterMode::None).is_none());

    assert!(get_not_null_filter(AutoCompleteFilterMode::Equals)("na", "na"));
    assert!(get_not_null_filter(AutoCompleteFilterMode::Equals)("na", "NA"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::Equals)("hello", "name"));

    assert!(get_not_null_filter(AutoCompleteFilterMode::EqualsCaseSensitive)("na", "na"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::EqualsCaseSensitive)("na", "NA"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::EqualsCaseSensitive)("hello", "name"));

    assert!(get_not_null_filter(AutoCompleteFilterMode::StartsWith)("na", "name"));
    assert!(get_not_null_filter(AutoCompleteFilterMode::StartsWith)("NAM", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::StartsWith)("hello", "name"));

    assert!(get_not_null_filter(AutoCompleteFilterMode::StartsWithCaseSensitive)("na", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::StartsWithCaseSensitive)("NAM", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::StartsWithCaseSensitive)("hello", "name"));
}

#[test]
fn ordinal_search_filters() {
    let _scope = test_scope();

    assert!(get_not_null_filter(AutoCompleteFilterMode::ContainsOrdinal)("am", "name"));
    assert!(get_not_null_filter(AutoCompleteFilterMode::ContainsOrdinal)("AME", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::ContainsOrdinal)("hello", "name"));

    assert!(get_not_null_filter(AutoCompleteFilterMode::ContainsOrdinalCaseSensitive)("na", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::ContainsOrdinalCaseSensitive)("AME", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::ContainsOrdinalCaseSensitive)("hello", "name"));

    assert!(get_not_null_filter(AutoCompleteFilterMode::EqualsOrdinal)("na", "na"));
    assert!(get_not_null_filter(AutoCompleteFilterMode::EqualsOrdinal)("na", "NA"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::EqualsOrdinal)("hello", "name"));

    assert!(get_not_null_filter(AutoCompleteFilterMode::EqualsOrdinalCaseSensitive)("na", "na"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::EqualsOrdinalCaseSensitive)("na", "NA"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::EqualsOrdinalCaseSensitive)("hello", "name"));

    assert!(get_not_null_filter(AutoCompleteFilterMode::StartsWithOrdinal)("na", "name"));
    assert!(get_not_null_filter(AutoCompleteFilterMode::StartsWithOrdinal)("NAM", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::StartsWithOrdinal)("hello", "name"));

    assert!(get_not_null_filter(AutoCompleteFilterMode::StartsWithOrdinalCaseSensitive)("na", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::StartsWithOrdinalCaseSensitive)("NAM", "name"));
    assert!(!get_not_null_filter(AutoCompleteFilterMode::StartsWithOrdinalCaseSensitive)("hello", "name"));
}

#[test]
fn fires_drop_down_events() {
    run_test(|control, textbox| {
        let open_event = Rc::new(Cell::new(false));
        let close_event = Rc::new(Cell::new(false));
        let flag = open_event.clone();
        control.drop_down_opened(move || flag.set(true));
        let flag = close_event.clone();
        control.drop_down_closed(move || flag.set(true));
        control.set_items_source(Some(create_simple_string_array()));

        textbox.set_text(Some("a"));
        run_jobs();
        assert!(control.search_text().as_deref() == Some("a"));
        assert!(control.is_drop_down_open());
        assert!(open_event.get());

        textbox.set_text(Some(""));
        run_jobs();
        assert!(control.search_text().as_deref() == Some(""));
        assert!(!control.is_drop_down_open());
        assert!(close_event.get());
    });
}

#[test]
fn custom_filter_mode_without_item_filter_setting_throws_exception() {
    run_test(|control, _textbox| {
        control.set_filter_mode(AutoCompleteFilterMode::Custom);
        let result = catch_unwind(AssertUnwindSafe(|| control.set_text(Some("a"))));
        assert!(result.is_err());
    });
}

#[test]
fn text_completion_via_text_property() {
    run_test(|control, _textbox| {
        control.set_is_text_completion_enabled(true);

        assert_eq!(Some(""), control.text().as_deref());
        control.set_text(Some("close"));
        assert!(control.selected_item().is_some());
    });
}

#[test]
fn text_completion_selects_text() {
    run_test(|control, textbox| {
        control.set_is_text_completion_enabled(true);

        textbox.set_text(Some("ac"));
        textbox.set_selection_start(2);
        textbox.set_selection_end(2);
        run_jobs();

        assert!(control.is_drop_down_open());
        assert!((textbox.selection_end() - textbox.selection_start()).abs() > 2);
    });
}

#[test]
fn text_changed_event_fires() {
    run_test(|control, textbox| {
        let text_changed = Rc::new(Cell::new(false));
        let flag = text_changed.clone();
        control.text_changed(move |_, _| flag.set(true));

        textbox.set_text(Some("a"));
        run_jobs();
        assert!(text_changed.get());

        text_changed.set(false);
        control.set_text(Some("conversati"));
        run_jobs();
        assert!(text_changed.get());

        text_changed.set(false);
        control.set_text(None);
        run_jobs();
        assert!(text_changed.get());
    });
}

#[test]
fn minimum_prefix_length_works() {
    run_test(|control, textbox| {
        textbox.set_text(Some("a"));
        run_jobs();
        assert!(control.is_drop_down_open());

        textbox.set_text(Some(""));
        run_jobs();
        assert!(!control.is_drop_down_open());

        control.set_minimum_prefix_length(3);

        textbox.set_text(Some("a"));
        run_jobs();
        assert!(!control.is_drop_down_open());

        textbox.set_text(Some("acc"));
        run_jobs();
        assert!(control.is_drop_down_open());
    });
}

#[test]
fn can_cancel_drop_down_opening() {
    run_test(|control, textbox| {
        control.drop_down_opening(|e| e.set_cancel(true));

        textbox.set_text(Some("a"));
        run_jobs();
        assert!(!control.is_drop_down_open());
    });
}

#[test]
fn can_cancel_drop_down_closing() {
    run_test(|control, textbox| {
        control.drop_down_closing(|e| e.set_cancel(true));

        textbox.set_text(Some("a"));
        run_jobs();
        assert!(control.is_drop_down_open());

        control.set_is_drop_down_open(false);
        assert!(control.is_drop_down_open());
    });
}

#[test]
fn can_cancel_population() {
    run_test(|control, textbox| {
        let populating = Rc::new(Cell::new(false));
        let populated = Rc::new(Cell::new(false));
        control.set_filter_mode(AutoCompleteFilterMode::None);
        let flag = populating.clone();
        control.populating(move |e| {
            e.set_cancel(true);
            flag.set(true);
        });
        let flag = populated.clone();
        control.populated(move |_| flag.set(true));

        textbox.set_text(Some("accounti"));
        run_jobs();

        assert!(populating.get());
        assert!(!populated.get());
    });
}

#[test]
fn custom_population_supported() {
    run_test(|control, textbox| {
        let custom = "Custom!";
        let search = "accounti";
        let populated = Rc::new(Cell::new(false));
        let populated_ok = Rc::new(Cell::new(false));
        control.set_filter_mode(AutoCompleteFilterMode::None);
        let weak = control.downgrade();
        control.populating(move |e| {
            let control = weak.upgrade().expect("the control is alive");
            control.set_items_source(Some(ItemsSource::from_strs([custom])));
            assert_eq!(Some(search), e.parameter());
        });
        let (flag, ok) = (populated.clone(), populated_ok.clone());
        control.populated(move |e: &PopulatedEventArgs| {
            flag.set(true);
            ok.set(e.data().count() == 1);
        });

        textbox.set_text(Some(search));
        run_jobs();

        assert!(populated.get());
        assert!(populated_ok.get());
    });
}

#[test]
fn text_completion() {
    run_test(|control, textbox| {
        control.set_is_text_completion_enabled(true);
        textbox.set_text(Some("accounti"));
        let length = textbox.text().map_or(0, |text| text.encode_utf16().count() as i32);
        textbox.set_selection_end(length);
        textbox.set_selection_start(length);
        run_jobs();
        assert_eq!(Some("accounti"), control.search_text().as_deref());
        assert_eq!(Some("accounting"), textbox.text().as_deref());
    });
}

/// Sets the texts of the search tests and compares the text of the control
/// with the one of the text box after each.
fn assert_text_follows_text_box(control: &AutoCompleteBox, textbox: &TextBox) {
    for text in ["a", "acc", "a", "", "cook", "accept", "cook"] {
        textbox.set_text(Some(text));
        run_jobs();
        assert_eq!(textbox.text(), control.text());
    }
}

#[test]
fn string_search() {
    run_test(|control, textbox| {
        assert_text_follows_text_box(control, textbox);
    });
}

#[test]
fn item_search() {
    run_test(|control, textbox| {
        control.set_filter_mode(AutoCompleteFilterMode::Custom);
        control.set_item_filter(Some(AutoCompleteFilterPredicate::new(|_, item: &Option<BoxedValue>| {
            item.as_ref().is_some_and(|item| {
                let item: &dyn AnyValue = &**item;
                item.downcast_ref::<String>().is_some()
            })
        })));

        // Just set to null briefly to exercise that code path.
        let filter = control.item_filter();
        assert!(filter.is_some());
        control.set_item_filter(None);
        assert!(control.item_filter().is_none());
        control.set_item_filter(filter);
        assert!(control.item_filter().is_some());

        assert_text_follows_text_box(control, textbox);
    });
}

#[test]
fn custom_text_selector() {
    run_test(|control, _textbox| {
        let items_source = control.items_source().expect("the control has items");

        let selected_item = items_source.get_at(0);
        let input = "42";

        control.set_text_selector(Some(AutoCompleteSelector::new(|text, item: &Option<String>| {
            format!("{}{}", text.unwrap_or_default(), item.as_deref().unwrap_or_default())
        })));
        let text_selector = control.text_selector().expect("the selector was set");
        assert_eq!(text_selector.invoke(Some("4"), &Some("2".to_string())), "42");

        control.set_text(Some(input));
        control.set_selected_item(selected_item.clone());
        assert_eq!(control.text(), Some(text_selector.invoke(Some(input), &Some(text_of(&selected_item)))));
    });
}

#[test]
fn custom_item_selector() {
    run_test(|control, _textbox| {
        let items_source = control.items_source().expect("the control has items");

        let selected_item = items_source.get_at(0);
        let input = "42";

        control.set_item_selector(Some(AutoCompleteSelector::new(|text, item: &BoxedValue| {
            format!("{}{}", text.unwrap_or_default(), ValueTypes::to_display_string(Some(item)))
        })));
        let item_selector = control.item_selector().expect("the selector was set");
        let two: BoxedValue = Rc::new(2);
        assert_eq!(item_selector.invoke(Some("4"), &two), "42");

        control.set_text(Some(input));
        control.set_selected_item(selected_item.clone());
        assert_eq!(
            control.text(),
            Some(item_selector.invoke(Some(input), selected_item.as_ref().expect("the item is not null")))
        );
    });
}

#[test]
fn text_validation() {
    run_test(|control, _textbox| {
        let exception = BindingError::message("failed validation");
        control.bind_property_untyped(
            AutoCompleteBox::text_property().as_property(),
            validation_error_observable(&exception),
            BindingPriority::LocalValue,
        );
        run_jobs();

        assert!(DataValidationErrors::get_has_errors(control));
        assert_errors_are(control, &exception);
    });
}

#[test]
fn text_validation_text_box_errors_binding() {
    run_test(|control, textbox| {
        // Simulate the template binding that would be used within the
        // control theme of the auto-complete box for the inner text box
        // part: the errors of the text box are those of the control.
        let object: &FerroObject = control;
        textbox.bind_typed_value(
            DataValidationErrors::errors_property(),
            FerroObjectExtensions::get_binding_observable(object, DataValidationErrors::errors_property()),
            BindingPriority::LocalValue,
        );

        let exception = BindingError::message("failed validation");
        control.bind_property_untyped(
            AutoCompleteBox::text_property().as_property(),
            validation_error_observable(&exception),
            BindingPriority::LocalValue,
        );
        run_jobs();

        assert!(DataValidationErrors::get_has_errors(control));
        assert_errors_are(control, &exception);

        assert!(DataValidationErrors::get_has_errors(textbox));
        assert_errors_are(textbox, &exception);
    });
}

#[test]
fn selected_item_validation() {
    run_test(|control, _textbox| {
        let exception = BindingError::message("failed validation");
        control.bind_property_untyped(
            AutoCompleteBox::selected_item_property().as_property(),
            validation_error_observable(&exception),
            BindingPriority::LocalValue,
        );
        run_jobs();

        assert!(DataValidationErrors::get_has_errors(control));
        assert_errors_are(control, &exception);
    });
}

#[test]
fn explicit_dropdown_open_request_minimum_prefix_length_0() {
    run_test(|control, _textbox| {
        control.set_text(Some(""));
        control.set_minimum_prefix_length(0);
        run_jobs();

        assert!(!control.is_drop_down_open());

        raise_key_down(control, Key::Down);

        run_jobs();

        assert!(control.is_drop_down_open());
    });
}

#[test]
fn caret_index_changes() {
    let text = "Sample text";
    let expected_text = "Saple text";
    run_test(|control, textbox| {
        control.set_text(Some(text));
        control.measure(Size::INFINITY);
        run_jobs();

        raise_key_down(textbox, Key::Right);
        run_jobs();

        assert_eq!(1, control.caret_index());
        assert_eq!(textbox.caret_index(), control.caret_index());

        control.set_caret_index(3);

        assert_eq!(3, control.caret_index());
        assert_eq!(textbox.caret_index(), control.caret_index());

        raise_key_down(textbox, Key::Back);
        run_jobs();

        assert_eq!(2, control.caret_index());
        assert_eq!(textbox.caret_index(), control.caret_index());
        assert!(control.text().as_deref() == Some(expected_text) && textbox.text().as_deref() == Some(expected_text));
    });
}

#[test]
fn attempting_to_open_without_items_does_not_prevent_future_opening_with_items() {
    run_test(|control, _textbox| {
        // Allow the drop down to open without anything entered.
        control.set_minimum_prefix_length(0);

        // Clear the items.
        let source = control.items_source();
        control.set_items_source(None);
        control.set_is_drop_down_open(true);

        // DropDown was not actually opened because there are no items.
        assert!(!control.is_drop_down_open());

        // Set the items and try to open the drop down again.
        control.set_items_source(source);
        control.set_is_drop_down_open(true);

        // DropDown can now be opened.
        assert!(control.is_drop_down_open());
    });
}

/// The services of the reference tests that move the keyboard focus.
fn start_focus_services() -> (AppScope, crate::test_support_buttons::FocusScope) {
    let text = TextTestScope::new();
    let app = UnitTestApplication::start(TestServices::real_focus());
    (AppScope { _app: app, _text: text }, focus_scope())
}

#[test]
fn opening_context_menu_does_not_lose_selection() {
    let _services = start_focus_services();

    let target1 = create_control();
    target1.set_context_menu(&test_context_menu());
    let text_box1 = get_text_box(&target1);
    text_box1.set_text(Some("1234"));

    let target2 = create_control();
    let text_box2 = get_text_box(&target2);
    text_box2.set_text(Some("5678"));

    let sp = StackPanel::new();
    sp.children().add(target1.clone());
    sp.children().add(target2.clone());

    target1.apply_template();
    target2.apply_template();

    let _root = TestRoot::with_child(sp);

    text_box1.set_selection_start(0);
    text_box1.set_selection_end(3);

    target1.focus();
    assert!(!target2.is_focused());
    assert!(target1.is_focused());

    target2.focus();

    assert_eq!("123", text_box1.selected_text());
}

#[test]
fn losing_focus_closes_drop_down() {
    let _services = start_focus_services();

    let target1 = create_control();
    target1.set_items_source(Some(create_simple_string_array()));
    let text_box1 = get_text_box(&target1);

    let target2 = create_control();

    target1.apply_template();
    target2.apply_template();

    let sp = StackPanel::new();
    sp.children().add(target1.clone());
    sp.children().add(target2.clone());
    let _root = TestRoot::with_child(sp);

    target1.focus();
    text_box1.set_text(Some("a"));
    run_jobs();
    assert!(target1.is_drop_down_open());

    target2.focus();
    run_jobs();

    assert!(!target1.is_focused());
    assert!(!target1.is_drop_down_open());
}

#[test]
fn placeholder_foreground_can_be_set() {
    let _app = start_styled_window();
    let control = create_control();
    control.set_placeholder_text(Some("Search..."));
    let green: Rc<dyn IBrush> = Brushes::green();
    control.set_placeholder_foreground(Some(green.clone()));

    assert!(control.placeholder_foreground() == Some(green));
}

#[test]
fn bound_text_will_update_always() {
    let _app = start_styled_window();
    let view_model = AutoCompleteBoxViewModel::new();

    let control = create_control();

    // Setup the binding.
    bind_text(&control, &view_model);

    // Ensure the bound text matches "foo".
    assert_eq!(Some("foo"), control.text().as_deref());

    // Change the view model value several times and ensure the bound text
    // is updated.
    for _ in 0..10 {
        view_model.update_text_value_twice();
        run_jobs();
        assert_eq!(Some("bar"), control.text().as_deref());
    }
}

#[test]
fn bound_text_will_update_from_bar_to_bar_via_foo() {
    let _app = start_styled_window();
    let view_model = AutoCompleteBoxViewModel::new();
    view_model.set_text_value(Some("bar".to_string()));

    let control = create_control();
    control.apply_template();

    // Setup the binding.
    bind_text(&control, &view_model);

    assert_eq!(Some("bar"), control.text().as_deref());

    let text_changed_count = Rc::new(Cell::new(0));
    let count = text_changed_count.clone();
    control.text_changed(move |_, _| count.set(count.get() + 1));

    // Change the view model value "bar" -> "foo" -> "bar".
    view_model.update_text_value_twice();

    // Programmatic text property updates should synchronously raise the
    // text changed event, and the text changed handler of the text box is
    // suppressed for the corresponding updates of the text of the text box.
    assert_eq!(Some("bar"), control.text().as_deref());
    assert_eq!(2, text_changed_count.get());

    run_jobs();

    assert_eq!(Some("bar"), control.text().as_deref());
    assert_eq!(2, text_changed_count.get());
}

/// Creates a large list of strings for auto-complete box testing.
fn create_simple_string_array() -> ItemsSource {
    ItemsSource::from_strs(SIMPLE_STRINGS)
}

const SIMPLE_STRINGS: [&str; 587] = [
    "a",
    "abide",
    "able",
    "about",
    "above",
    "absence",
    "absurd",
    "accept",
    "acceptance",
    "accepted",
    "accepting",
    "access",
    "accessed",
    "accessible",
    "accident",
    "accidentally",
    "accordance",
    "account",
    "accounting",
    "accounts",
    "accusation",
    "accustomed",
    "ache",
    "across",
    "act",
    "active",
    "actual",
    "actually",
    "ada",
    "added",
    "adding",
    "addition",
    "additional",
    "additions",
    "address",
    "addressed",
    "addresses",
    "addressing",
    "adjourn",
    "adoption",
    "advance",
    "advantage",
    "adventures",
    "advice",
    "advisable",
    "advise",
    "affair",
    "affectionately",
    "afford",
    "afore",
    "afraid",
    "after",
    "afterwards",
    "again",
    "against",
    "age",
    "aged",
    "agent",
    "ago",
    "agony",
    "agree",
    "agreed",
    "agreement",
    "ah",
    "ahem",
    "air",
    "airs",
    "ak",
    "alarm",
    "alarmed",
    "alas",
    "alice",
    "alive",
    "all",
    "allow",
    "almost",
    "alone",
    "along",
    "aloud",
    "already",
    "also",
    "alteration",
    "altered",
    "alternate",
    "alternately",
    "altogether",
    "always",
    "am",
    "ambition",
    "among",
    "an",
    "ancient",
    "and",
    "anger",
    "angrily",
    "angry",
    "animal",
    "animals",
    "ann",
    "annoy",
    "annoyed",
    "another",
    "answer",
    "answered",
    "answers",
    "antipathies",
    "anxious",
    "anxiously",
    "any",
    "anyone",
    "anything",
    "anywhere",
    "appealed",
    "appear",
    "appearance",
    "appeared",
    "appearing",
    "appears",
    "applause",
    "apple",
    "apples",
    "applicable",
    "apply",
    "approach",
    "arch",
    "archbishop",
    "arches",
    "archive",
    "are",
    "argue",
    "argued",
    "argument",
    "arguments",
    "arise",
    "arithmetic",
    "arm",
    "arms",
    "around",
    "arranged",
    "array",
    "arrived",
    "arrow",
    "arrum",
    "as",
    "ascii",
    "ashamed",
    "ask",
    "askance",
    "asked",
    "asking",
    "asleep",
    "assembled",
    "assistance",
    "associated",
    "at",
    "ate",
    "atheling",
    "atom",
    "attached",
    "attempt",
    "attempted",
    "attempts",
    "attended",
    "attending",
    "attends",
    "audibly",
    "australia",
    "author",
    "authority",
    "available",
    "avoid",
    "away",
    "awfully",
    "axes",
    "axis",
    "b",
    "baby",
    "back",
    "backs",
    "bad",
    "bag",
    "baked",
    "balanced",
    "bank",
    "banks",
    "banquet",
    "bark",
    "barking",
    "barley",
    "barrowful",
    "based",
    "bat",
    "bathing",
    "bats",
    "bawled",
    "be",
    "beak",
    "bear",
    "beast",
    "beasts",
    "beat",
    "beating",
    "beau",
    "beauti",
    "beautiful",
    "beautifully",
    "beautify",
    "became",
    "because",
    "become",
    "becoming",
    "bed",
    "beds",
    "bee",
    "been",
    "before",
    "beg",
    "began",
    "begged",
    "begin",
    "beginning",
    "begins",
    "begun",
    "behead",
    "beheaded",
    "beheading",
    "behind",
    "being",
    "believe",
    "believed",
    "bells",
    "belong",
    "belongs",
    "beloved",
    "below",
    "belt",
    "bend",
    "bent",
    "besides",
    "best",
    "better",
    "between",
    "bill",
    "binary",
    "bird",
    "birds",
    "birthday",
    "bit",
    "bite",
    "bitter",
    "blacking",
    "blades",
    "blame",
    "blasts",
    "bleeds",
    "blew",
    "blow",
    "blown",
    "blows",
    "body",
    "boldly",
    "bone",
    "bones",
    "book",
    "books",
    "boon",
    "boots",
    "bore",
    "both",
    "bother",
    "bottle",
    "bottom",
    "bough",
    "bound",
    "bowed",
    "bowing",
    "box",
    "boxed",
    "boy",
    "brain",
    "branch",
    "branches",
    "brandy",
    "brass",
    "brave",
    "breach",
    "bread",
    "break",
    "breath",
    "breathe",
    "breeze",
    "bright",
    "brightened",
    "bring",
    "bringing",
    "bristling",
    "broke",
    "broken",
    "brother",
    "brought",
    "brown",
    "brush",
    "brushing",
    "burn",
    "burning",
    "burnt",
    "burst",
    "bursting",
    "busily",
    "business",
    "business@pglaf",
    "busy",
    "but",
    "butter",
    "buttercup",
    "buttered",
    "butterfly",
    "buttons",
    "by",
    "bye",
    "c",
    "cackled",
    "cake",
    "cakes",
    "calculate",
    "calculated",
    "call",
    "called",
    "calling",
    "calmly",
    "came",
    "camomile",
    "can",
    "canary",
    "candle",
    "cannot",
    "canterbury",
    "canvas",
    "capering",
    "capital",
    "card",
    "cardboard",
    "cards",
    "care",
    "carefully",
    "cares",
    "carried",
    "carrier",
    "carroll",
    "carry",
    "carrying",
    "cart",
    "cartwheels",
    "case",
    "cat",
    "catch",
    "catching",
    "caterpillar",
    "cats",
    "cattle",
    "caucus",
    "caught",
    "cauldron",
    "cause",
    "caused",
    "cautiously",
    "cease",
    "ceiling",
    "centre",
    "certain",
    "certainly",
    "chain",
    "chains",
    "chair",
    "chance",
    "chanced",
    "change",
    "changed",
    "changes",
    "changing",
    "chapter",
    "character",
    "charge",
    "charges",
    "charitable",
    "charities",
    "chatte",
    "cheap",
    "cheated",
    "check",
    "checked",
    "checks",
    "cheeks",
    "cheered",
    "cheerfully",
    "cherry",
    "cheshire",
    "chief",
    "child",
    "childhood",
    "children",
    "chimney",
    "chimneys",
    "chin",
    "choice",
    "choke",
    "choked",
    "choking",
    "choose",
    "choosing",
    "chop",
    "chorus",
    "chose",
    "christmas",
    "chrysalis",
    "chuckled",
    "circle",
    "circumstances",
    "city",
    "civil",
    "claim",
    "clamour",
    "clapping",
    "clasped",
    "classics",
    "claws",
    "clean",
    "clear",
    "cleared",
    "clearer",
    "clearly",
    "clever",
    "climb",
    "clinging",
    "clock",
    "close",
    "closed",
    "closely",
    "closer",
    "clubs",
    "coast",
    "coaxing",
    "codes",
    "coils",
    "cold",
    "collar",
    "collected",
    "collection",
    "come",
    "comes",
    "comfits",
    "comfort",
    "comfortable",
    "comfortably",
    "coming",
    "commercial",
    "committed",
    "common",
    "commotion",
    "company",
    "compilation",
    "complained",
    "complaining",
    "completely",
    "compliance",
    "comply",
    "complying",
    "compressed",
    "computer",
    "computers",
    "concept",
    "concerning",
    "concert",
    "concluded",
    "conclusion",
    "condemn",
    "conduct",
    "confirmation",
    "confirmed",
    "confused",
    "confusing",
    "confusion",
    "conger",
    "conqueror",
    "conquest",
    "consented",
    "consequential",
    "consider",
    "considerable",
    "considered",
    "considering",
    "constant",
    "consultation",
    "contact",
    "contain",
    "containing",
    "contempt",
    "contemptuous",
    "contemptuously",
    "content",
    "continued",
    "contract",
    "contradicted",
    "contributions",
    "conversation",
    "conversations",
    "convert",
    "cook",
    "cool",
    "copied",
    "copies",
    "copy",
    "copying",
    "copyright",
    "corner",
    "corners",
    "corporation",
    "corrupt",
    "cost",
    "costs",
    "could",
    "couldn",
    "counting",
    "countries",
    "country",
    "couple",
    "couples",
    "courage",
    "course",
    "court",
    "courtiers",
    "coward",
    "crab",
    "crash",
    "crashed",
    "crawled",
    "crawling",
    "crazy",
    "created",
    "creating",
    "creation",
    "creature",
    "creatures",
    "credit",
    "creep",
    "crept",
    "cried",
    "cries",
    "crimson",
    "critical",
    "crocodile",
    "croquet",
    "croqueted",
    "croqueting",
    "cross",
    "crossed",
    "crossly",
    "crouched",
    "crowd",
    "crowded",
    "crown",
    "crumbs",
    "crust",
    "cry",
    "crying",
    "cucumber",
    "cunning",
    "cup",
    "cupboards",
    "cur",
    "curiosity",
    "curious",
    "curiouser",
    "curled",
    "curls",
    "curly",
    "currants",
    "current",
    "curtain",
    "curtsey",
    "curtseying",
    "curving",
    "cushion",
    "custard",
    "custody",
    "cut",
    "cutting",
];

// --- additional tests (not in the reference) ----------------------------------

/// A control that is itself a selection adapter.
#[repr(C)]
struct AdapterControl {
    base: Control,
    items_source: RefCell<Option<ItemsSource>>,
}

ferro_class!(AdapterControl: Control);
ferro_class_info!(AdapterControl {
    new: AdapterControl::new,
    interfaces: [Rc<dyn ISelectionAdapter> => AdapterControl::as_selection_adapter],
});
ferro_impl_classes!(
    AdapterControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl AdapterControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct(), items_source: RefCell::new(None) })
    }

    fn as_selection_adapter(this: Ref<Self>) -> Rc<dyn ISelectionAdapter> {
        Rc::new(AdapterControlHandle(this))
    }
}

struct AdapterControlHandle(Ref<AdapterControl>);

impl ISelectionAdapter for AdapterControlHandle {
    fn selected_item(&self) -> Option<BoxedValue> {
        None
    }

    fn set_selected_item(&self, _value: Option<BoxedValue>) {}

    fn selection_changed(&self, _handler: Rc<dyn Fn(&SelectionChangedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    fn items_source(&self) -> Option<ItemsSource> {
        self.0.items_source.borrow().clone()
    }

    fn set_items_source(&self, value: Option<ItemsSource>) {
        *self.0.items_source.borrow_mut() = value;
    }

    fn commit(&self, _handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    fn cancel(&self, _handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    fn handle_key_down(&self, _e: &KeyEventArgs) {}
}

/// The selection adapter part of the template can be any element whose
/// class is a selection adapter; it is handed the view of the control.
#[test]
fn selection_adapter_part_is_found_by_its_contract() {
    let _scope = test_scope();

    let adapter = AdapterControl::new();
    let part = adapter.clone();
    let control = AutoCompleteBox::new();
    control.set_template(Some(FuncControlTemplate::for_type::<AutoCompleteBox>(move |_, scope| {
        part.set_name(Some("PART_SelectionAdapter".to_string()));
        part.clone().register_in_name_scope(&**scope).upcast()
    })));
    control.apply_template();

    assert!(control.selection_adapter().is_some());
    assert!(adapter.items_source.borrow().is_some());
}

/// The ordinal filters that ignore case fold character by character, to
/// upper case, without expansions and without a culture.
#[test]
fn ordinal_filters_fold_single_characters_without_culture() {
    let _scope = test_scope();

    for mode in [
        AutoCompleteFilterMode::EqualsOrdinal,
        AutoCompleteFilterMode::StartsWithOrdinal,
        AutoCompleteFilterMode::ContainsOrdinal,
    ] {
        let filter = get_not_null_filter(mode);
        assert!(filter("\u{00E9}", "\u{00C9}"), "{mode:?}");
        // The sharp s is not "SS".
        assert!(!filter("SS", "\u{00DF}"), "{mode:?}");
        assert!(!filter("\u{00DF}", "SS"), "{mode:?}");
        assert!(filter("\u{00DF}", "\u{00DF}"), "{mode:?}");
        // The dotless i and the dotted capital I are not the Latin letters.
        assert!(!filter("I", "\u{0131}"), "{mode:?}");
        assert!(!filter("i", "\u{0130}"), "{mode:?}");
        assert!(filter("I", "i"), "{mode:?}");
        // Characters outside of the basic plane.
        assert!(filter("\u{1F600}", "\u{1F600}"), "{mode:?}");
        assert!(!filter("\u{1F600}", "\u{1F601}"), "{mode:?}");
    }

    let case_sensitive = get_not_null_filter(AutoCompleteFilterMode::EqualsOrdinalCaseSensitive);
    assert!(!case_sensitive("\u{00E9}", "\u{00C9}"));
}

/// Overrides of the adapter that count the calls; the cancel override does
/// not run the base implementation.
#[derive(Default)]
struct CountingAdapterOverrides {
    commits: Cell<i32>,
    cancels: Cell<i32>,
}

impl SelectingItemsControlSelectionAdapterOverrides for CountingAdapterOverrides {
    fn on_commit(&self, adapter: &SelectingItemsControlSelectionAdapter) {
        self.commits.set(self.commits.get() + 1);
        adapter.base_on_commit();
    }

    fn on_cancel(&self, _adapter: &SelectingItemsControlSelectionAdapter) {
        self.cancels.set(self.cancels.get() + 1);
    }
}

#[test]
fn selection_adapter_commit_and_cancel_can_be_overridden() {
    let _scope = test_scope();

    let overrides = Rc::new(CountingAdapterOverrides::default());
    let list_box = ListBox::new();
    let adapter = SelectingItemsControlSelectionAdapter::with_overrides(Some(list_box.upcast()), overrides.clone());

    let committed = Rc::new(Cell::new(0));
    let canceled = Rc::new(Cell::new(0));
    let count = committed.clone();
    adapter.commit(Rc::new(move |_| count.set(count.get() + 1)));
    let count = canceled.clone();
    adapter.cancel(Rc::new(move |_| count.set(count.get() + 1)));

    let key_down = |key| {
        let mut args = KeyEventArgs::new();
        args.set_routed_event(Some(InputElement::key_down_event()));
        args.key = key;
        adapter.handle_key_down(&args);
        assert!(args.handled());
    };

    // The override runs and calls the base implementation, which raises
    // the event.
    key_down(Key::Enter);
    assert_eq!(1, overrides.commits.get());
    assert_eq!(1, committed.get());

    // The override runs instead of the base implementation.
    key_down(Key::Escape);
    assert_eq!(1, overrides.cancels.get());
    assert_eq!(0, canceled.get());

    // Without overrides the events are raised.
    let plain = SelectingItemsControlSelectionAdapter::new();
    let count = canceled.clone();
    plain.cancel(Rc::new(move |_| count.set(count.get() + 1)));
    plain.on_cancel();
    assert_eq!(1, canceled.get());
}

type PopulationResult = Result<Vec<Option<BoxedValue>>, OperationCanceledError>;

/// A population that completes when the test says so.
#[derive(Default)]
struct PendingPopulation {
    result: RefCell<Option<PopulationResult>>,
    waker: RefCell<Option<Waker>>,
}

impl PendingPopulation {
    fn complete(&self, result: PopulationResult) {
        *self.result.borrow_mut() = Some(result);
        let waker = self.waker.take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

struct PendingPopulationFuture(Rc<PendingPopulation>);

impl Future for PendingPopulationFuture {
    type Output = PopulationResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<PopulationResult> {
        match self.0.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                *self.0.waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// A call of the asynchronous populator of the tests.
struct PopulatorCall {
    search_text: Option<String>,
    cancellation_token: CancellationToken,
    population: Rc<PendingPopulation>,
}

/// Installs an asynchronous populator that records its calls and leaves
/// their completion to the test.
fn install_async_populator(control: &AutoCompleteBox) -> Rc<RefCell<Vec<PopulatorCall>>> {
    let calls: Rc<RefCell<Vec<PopulatorCall>>> = Rc::default();
    let recorded = calls.clone();
    control.set_async_populator(Some(AutoCompleteAsyncPopulator::new(move |search_text, cancellation_token| {
        let population = Rc::new(PendingPopulation::default());
        recorded.borrow_mut().push(PopulatorCall { search_text, cancellation_token, population: population.clone() });
        Box::pin(PendingPopulationFuture(population))
    })));
    calls
}

fn strings(values: &[&str]) -> Vec<Option<BoxedValue>> {
    values.iter().map(|value| Some(Rc::new(value.to_string()) as BoxedValue)).collect()
}

#[test]
fn async_populator_supplies_the_items() {
    run_test(|control, textbox| {
        let calls = install_async_populator(control);
        let populating = Rc::new(Cell::new(false));
        let flag = populating.clone();
        control.populating(move |_| flag.set(true));
        let populated = Rc::new(Cell::new(0));
        let count = populated.clone();
        control.populated(move |_| count.set(count.get() + 1));
        let original_count = control.items_source().expect("the control has items").count();

        textbox.set_text(Some("al"));
        run_jobs();

        // The populator was asked; nothing happens until it answers.
        assert_eq!(1, calls.borrow().len());
        assert_eq!(Some("al"), calls.borrow()[0].search_text.as_deref());
        assert!(!calls.borrow()[0].cancellation_token.is_cancellation_requested());
        assert_eq!(Some("al"), control.search_text().as_deref());
        assert!(!control.is_drop_down_open());
        assert_eq!(0, populated.get());
        assert_eq!(original_count, control.items_source().expect("the control has items").count());

        let population = calls.borrow()[0].population.clone();
        population.complete(Ok(strings(&["alpha", "beta", "alto"])));
        run_jobs();

        // The answer is the items source, filtered into the drop-down.
        assert_eq!(3, control.items_source().expect("the control has items").count());
        assert_eq!(1, populated.get());
        assert!(control.is_drop_down_open());
        // The populating event is not raised for an asynchronous population.
        assert!(!populating.get());
    });
}

#[test]
fn async_population_is_cancelled_when_the_text_changes_again() {
    run_test(|control, textbox| {
        let calls = install_async_populator(control);
        let populated = Rc::new(Cell::new(0));
        let count = populated.clone();
        control.populated(move |_| count.set(count.get() + 1));
        let original_count = control.items_source().expect("the control has items").count();

        textbox.set_text(Some("a"));
        run_jobs();
        textbox.set_text(Some("al"));
        run_jobs();

        // The second request cancelled the first.
        assert_eq!(2, calls.borrow().len());
        assert_eq!(Some("a"), calls.borrow()[0].search_text.as_deref());
        assert_eq!(Some("al"), calls.borrow()[1].search_text.as_deref());
        assert!(calls.borrow()[0].cancellation_token.is_cancellation_requested());
        assert!(!calls.borrow()[1].cancellation_token.is_cancellation_requested());

        // The answer to the cancelled request is dropped.
        let first = calls.borrow()[0].population.clone();
        first.complete(Ok(strings(&["stale"])));
        run_jobs();
        assert_eq!(original_count, control.items_source().expect("the control has items").count());
        assert_eq!(0, populated.get());
        assert!(!control.is_drop_down_open());

        // The answer to the current request is used.
        let second = calls.borrow()[1].population.clone();
        second.complete(Ok(strings(&["alpha", "alto"])));
        run_jobs();
        assert_eq!(2, control.items_source().expect("the control has items").count());
        assert_eq!(1, populated.get());
        assert!(control.is_drop_down_open());
    });
}

#[test]
fn async_population_that_reports_cancellation_changes_nothing() {
    run_test(|control, textbox| {
        let calls = install_async_populator(control);
        let original_count = control.items_source().expect("the control has items").count();

        textbox.set_text(Some("a"));
        run_jobs();

        let population = calls.borrow()[0].population.clone();
        population.complete(Err(OperationCanceledError));
        run_jobs();

        assert_eq!(original_count, control.items_source().expect("the control has items").count());
        assert!(!control.is_drop_down_open());
    });
}

#[test]
fn minimum_populate_delay_defers_the_population_to_the_timer() {
    run_test(|control, textbox| {
        let populated = Rc::new(Cell::new(0));
        let count = populated.clone();
        control.populated(move |_| count.set(count.get() + 1));
        let timers_before = Dispatcher::timers_for_unit_tests().len();

        control.set_minimum_populate_delay(Duration::from_millis(250));

        textbox.set_text(Some("a"));
        run_jobs();

        // The text is taken over, the population waits for the timer.
        assert_eq!(Some("a"), control.text().as_deref());
        assert_eq!(Some(""), control.search_text().as_deref());
        assert_eq!(0, populated.get());
        assert!(!control.is_drop_down_open());

        let timers = Dispatcher::timers_for_unit_tests();
        assert_eq!(timers_before + 1, timers.len());
        let timer = timers
            .iter()
            .find(|timer| timer.interval() == Duration::from_millis(250))
            .expect("the delay timer is scheduled")
            .clone();
        Dispatcher::force_fire_timer_for_unit_tests(&timer);

        assert_eq!(Some("a"), control.search_text().as_deref());
        assert_eq!(1, populated.get());
        assert!(control.is_drop_down_open());
        // The tick stopped the timer.
        assert!(!timer.is_enabled());
        assert_eq!(timers_before, Dispatcher::timers_for_unit_tests().len());

        // Without a delay the population is immediate again.
        control.set_minimum_populate_delay(Duration::ZERO);
        textbox.set_text(Some("ac"));
        run_jobs();
        assert_eq!(Some("ac"), control.search_text().as_deref());
        assert_eq!(2, populated.get());
        assert_eq!(timers_before, Dispatcher::timers_for_unit_tests().len());
    });
}

/// An item with a name, for the tests of the value member binding.
struct NamedItem {
    name: String,
}

ferro_model!(NamedItem, |b| b.read_only::<ferroui_base::data::core::Value<String>>("Name", |x| x.name.clone()));

fn named_item(name: &str) -> Option<BoxedValue> {
    Some(Model::new_model(NamedItem { name: name.to_string() }) as BoxedValue)
}

fn name_binding() -> Option<AssignedBinding> {
    let binding: Rc<dyn BindingBase> = ReflectionBinding::new("Name");
    Some(AssignedBinding::new(binding))
}

#[test]
fn value_member_binding_gives_the_text_of_the_items() {
    run_test(|control, textbox| {
        let items = [named_item("alpha"), named_item("beta"), named_item("bell")];
        control.set_items_source(Some(ItemsSource::from_items(items.clone())));

        assert!(control.item_template().is_none());
        control.set_value_member_binding(name_binding());
        // The binding also gives the template of the items.
        let template = control.item_template();
        assert!(template.is_some());

        // The filter runs on the bound values.
        let view_count = Rc::new(Cell::new(0));
        let count = view_count.clone();
        control.populated(move |e| count.set(e.data().count()));
        textbox.set_text(Some("be"));
        run_jobs();
        assert_eq!(2, view_count.get());
        assert!(control.is_drop_down_open());

        // The text of a selected item is its bound value.
        control.set_selected_item(items[1].clone());
        assert_eq!(Some("beta"), control.text().as_deref());
        assert_eq!(Some("beta"), textbox.text().as_deref());

        // A text equal to a bound value selects the item.
        control.set_text(Some("bell"));
        let selected = control.selected_item().expect("an item is selected");
        assert!(Rc::ptr_eq(&selected, items[2].as_ref().expect("the item is not null")));

        // A new binding replaces the template made from the old one; a
        // template that was set otherwise is kept.
        control.set_value_member_binding(name_binding());
        let replaced = control.item_template().expect("the binding gives a template");
        assert!(!std::ptr::addr_eq(Rc::as_ptr(&replaced), Rc::as_ptr(template.as_ref().expect("checked above"))));
        let custom: Rc<dyn IDataTemplate> = FuncDataTemplate::new(|_| true, |_, _| None, false);
        control.set_item_template(Some(custom.clone()));
        control.set_value_member_binding(name_binding());
        let kept = control.item_template().expect("the template is set");
        assert!(std::ptr::addr_eq(Rc::as_ptr(&kept), Rc::as_ptr(&custom)));
    });
}

// --- additional tests (not in the reference): comparison edge cases and the
// --- delayed asynchronous population -------------------------------------------

/// Pairs of texts and whether the ordinal comparison that ignores case of the
/// reference runtime (version 10.0.5, where every row was run) treats them as
/// equal: it folds with the simple upper-case mapping, character by
/// character, without expansions and without a culture, and folds a
/// surrogate pair as the character it encodes.
const ORDINAL_IGNORE_CASE_ROWS: &[(&str, &str, bool)] = &[
    ("\u{00E9}", "\u{00C9}", true),
    // No expansions.
    ("\u{00DF}", "SS", false),
    ("\u{00DF}", "ss", false),
    ("\u{00DF}", "\u{1E9E}", false),
    ("\u{FB01}", "FI", false),
    ("\u{0149}", "\u{02BC}N", false),
    // No culture: the dotless i and the dotted capital I are not the Latin
    // letters, and neither are the long s and the Kelvin sign.
    ("\u{0131}", "I", false),
    ("\u{0131}", "i", false),
    ("\u{0130}", "i", false),
    ("\u{0130}", "I", false),
    ("\u{017F}", "S", false),
    ("\u{017F}", "s", false),
    ("\u{212A}", "k", false),
    // The simple mapping.
    ("\u{00B5}", "\u{03BC}", true),
    ("\u{03C2}", "\u{03C3}", true),
    ("\u{01C5}", "\u{01C6}", true),
    ("\u{1F80}", "\u{1F88}", true),
    ("\u{1F80}", "\u{1F08}\u{0399}", false),
    ("\u{A7CF}", "\u{A7CE}", true),
    // Characters outside of the basic plane.
    ("\u{10428}", "\u{10400}", true),
    ("\u{1E922}", "\u{1E900}", true),
    ("\u{1F600}", "\u{1F600}", true),
    ("\u{1F600}", "\u{1F601}", false),
];

/// Additional test (not a port).
#[test]
fn ordinal_filters_match_the_reference_runtime_for_edge_characters() {
    let _scope = test_scope();

    let equals = get_not_null_filter(AutoCompleteFilterMode::EqualsOrdinal);
    let starts_with = get_not_null_filter(AutoCompleteFilterMode::StartsWithOrdinal);
    let contains = get_not_null_filter(AutoCompleteFilterMode::ContainsOrdinal);
    let equals_case_sensitive = get_not_null_filter(AutoCompleteFilterMode::EqualsOrdinalCaseSensitive);
    let starts_with_case_sensitive = get_not_null_filter(AutoCompleteFilterMode::StartsWithOrdinalCaseSensitive);
    let contains_case_sensitive = get_not_null_filter(AutoCompleteFilterMode::ContainsOrdinalCaseSensitive);

    for (a, b, equal) in ORDINAL_IGNORE_CASE_ROWS {
        for (search, value) in [(*a, *b), (*b, *a)] {
            assert_eq!(*equal, equals(search, value), "{search:?} equals {value:?}");
            assert_eq!(*equal, starts_with(search, value), "{value:?} starts with {search:?}");
            assert_eq!(*equal, contains(search, value), "{value:?} contains {search:?}");

            // The value as the start and in the middle of a longer text.
            let prefixed = format!("{value}--");
            assert!(!equals(search, &prefixed), "{search:?} equals {prefixed:?}");
            assert_eq!(*equal, starts_with(search, &prefixed), "{prefixed:?} starts with {search:?}");
            assert_eq!(*equal, contains(search, &prefixed), "{prefixed:?} contains {search:?}");
            let embedded = format!("-{value}-");
            assert!(!starts_with(search, &embedded), "{embedded:?} starts with {search:?}");
            assert_eq!(*equal, contains(search, &embedded), "{embedded:?} contains {search:?}");

            // The case-sensitive ordinal modes compare the texts as they are.
            let same = search == value;
            assert_eq!(same, equals_case_sensitive(search, value), "{search:?} {value:?}");
            assert_eq!(same, starts_with_case_sensitive(search, value), "{search:?} {value:?}");
            assert_eq!(same, contains_case_sensitive(search, value), "{search:?} {value:?}");
        }
    }
}

/// Compare rules of a test culture: digits are all equal, and case is
/// ignored when the options say so. They record the options they are asked
/// with.
struct DigitsAreEqualRules {
    options: RefCell<Vec<CompareOptions>>,
}

impl DigitsAreEqualRules {
    fn key(&self, text: &str, options: CompareOptions) -> Vec<char> {
        self.options.borrow_mut().push(options);
        text.chars()
            .map(|c| if c.is_ascii_digit() { '#' } else { c })
            .map(|c| if options.intersects(CompareOptions::IGNORE_CASE) { c.to_ascii_lowercase() } else { c })
            .collect()
    }
}

impl ICompareRules for DigitsAreEqualRules {
    fn compare(&self, a: &str, b: &str, options: CompareOptions) -> std::cmp::Ordering {
        self.key(a, options).cmp(&self.key(b, options))
    }

    fn is_prefix(&self, source: &str, prefix: &str, options: CompareOptions) -> bool {
        self.key(source, options).starts_with(&self.key(prefix, options))
    }

    fn index_of(&self, source: &str, value: &str, options: CompareOptions) -> i32 {
        let (source, value) = (self.key(source, options), self.key(value, options));
        if value.is_empty() {
            return 0;
        }
        source.windows(value.len()).position(|window| window == value.as_slice()).map_or(-1, |index| index as i32)
    }
}

struct TestCultureRulesProvider(Rc<DigitsAreEqualRules>);

impl ICultureDataProvider for TestCultureRulesProvider {
    fn get_compare_rules(&self, culture_name: &str) -> Option<Rc<dyn ICompareRules>> {
        (culture_name == "xx").then(|| self.0.clone() as Rc<dyn ICompareRules>)
    }
}

/// Additional test (not a port): the filter modes that are not ordinal
/// compare with the rules of the current culture, the ordinal ones never do.
#[test]
fn culture_filters_use_the_compare_rules_of_the_current_culture() {
    use AutoCompleteFilterMode::*;

    let _scope = test_scope();
    let locator_scope = FerroLocator::enter_scope();
    let rules = Rc::new(DigitsAreEqualRules { options: RefCell::new(Vec::new()) });
    FerroLocator::current_mutable()
        .bind::<dyn ICultureDataProvider>()
        .to_constant(Rc::new(TestCultureRulesProvider(rules.clone())));

    // (mode, search text, value, with the rules of the culture, without).
    let rows: &[(AutoCompleteFilterMode, &str, &str, bool, bool)] = &[
        (StartsWith, "a1", "A2b", true, false),
        (StartsWithCaseSensitive, "a1", "A2b", false, false),
        (StartsWithCaseSensitive, "A1", "A2b", true, false),
        (Contains, "b1", "aB2c", true, false),
        (ContainsCaseSensitive, "b1", "aB2c", false, false),
        (ContainsCaseSensitive, "B1", "aB2c", true, false),
        (Equals, "a1", "A2", true, false),
        (EqualsCaseSensitive, "a1", "A2", false, false),
        (EqualsCaseSensitive, "A1", "A2", true, false),
        // Without the rules of a culture the built-in comparison applies.
        (StartsWith, "a1", "A1b", true, true),
        (StartsWithCaseSensitive, "a1", "A1b", false, false),
        // The ordinal modes do not reach the rules of the culture.
        (StartsWithOrdinal, "a1", "A2b", false, false),
        (StartsWithOrdinalCaseSensitive, "A1", "A2b", false, false),
        (ContainsOrdinal, "b1", "aB2c", false, false),
        (ContainsOrdinalCaseSensitive, "B1", "aB2c", false, false),
        (EqualsOrdinal, "a1", "A2", false, false),
        (EqualsOrdinalCaseSensitive, "A1", "A2", false, false),
    ];

    let previous = CultureInfo::current_culture();

    // The culture is asked when the filter runs, not when it is created;
    // the rules of the parent culture apply to a culture without rules.
    let filters: Vec<_> = rows.iter().map(|row| get_not_null_filter(row.0)).collect();
    CultureInfo::set_current_culture(CultureInfo::get_culture_info("xx-YY"));
    for (filter, (mode, search, value, expected, _)) in filters.iter().zip(rows) {
        rules.options.borrow_mut().clear();
        assert_eq!(*expected, filter(search, value), "{mode:?} {search:?} {value:?} with the rules");

        let options = rules.options.borrow();
        let mode_name = format!("{mode:?}");
        if mode_name.contains("Ordinal") {
            assert!(options.is_empty(), "{mode:?} asked the culture");
        } else {
            let ignore_case = !mode_name.ends_with("CaseSensitive");
            assert!(!options.is_empty(), "{mode:?} did not ask the culture");
            assert!(
                options.iter().all(|options| options.intersects(CompareOptions::IGNORE_CASE) == ignore_case),
                "{mode:?}"
            );
        }
    }

    CultureInfo::set_current_culture(CultureInfo::get_culture_info("zz"));
    for (filter, (mode, search, value, _, expected)) in filters.iter().zip(rows) {
        rules.options.borrow_mut().clear();
        assert_eq!(*expected, filter(search, value), "{mode:?} {search:?} {value:?} without the rules");
        assert!(rules.options.borrow().is_empty(), "{mode:?}");
    }

    CultureInfo::set_current_culture(previous);
    locator_scope.dispose();
}

/// The delay timers of the dispatcher with the interval of the tests.
fn delay_timers() -> Vec<Rc<ferroui_base::threading::DispatcherTimer>> {
    Dispatcher::timers_for_unit_tests()
        .iter()
        .filter(|timer| timer.interval() == Duration::from_millis(250))
        .cloned()
        .collect()
}

/// Additional test (not a port): every text change restarts the one delay
/// timer; the population (and with it the cancellation of the previous
/// asynchronous one) happens when the timer fires, with the text of then.
#[test]
fn minimum_populate_delay_delays_the_asynchronous_population_and_its_cancellation() {
    run_test(|control, textbox| {
        let calls = install_async_populator(control);
        let populated = Rc::new(Cell::new(0));
        let count = populated.clone();
        control.populated(move |_| count.set(count.get() + 1));
        control.set_minimum_populate_delay(Duration::from_millis(250));
        assert!(delay_timers().is_empty());

        textbox.set_text(Some("a"));
        run_jobs();
        textbox.set_text(Some("al"));
        run_jobs();

        // One timer runs; the populator has not been asked.
        let timers = delay_timers();
        assert_eq!(1, timers.len());
        assert!(timers[0].is_enabled());
        assert!(calls.borrow().is_empty());
        assert_eq!(Some(""), control.search_text().as_deref());

        Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);

        // The populator is asked once, for the text of now.
        assert_eq!(1, calls.borrow().len());
        assert_eq!(Some("al"), calls.borrow()[0].search_text.as_deref());
        assert_eq!(Some("al"), control.search_text().as_deref());
        assert!(delay_timers().is_empty());

        // A further change waits for the timer again and leaves the pending
        // population alone until then.
        textbox.set_text(Some("alp"));
        run_jobs();
        assert_eq!(1, calls.borrow().len());
        assert!(!calls.borrow()[0].cancellation_token.is_cancellation_requested());
        let timers = delay_timers();
        assert_eq!(1, timers.len());

        Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);

        assert_eq!(2, calls.borrow().len());
        assert_eq!(Some("alp"), calls.borrow()[1].search_text.as_deref());
        assert!(calls.borrow()[0].cancellation_token.is_cancellation_requested());
        assert!(!calls.borrow()[1].cancellation_token.is_cancellation_requested());

        // Only the answer to the current request is used.
        let first = calls.borrow()[0].population.clone();
        first.complete(Ok(strings(&["stale"])));
        run_jobs();
        assert_eq!(0, populated.get());
        let second = calls.borrow()[1].population.clone();
        second.complete(Ok(strings(&["alpha", "alpine"])));
        run_jobs();
        assert_eq!(1, populated.get());
        assert_eq!(2, control.items_source().expect("the control has items").count());
        assert!(control.is_drop_down_open());

        // Removing the delay stops and drops the timer: a change populates
        // at once.
        textbox.set_text(Some("alpi"));
        run_jobs();
        assert_eq!(1, delay_timers().len());
        control.set_minimum_populate_delay(Duration::ZERO);
        assert!(delay_timers().is_empty());
        assert_eq!(2, calls.borrow().len());
        textbox.set_text(Some("alpin"));
        run_jobs();
        assert_eq!(3, calls.borrow().len());
        assert_eq!(Some("alpin"), calls.borrow()[2].search_text.as_deref());
    });
}

/// Additional test (not a port): without a value member binding the text of
/// an item is its text form, and resetting the binding restores that.
#[test]
fn value_member_binding_can_be_reset() {
    run_test(|control, textbox| {
        let items = [named_item("alpha"), named_item("beta")];
        control.set_items_source(Some(ItemsSource::from_items(items.clone())));
        let view_count = Rc::new(Cell::new(usize::MAX));
        let count = view_count.clone();
        control.populated(move |e| count.set(e.data().count()));

        control.set_value_member_binding(name_binding());
        assert!(control.value_member_binding().is_some());
        textbox.set_text(Some("be"));
        run_jobs();
        assert_eq!(1, view_count.get());

        // Without the binding the names are not what the filter sees.
        control.set_value_member_binding(None);
        assert!(control.value_member_binding().is_none());
        textbox.set_text(Some("bet"));
        run_jobs();
        assert_eq!(0, view_count.get());
        assert!(!control.is_drop_down_open());
    });
}

/// Additional test (not a port): a typed list of strings boxes its items anew
/// on every read, where the lists of the reference hand out the same string
/// instances. Populating again with the same match must not look like another
/// selected object: the reference then leaves its "skip the text update" flag
/// clear, raises no selection change, and the next selection updates the text.
#[test]
fn repopulating_a_typed_string_list_keeps_the_selected_item_and_the_next_selection_updates_the_text() {
    fn typed_items() -> ItemsSource {
        let items = Rc::new(ferroui_base::collections::FerroList::<String>::new());
        items.add_range(["able", "abide", "about"].map(str::to_string));
        ItemsSource::from(items)
    }

    run_test(|control, textbox| {
        control.set_items_source(Some(typed_items()));

        textbox.set_text(Some("able"));
        run_jobs();
        assert_eq!("able", text_of(&control.selected_item()));
        let selected = control.selected_item().expect("the exact match is selected");

        let selection_changes = Rc::new(Cell::new(0));
        let count = selection_changes.clone();
        control.selection_changed(move |_, _| count.set(count.get() + 1));

        // The same strings again, in new boxes, and another population.
        control.set_items_source(Some(typed_items()));
        control.populate_complete();

        assert_eq!(0, selection_changes.get());
        assert_eq!(text_of(&Some(selected)), text_of(&control.selected_item()));
        assert_eq!(Some("able"), control.text().as_deref());

        // The next selection updates the text.
        let abide: BoxedValue = Rc::new("abide".to_string());
        control.set_selected_item(Some(abide));

        assert_eq!(1, selection_changes.get());
        assert_eq!(Some("abide"), control.text().as_deref());
        assert_eq!(Some("abide"), textbox.text().as_deref());
    });
}
