//! Tests of `Frame`, `FramePage` and the navigation model (WinUI; docs/porting/winui-controls.md,
//! section 10.4), adapted from #286.

use crate::navigation::{
    FrameNavigationOptions, NavigatingCancelEventArgs, NavigationCacheMode, NavigationError, NavigationEventArgs,
    NavigationMode, PageStackEntry,
};
use crate::primitives::TemplatedControlImpl;
use crate::testing::TestLogSink;
use crate::{Button, ContentControlImpl, ControlImpl, Frame, FramePage, FramePageImpl, PageType};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, TypeInfo, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    static EVENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static CANCEL_PAGE: Cell<bool> = const { Cell::new(false) };
}

fn record(value: impl Into<String>) {
    EVENTS.with(|events| events.borrow_mut().push(value.into()));
}
fn events() -> Vec<String> {
    EVENTS.with(|events| events.borrow_mut().drain(..).collect())
}

macro_rules! page_type {
    ($name:ident) => {
        #[repr(C)]
        struct $name { base: FramePage }
        ferro_class!($name: FramePage);
        ferro_class_info!($name { new: $name::new });
        ferro_impl_classes!($name: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl,
            InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);
        impl FramePageImpl for $name {
            fn on_navigating_from(_this: &Self, args: &NavigatingCancelEventArgs) {
                record(concat!(stringify!($name), ".NavigatingFrom"));
                if CANCEL_PAGE.with(Cell::get) { args.set_cancel(true); }
            }
            fn on_navigated_from(_this: &Self, _args: &NavigationEventArgs) { record(concat!(stringify!($name), ".NavigatedFrom")); }
            fn on_navigated_to(_this: &Self, args: &NavigationEventArgs) {
                assert!(args.content().is_some());
                record(concat!(stringify!($name), ".NavigatedTo"));
            }
        }
        impl $name {
            fn new() -> Ref<Self> { instantiate(Self { base: FramePage::construct() }) }
        }
    };
}

page_type!(FirstPage);
page_type!(SecondPage);
page_type!(ThirdPage);

/// The page types of the test pages, made in a constant context.
const FIRST: PageType = PageType::of::<FirstPage>();
const SECOND: PageType = PageType::of::<SecondPage>();
const THIRD: PageType = PageType::of::<ThirdPage>();

/// A page class without a default constructor: navigating to it is possible only through its
/// runtime type, which has no constructor to create it with.
#[allow(dead_code)]
#[repr(C)]
struct NoConstructorPage {
    base: FramePage,
}
ferro_class!(NoConstructorPage: FramePage);
ferro_impl_classes!(NoConstructorPage: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl,
    InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);
impl FramePageImpl for NoConstructorPage {
    fn on_navigating_from(_this: &Self, _args: &NavigatingCancelEventArgs) {}
    fn on_navigated_from(_this: &Self, _args: &NavigationEventArgs) {}
    fn on_navigated_to(_this: &Self, _args: &NavigationEventArgs) {}
}

fn frame() -> Ref<Frame> {
    crate::register_types();
    events();
    CANCEL_PAGE.with(|cancel| cancel.set(false));
    Frame::new()
}

fn factory(
    frame: &Frame,
    ty: PageType,
    mode: NavigationCacheMode,
    count: Rc<Cell<usize>>,
) {
    frame
        .register_page_factory(
            ty,
            Rc::new(move || {
                count.set(count.get() + 1);
                let page = ty.create()?;
                page.set_navigation_cache_mode(mode);
                Ok(page)
            }),
        )
        .unwrap();
}

#[test]
fn defaults_and_existing_page_type_remain_distinct() {
    let frame = frame();
    assert_eq!(frame.cache_size(), 10);
    assert!(frame.is_navigation_stack_enabled());
    assert!(!frame.can_go_back());
    assert!(!frame.can_go_forward());
    assert!(frame.source_page_type().is_none());
    assert_eq!(
        FramePage::new().navigation_cache_mode(),
        NavigationCacheMode::Disabled
    );
    assert_ne!(FramePage::TYPE, crate::Page::TYPE);
    assert_eq!(FramePage::TYPE.full_name(), "FerroUI.Controls.FramePage");
    assert_eq!(
        crate::Page::TYPE.full_name(),
        "FerroUI.Controls.Page"
    );
}

#[test]
fn virtual_callbacks_follow_frame_event_order_and_parameters_survive_history() {
    let frame = frame();
    frame.navigating(|args| {
        assert!(!args.cancel());
        record("Frame.Navigating");
    }).detach();
    frame.navigated(|_| record("Frame.Navigated")).detach();
    let parameter: BoxedValue = Rc::new("first parameter".to_string());
    assert!(frame
        .navigate(FIRST, Some(parameter.clone()))
        .unwrap());
    assert_eq!(
        events(),
        [
            "Frame.Navigating",
            "Frame.Navigated",
            "FirstPage.NavigatedTo"
        ]
    );
    assert!(frame.navigate(SECOND, None).unwrap());
    assert_eq!(
        events(),
        [
            "Frame.Navigating",
            "FirstPage.NavigatingFrom",
            "Frame.Navigated",
            "FirstPage.NavigatedFrom",
            "SecondPage.NavigatedTo"
        ]
    );
    assert_eq!(frame.back_stack_depth(), 1);
    assert_eq!(
        frame.back_stack().get(0).unwrap().source_page_type(),
        FIRST
    );
    assert!(Rc::ptr_eq(
        &frame.back_stack().get(0).unwrap().parameter().unwrap(),
        &parameter
    ));
    frame.go_back().unwrap();
    assert_eq!(frame.current_source_page_type(), Some(FIRST));
    assert!(!frame.can_go_back());
    assert!(frame.can_go_forward());
    assert_eq!(
        frame.forward_stack().get(0).unwrap().source_page_type(),
        SECOND
    );
    frame.go_forward().unwrap();
    assert_eq!(frame.current_source_page_type(), Some(SECOND));
    frame.go_back().unwrap();
    frame.navigate(THIRD, None).unwrap();
    assert!(!frame.can_go_forward());
}

#[test]
fn frame_cancellation_stops_before_page_callback_and_accepts_request() {
    let frame = frame();
    frame.navigate(FIRST, None).unwrap();
    let old = frame.current_page().unwrap();
    events();
    frame.navigating(|args| {
        record("Frame.Cancel");
        let cloned = args.clone();
        cloned.set_cancel(true);
    }).detach();
    frame.navigation_stopped(|args| {
        assert!(args.content().is_none());
        record("Frame.Stopped");
    }).detach();
    assert!(frame.navigate(SECOND, None).unwrap());
    assert_eq!(frame.current_page(), Some(old));
    assert_eq!(frame.back_stack_depth(), 0);
    assert_eq!(events(), ["Frame.Cancel", "Frame.Stopped"]);
}

#[test]
fn page_cancellation_is_virtual_and_does_not_commit_history() {
    let frame = frame();
    frame.navigate(FIRST, None).unwrap();
    events();
    CANCEL_PAGE.with(|cancel| cancel.set(true));
    frame.navigation_stopped(|_| record("Stopped")).detach();
    assert!(frame.navigate(SECOND, None).unwrap());
    assert_eq!(events(), ["FirstPage.NavigatingFrom", "Stopped"]);
    assert_eq!(frame.current_source_page_type(), Some(FIRST));
    assert_eq!(frame.back_stack_depth(), 0);
}

#[test]
fn navigation_is_not_reentrant_and_history_edits_are_rejected_while_pending() {
    let frame = frame();
    let weak = frame.downgrade();
    frame.navigating(move |_| {
        let frame = weak.upgrade().unwrap();
        assert!(!frame.navigate(THIRD, None).unwrap());
        assert!(frame.go_back().is_ok());
        assert_eq!(
            frame.back_stack().clear(),
            Err(NavigationError::NavigationPending)
        );
        assert_eq!(
            frame
                .back_stack()
                .add(PageStackEntry::new(THIRD, None)),
            Err(NavigationError::NavigationPending)
        );
    }).detach();
    let weak = frame.downgrade();
    frame.navigated(move |_| {
        let frame = weak.upgrade().unwrap();
        assert!(!frame.navigate(THIRD, None).unwrap());
        // Journal edits are allowed after its commit, even though nested navigation is not.
        frame.forward_stack().clear().unwrap();
    }).detach();
    frame.navigate(FIRST, None).unwrap();
    frame.navigate(SECOND, None).unwrap();
    assert_eq!(frame.current_source_page_type(), Some(SECOND));
    assert_eq!(frame.back_stack_depth(), 1);
}

#[test]
fn history_handles_are_live_observable_and_mutations_update_properties() {
    let frame = frame();
    let stack = frame.back_stack();
    assert!(stack == frame.back_stack());
    let changes = Rc::new(Cell::new(0));
    let count = changes.clone();
    stack.add_collection_changed(Rc::new(move |_| count.set(count.get() + 1)));
    stack
        .add(PageStackEntry::new(FIRST, None))
        .unwrap();
    assert!(frame.can_go_back());
    assert_eq!(frame.back_stack_depth(), 1);
    assert_eq!(changes.get(), 1);
    frame.go_back().unwrap();
    assert!(stack.is_empty());
    assert!(!frame.can_go_back());
    assert_eq!(frame.back_stack_depth(), 0);
    assert_eq!(changes.get(), 2);
    assert!(matches!(
        stack.remove_at(10),
        Err(NavigationError::IndexOutOfRange { .. })
    ));
}

#[test]
fn a_journal_is_a_live_items_source_and_bindings_keep_its_mutation_guards() {
    use ferroui_base::data::core::{ValueType, ValueTypes};
    use crate::{navigation::NavigationStack, ItemsSource};
    let frame = frame();
    let stack = frame.back_stack();
    let source: ItemsSource = stack.clone().into();
    assert_eq!(source.count(), 0);
    assert!(source.list().as_any().is::<NavigationStack>());
    stack
        .add(PageStackEntry::new(FIRST, None))
        .unwrap();
    assert_eq!(source.count(), 1);
    let boxed: BoxedValue = Rc::new(stack);
    let converted = ValueTypes::try_convert(Some(&boxed), ValueType::of::<ItemsSource>())
        .flatten()
        .unwrap();
    assert_eq!(*converted.downcast_ref::<ItemsSource>().unwrap(), source);
}

#[test]
fn history_entries_cannot_belong_to_two_frames_and_removed_entries_can_move() {
    let a = frame();
    let b = Frame::new();
    let entry = PageStackEntry::new(FIRST, None);
    a.back_stack().add(entry.clone()).unwrap();
    assert_eq!(
        b.back_stack().add(entry.clone()),
        Err(NavigationError::EntryOwnedByAnotherFrame)
    );
    a.back_stack().clear().unwrap();
    b.back_stack().add(entry).unwrap();
    assert_eq!(b.back_stack_depth(), 1);
}

#[test]
fn property_handlers_can_edit_history_without_overwriting_the_newer_state() {
    let frame = frame();
    let weak = frame.downgrade();
    frame.property_changed(move |change| {
        if change.property() == Frame::can_go_back_property().as_property() {
            let frame = weak.upgrade().unwrap();
            if frame.can_go_back() {
                assert_eq!(frame.back_stack_depth(), 1);
                frame.back_stack().clear().unwrap();
            }
        }
    });
    frame
        .back_stack()
        .add(PageStackEntry::new(FIRST, None))
        .unwrap();
    assert!(frame.back_stack().is_empty());
    assert!(!frame.can_go_back());
    assert_eq!(frame.back_stack_depth(), 0);
}

#[test]
fn setting_the_same_source_type_does_not_navigate_twice() {
    let frame = frame();
    frame.set_source_page_type(Some(FIRST)).unwrap();
    let page = frame.current_page();
    frame.set_source_page_type(Some(FIRST)).unwrap();
    assert_eq!(frame.current_page(), page);
    assert_eq!(frame.back_stack_depth(), 0);
}

#[test]
fn disabled_cache_constructs_each_visit_and_enabled_cache_is_per_type() {
    let frame = frame();
    let count = Rc::new(Cell::new(0));
    factory(
        &frame,
        FIRST,
        NavigationCacheMode::Disabled,
        count.clone(),
    );
    frame.navigate(FIRST, None).unwrap();
    let first = frame.current_page().unwrap();
    frame.navigate(SECOND, None).unwrap();
    frame.go_back().unwrap();
    assert_ne!(frame.current_page(), Some(first));
    assert_eq!(count.get(), 2);
    factory(
        &frame,
        FIRST,
        NavigationCacheMode::Enabled,
        count.clone(),
    );
    frame
        .navigate(FIRST, Some(Rc::new(1_i32)))
        .unwrap();
    let cached = frame.current_page().unwrap();
    frame
        .navigate(FIRST, Some(Rc::new(2_i32)))
        .unwrap();
    assert_eq!(frame.current_page(), Some(cached));
    assert_eq!(count.get(), 3);
}

#[test]
fn enabled_cache_is_lru_and_required_entries_survive_shrinking() {
    let frame = frame();
    frame.set_cache_size(2).unwrap();
    let count = Rc::new(Cell::new(0));
    for ty in [FIRST, SECOND, THIRD] {
        factory(&frame, ty, NavigationCacheMode::Enabled, count.clone());
    }
    frame.navigate(FIRST, None).unwrap();
    let first = frame.current_page().unwrap();
    frame.navigate(SECOND, None).unwrap();
    let second = frame.current_page().unwrap();
    frame.navigate(FIRST, None).unwrap(); // First is most recently used.
    frame.navigate(THIRD, None).unwrap(); // Second is evicted.
    frame.navigate(FIRST, None).unwrap();
    assert_eq!(frame.current_page(), Some(first));
    frame.navigate(SECOND, None).unwrap();
    assert_ne!(frame.current_page(), Some(second));
    assert_eq!(count.get(), 4);

    frame.set_cache_size(0).unwrap();
    frame.set_cache_size(1).unwrap();
    factory(
        &frame,
        FIRST,
        NavigationCacheMode::Required,
        count.clone(),
    );
    frame.navigate(FIRST, None).unwrap();
    let required = frame.current_page().unwrap();
    frame.set_cache_size(0).unwrap();
    frame.navigate(SECOND, None).unwrap();
    frame.navigate(FIRST, None).unwrap();
    assert_eq!(frame.current_page(), Some(required));
}

#[test]
fn zero_capacity_blocks_new_required_admission_and_disabled_mode_uncaches() {
    let frame = frame();
    frame.set_cache_size(0).unwrap();
    let count = Rc::new(Cell::new(0));
    factory(
        &frame,
        FIRST,
        NavigationCacheMode::Required,
        count.clone(),
    );
    frame.navigate(FIRST, None).unwrap();
    frame.navigate(SECOND, None).unwrap();
    frame.navigate(FIRST, None).unwrap();
    assert_eq!(count.get(), 2);
    frame.set_cache_size(10).unwrap();
    frame.navigate(FIRST, None).unwrap();
    frame
        .current_page()
        .unwrap()
        .set_navigation_cache_mode(NavigationCacheMode::Disabled);
    frame.navigate(FIRST, None).unwrap();
    assert_eq!(count.get(), 4);
}

#[test]
fn failures_preserve_content_history_and_allow_retry() {
    let frame = frame();
    frame.navigate(FIRST, None).unwrap();
    let old = frame.current_page();
    frame
        .register_page_factory(
            SECOND,
            Rc::new(|| Err(NavigationError::FactoryFailed("construction failed".into()))),
        )
        .unwrap();
    assert!(frame.navigate(SECOND, None).is_err());
    assert_eq!(frame.current_page(), old);
    assert_eq!(frame.back_stack_depth(), 0);
    frame.navigation_failed(|args| {
        args.clone().set_handled(true);
    }).detach();
    assert!(!frame.navigate(SECOND, None).unwrap());
    frame.unregister_page_factory(SECOND);
    assert!(frame.navigate(SECOND, None).unwrap());
    assert!(frame.can_go_back());
    assert_eq!(
        frame.set_cache_size(-1),
        Err(NavigationError::InvalidCacheSize(-1))
    );
    assert_eq!(
        frame.set_source_page_type(None),
        Err(NavigationError::MissingSourcePageType)
    );
    assert!(matches!(
        frame.register_page_factory(PageType::from(Button::TYPE), Rc::new(|| Ok(FramePage::new()))),
        Err(NavigationError::NotAPage(_))
    ));
}

#[test]
fn disabled_journal_changes_content_without_committing_history_or_source_type() {
    let frame = frame();
    frame.navigate(FIRST, None).unwrap();
    let options = FrameNavigationOptions::new();
    options.set_is_navigation_stack_enabled(false);
    frame
        .navigate_to_type(SECOND, None, &options)
        .unwrap();
    assert!(SECOND.is_assignable_from(frame.current_page().unwrap().get_type()));
    assert!(frame.current_page().unwrap().is::<SecondPage>());
    assert_eq!(frame.current_source_page_type(), Some(FIRST));
    assert_eq!(frame.back_stack_depth(), 0);
    frame.navigate(THIRD, None).unwrap();
    assert_eq!(
        frame.back_stack().get(0).unwrap().source_page_type(),
        FIRST
    );
    assert_eq!(frame.back_stack_depth(), 1);
}

#[test]
fn page_and_journal_handles_do_not_keep_a_frame_alive() {
    let frame = frame();
    let weak = frame.downgrade();
    frame.navigate(FIRST, None).unwrap();
    let page = frame.current_page().unwrap();
    let stack = frame.back_stack();
    assert_eq!(page.frame(), Some(frame.clone()));
    drop(frame);
    assert!(weak.upgrade().is_none());
    assert!(page.frame().is_none());
    // A detached journal remains a valid observable collection with no dangling owner.
    stack
        .add(PageStackEntry::new(SECOND, None))
        .unwrap();
    assert_eq!(stack.count(), 1);
}

#[test]
fn enum_values_match_winui_and_empty_back_forward_are_errors() {
    assert_eq!(NavigationMode::New as i32, 0);
    assert_eq!(NavigationMode::Back as i32, 1);
    assert_eq!(NavigationMode::Forward as i32, 2);
    assert_eq!(NavigationMode::Refresh as i32, 3);
    assert_eq!(NavigationCacheMode::Disabled as i32, 0);
    assert_eq!(NavigationCacheMode::Required as i32, 1);
    assert_eq!(NavigationCacheMode::Enabled as i32, 2);
    let frame = frame();
    assert_eq!(frame.go_back(), Err(NavigationError::NoBackEntry));
    assert_eq!(frame.go_forward(), Err(NavigationError::NoForwardEntry));
}

/// WinUI reads a negative `CacheSize` as an unsigned number; from markup (an untyped set, as the
/// loader sets it) the port coerces it to 0 and logs a warning, where #286 panicked.
#[test]
fn a_negative_cache_size_from_markup_is_coerced_to_zero_and_logged() {
    let frame = frame();
    let warnings = Rc::new(Cell::new(0));
    let counter = warnings.clone();
    let sink = TestLogSink::start(move |level, _, _, message, _| {
        if level >= LogEventLevel::Warning && message.contains("CacheSize") {
            counter.set(counter.get() + 1);
        }
    });
    frame.set_value_untyped(Frame::cache_size_property().as_property(), &-1_i32, BindingPriority::LocalValue);
    assert_eq!(0, frame.cache_size());
    assert_eq!(1, warnings.get());
    frame.set_value_untyped(Frame::cache_size_property().as_property(), &3_i32, BindingPriority::LocalValue);
    assert_eq!(3, frame.cache_size());
    assert_eq!(1, warnings.get());
    // The Rust setter reports the error and changes nothing.
    assert_eq!(Err(NavigationError::InvalidCacheSize(-5)), frame.set_cache_size(-5));
    assert_eq!(3, frame.cache_size());
    sink.dispose();
}

/// An invalid `SourcePageType` from markup (null, a type that is not a page) is logged and
/// changes nothing, where WinUI's setter fails and #286 panicked.
#[test]
fn an_invalid_source_page_type_from_markup_is_logged() {
    let frame = frame();
    frame.navigate(FIRST, None).unwrap();
    let page = frame.current_page();
    let warnings = Rc::new(Cell::new(0));
    let counter = warnings.clone();
    let sink = TestLogSink::start(move |level, _, _, message, _| {
        if level >= LogEventLevel::Warning && message.contains("SourcePageType") {
            counter.set(counter.get() + 1);
        }
    });
    let none: Option<PageType> = None;
    frame.set_value_untyped(Frame::source_page_type_property().as_property(), &none, BindingPriority::LocalValue);
    let not_a_page: Option<PageType> = Some(PageType::from(Button::TYPE));
    frame.set_value_untyped(Frame::source_page_type_property().as_property(), &not_a_page, BindingPriority::LocalValue);
    assert_eq!(2, warnings.get());
    assert_eq!(frame.current_page(), page);
    assert_eq!(frame.source_page_type(), Some(FIRST));
    // A page type navigates.
    let second: Option<PageType> = Some(SECOND);
    frame.set_value_untyped(Frame::source_page_type_property().as_property(), &second, BindingPriority::LocalValue);
    assert_eq!(frame.source_page_type(), Some(SECOND));
    assert_eq!(2, warnings.get());
    sink.dispose();
}

/// A page type compares and hashes by the identity of its class (B3 of #612): the same class gives
/// equal page types whether it is named statically or converted from its runtime type.
#[test]
fn page_types_compare_and_hash_by_the_identity_of_the_class() {
    use std::collections::HashSet;
    assert_eq!(FIRST, PageType::of::<FirstPage>());
    assert_ne!(FIRST, SECOND);
    assert_eq!(FIRST, PageType::from(FirstPage::TYPE));
    #[allow(deprecated)]
    let deprecated = PageType::from_type_info(SecondPage::TYPE);
    assert_eq!(deprecated, SECOND);
    let set: HashSet<PageType> = [FIRST, PageType::from(FirstPage::TYPE), SECOND, THIRD].into_iter().collect();
    assert_eq!(set.len(), 3);
    assert!(set.contains(&PageType::from(ThirdPage::TYPE)));
    assert_eq!(FIRST.name(), "FirstPage");
    assert_eq!(FIRST.to_string(), "FirstPage");
    assert_eq!(format!("{FIRST:?}"), "PageType(\"FirstPage\")");
    assert!(FIRST.is::<FirstPage>());
    assert!(!FIRST.is::<SecondPage>());
    assert!(std::ptr::eq(FIRST.identity().unwrap(), <FirstPage as ferroui_base::ObjectType>::IDENTITY));
    assert!(FIRST.create().unwrap().is::<FirstPage>());
}

/// The journal, `SourcePageType` and the arguments of the navigation events carry the page type of
/// the navigation.
#[test]
fn the_journal_the_source_page_type_and_the_events_carry_the_page_type() {
    let frame = frame();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let navigating = seen.clone();
    frame.navigating(move |args| navigating.borrow_mut().push(args.source_page_type())).detach();
    let navigated = seen.clone();
    frame.navigated(move |args| navigated.borrow_mut().push(args.source_page_type())).detach();
    frame.navigate(FIRST, None).unwrap();
    frame.navigate(SECOND, None).unwrap();
    assert_eq!(*seen.borrow(), [FIRST, FIRST, SECOND, SECOND]);
    assert_eq!(frame.source_page_type(), Some(SECOND));
    assert_eq!(frame.current_source_page_type(), Some(SECOND));
    assert_eq!(frame.back_stack().get(0).unwrap().source_page_type(), FIRST);
    frame.go_back().unwrap();
    assert_eq!(frame.forward_stack().get(0).unwrap().source_page_type(), SECOND);
    assert_eq!(frame.current_source_page_type(), Some(FIRST));
}

/// The deprecated form of 0.5: a `&'static TypeInfo` converts to a page type that creates the page
/// by reflection, through the default constructor of its runtime type, and markup converts the
/// runtime type `{x:Type}` gives to a page type with the registered cast.
#[test]
fn a_runtime_type_converts_to_a_page_type_that_navigates() {
    use ferroui_base::data::core::{ValueType, ValueTypes};
    use ferroui_base::metadata::from_markup_value;
    let frame = frame();
    let class: &'static TypeInfo = SecondPage::TYPE;
    assert!(frame.navigate(class, None).unwrap());
    assert!(frame.current_page().unwrap().is::<SecondPage>());
    assert_eq!(frame.current_source_page_type(), Some(SECOND));
    frame.set_source_page_type(Some(PageType::from(FirstPage::TYPE))).unwrap();
    assert!(frame.current_page().unwrap().is::<FirstPage>());
    // The conversions of markup: the argument of `Navigate` and the value of `SourcePageType`.
    let boxed: BoxedValue = Rc::new(ThirdPage::TYPE);
    let cast = ValueTypes::try_cast(&boxed, ValueType::of::<PageType>()).unwrap();
    assert_eq!(*cast.downcast_ref::<PageType>().unwrap(), THIRD);
    let nullable = ValueTypes::try_cast(&boxed, ValueType::of::<Option<PageType>>()).unwrap();
    assert_eq!(*nullable.downcast_ref::<Option<PageType>>().unwrap(), Some(THIRD));
    assert_eq!(from_markup_value::<PageType>(&Some(boxed)), Some(THIRD));
}

/// A page type converted from the runtime type of a page without a default constructor fails with
/// `MissingConstructor`, whose message names the static form and the type tables.
#[test]
fn a_runtime_type_without_a_default_constructor_is_a_missing_constructor() {
    let frame = frame();
    let page_type = PageType::from(NoConstructorPage::TYPE);
    let error = frame.navigate(page_type, None).unwrap_err();
    let NavigationError::MissingConstructor(name) = &error else {
        panic!("expected MissingConstructor, got {error:?}");
    };
    assert!(name.ends_with("NoConstructorPage"), "{name}");
    let message = error.to_string();
    assert!(message.contains("PageType::of::<NoConstructorPage>()"), "{message}");
    assert!(message.contains("type-tables"), "{message}");
    assert!(frame.current_page().is_none());
    assert_eq!(frame.back_stack_depth(), 0);
    assert_eq!(page_type.create().err(), Some(error));
}
