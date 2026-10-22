//! Public behavior tests of the native frame navigation contract.

use ferroui_controls::navigation::{
    FrameNavigationOptions, NavigatingCancelEventArgs, NavigationCacheMode, NavigationError, NavigationEventArgs, NavigationMode, PageStackEntry,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Frame, FramePage, FramePageImpl,
};
use ferroui_controls::{Button, ContentControlImpl, ControlImpl};
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

fn frame() -> Ref<Frame> {
    ferroui_controls::register_types();
    events();
    CANCEL_PAGE.with(|cancel| cancel.set(false));
    Frame::new()
}

fn factory(
    frame: &Frame,
    ty: &'static ferroui_base::TypeInfo,
    mode: NavigationCacheMode,
    count: Rc<Cell<usize>>,
) {
    frame
        .register_page_factory(
            ty,
            Rc::new(move || {
                count.set(count.get() + 1);
                let page = ty.create_instance().unwrap().cast::<FramePage>().unwrap();
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
    assert_ne!(FramePage::TYPE, ferroui_controls::Page::TYPE);
    assert_eq!(FramePage::TYPE.full_name(), "FerroUI.Controls.FramePage");
    assert_eq!(
        ferroui_controls::Page::TYPE.full_name(),
        "FerroUI.Controls.Page"
    );
}

#[test]
fn virtual_callbacks_follow_frame_event_order_and_parameters_survive_history() {
    let frame = frame();
    frame.navigating(|args| {
        assert!(!args.cancel());
        record("Frame.Navigating");
    });
    frame.navigated(|_| record("Frame.Navigated"));
    let parameter: BoxedValue = Rc::new("first parameter".to_string());
    assert!(frame
        .navigate(FirstPage::TYPE, Some(parameter.clone()))
        .unwrap());
    assert_eq!(
        events(),
        [
            "Frame.Navigating",
            "Frame.Navigated",
            "FirstPage.NavigatedTo"
        ]
    );
    assert!(frame.navigate(SecondPage::TYPE, None).unwrap());
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
        FirstPage::TYPE
    );
    assert!(Rc::ptr_eq(
        &frame.back_stack().get(0).unwrap().parameter().unwrap(),
        &parameter
    ));
    frame.go_back().unwrap();
    assert_eq!(frame.current_source_page_type(), Some(FirstPage::TYPE));
    assert!(!frame.can_go_back());
    assert!(frame.can_go_forward());
    assert_eq!(
        frame.forward_stack().get(0).unwrap().source_page_type(),
        SecondPage::TYPE
    );
    frame.go_forward().unwrap();
    assert_eq!(frame.current_source_page_type(), Some(SecondPage::TYPE));
    frame.go_back().unwrap();
    frame.navigate(ThirdPage::TYPE, None).unwrap();
    assert!(!frame.can_go_forward());
}

#[test]
fn frame_cancellation_stops_before_page_callback_and_accepts_request() {
    let frame = frame();
    frame.navigate(FirstPage::TYPE, None).unwrap();
    let old = frame.current_page().unwrap();
    events();
    frame.navigating(|args| {
        record("Frame.Cancel");
        let cloned = args.clone();
        cloned.set_cancel(true);
    });
    frame.navigation_stopped(|args| {
        assert!(args.content().is_none());
        record("Frame.Stopped");
    });
    assert!(frame.navigate(SecondPage::TYPE, None).unwrap());
    assert_eq!(frame.current_page(), Some(old));
    assert_eq!(frame.back_stack_depth(), 0);
    assert_eq!(events(), ["Frame.Cancel", "Frame.Stopped"]);
}

#[test]
fn page_cancellation_is_virtual_and_does_not_commit_history() {
    let frame = frame();
    frame.navigate(FirstPage::TYPE, None).unwrap();
    events();
    CANCEL_PAGE.with(|cancel| cancel.set(true));
    frame.navigation_stopped(|_| record("Stopped"));
    assert!(frame.navigate(SecondPage::TYPE, None).unwrap());
    assert_eq!(events(), ["FirstPage.NavigatingFrom", "Stopped"]);
    assert_eq!(frame.current_source_page_type(), Some(FirstPage::TYPE));
    assert_eq!(frame.back_stack_depth(), 0);
}

#[test]
fn navigation_is_not_reentrant_and_history_edits_are_rejected_while_pending() {
    let frame = frame();
    let weak = frame.downgrade();
    frame.navigating(move |_| {
        let frame = weak.upgrade().unwrap();
        assert!(!frame.navigate(ThirdPage::TYPE, None).unwrap());
        assert!(frame.go_back().is_ok());
        assert_eq!(
            frame.back_stack().clear(),
            Err(NavigationError::NavigationPending)
        );
        assert_eq!(
            frame
                .back_stack()
                .add(PageStackEntry::new(ThirdPage::TYPE, None)),
            Err(NavigationError::NavigationPending)
        );
    });
    let weak = frame.downgrade();
    frame.navigated(move |_| {
        let frame = weak.upgrade().unwrap();
        assert!(!frame.navigate(ThirdPage::TYPE, None).unwrap());
        // Journal edits are allowed after its commit, even though nested navigation is not.
        frame.forward_stack().clear().unwrap();
    });
    frame.navigate(FirstPage::TYPE, None).unwrap();
    frame.navigate(SecondPage::TYPE, None).unwrap();
    assert_eq!(frame.current_source_page_type(), Some(SecondPage::TYPE));
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
        .add(PageStackEntry::new(FirstPage::TYPE, None))
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
    use ferroui_controls::{navigation::NavigationStack, ItemsSource};
    let frame = frame();
    let stack = frame.back_stack();
    let source: ItemsSource = stack.clone().into();
    assert_eq!(source.count(), 0);
    assert!(source.list().as_any().is::<NavigationStack>());
    stack
        .add(PageStackEntry::new(FirstPage::TYPE, None))
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
    let entry = PageStackEntry::new(FirstPage::TYPE, None);
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
        .add(PageStackEntry::new(FirstPage::TYPE, None))
        .unwrap();
    assert!(frame.back_stack().is_empty());
    assert!(!frame.can_go_back());
    assert_eq!(frame.back_stack_depth(), 0);
}

#[test]
fn setting_the_same_source_type_does_not_navigate_twice() {
    let frame = frame();
    frame.set_source_page_type(Some(FirstPage::TYPE)).unwrap();
    let page = frame.current_page();
    frame.set_source_page_type(Some(FirstPage::TYPE)).unwrap();
    assert_eq!(frame.current_page(), page);
    assert_eq!(frame.back_stack_depth(), 0);
}

#[test]
fn disabled_cache_constructs_each_visit_and_enabled_cache_is_per_type() {
    let frame = frame();
    let count = Rc::new(Cell::new(0));
    factory(
        &frame,
        FirstPage::TYPE,
        NavigationCacheMode::Disabled,
        count.clone(),
    );
    frame.navigate(FirstPage::TYPE, None).unwrap();
    let first = frame.current_page().unwrap();
    frame.navigate(SecondPage::TYPE, None).unwrap();
    frame.go_back().unwrap();
    assert_ne!(frame.current_page(), Some(first));
    assert_eq!(count.get(), 2);
    factory(
        &frame,
        FirstPage::TYPE,
        NavigationCacheMode::Enabled,
        count.clone(),
    );
    frame
        .navigate(FirstPage::TYPE, Some(Rc::new(1_i32)))
        .unwrap();
    let cached = frame.current_page().unwrap();
    frame
        .navigate(FirstPage::TYPE, Some(Rc::new(2_i32)))
        .unwrap();
    assert_eq!(frame.current_page(), Some(cached));
    assert_eq!(count.get(), 3);
}

#[test]
fn enabled_cache_is_lru_and_required_entries_survive_shrinking() {
    let frame = frame();
    frame.set_cache_size(2).unwrap();
    let count = Rc::new(Cell::new(0));
    for ty in [FirstPage::TYPE, SecondPage::TYPE, ThirdPage::TYPE] {
        factory(&frame, ty, NavigationCacheMode::Enabled, count.clone());
    }
    frame.navigate(FirstPage::TYPE, None).unwrap();
    let first = frame.current_page().unwrap();
    frame.navigate(SecondPage::TYPE, None).unwrap();
    let second = frame.current_page().unwrap();
    frame.navigate(FirstPage::TYPE, None).unwrap(); // First is most recently used.
    frame.navigate(ThirdPage::TYPE, None).unwrap(); // Second is evicted.
    frame.navigate(FirstPage::TYPE, None).unwrap();
    assert_eq!(frame.current_page(), Some(first));
    frame.navigate(SecondPage::TYPE, None).unwrap();
    assert_ne!(frame.current_page(), Some(second));
    assert_eq!(count.get(), 4);

    frame.set_cache_size(0).unwrap();
    frame.set_cache_size(1).unwrap();
    factory(
        &frame,
        FirstPage::TYPE,
        NavigationCacheMode::Required,
        count.clone(),
    );
    frame.navigate(FirstPage::TYPE, None).unwrap();
    let required = frame.current_page().unwrap();
    frame.set_cache_size(0).unwrap();
    frame.navigate(SecondPage::TYPE, None).unwrap();
    frame.navigate(FirstPage::TYPE, None).unwrap();
    assert_eq!(frame.current_page(), Some(required));
}

#[test]
fn zero_capacity_blocks_new_required_admission_and_disabled_mode_uncaches() {
    let frame = frame();
    frame.set_cache_size(0).unwrap();
    let count = Rc::new(Cell::new(0));
    factory(
        &frame,
        FirstPage::TYPE,
        NavigationCacheMode::Required,
        count.clone(),
    );
    frame.navigate(FirstPage::TYPE, None).unwrap();
    frame.navigate(SecondPage::TYPE, None).unwrap();
    frame.navigate(FirstPage::TYPE, None).unwrap();
    assert_eq!(count.get(), 2);
    frame.set_cache_size(10).unwrap();
    frame.navigate(FirstPage::TYPE, None).unwrap();
    frame
        .current_page()
        .unwrap()
        .set_navigation_cache_mode(NavigationCacheMode::Disabled);
    frame.navigate(FirstPage::TYPE, None).unwrap();
    assert_eq!(count.get(), 4);
}

#[test]
fn failures_preserve_content_history_and_allow_retry() {
    let frame = frame();
    frame.navigate(FirstPage::TYPE, None).unwrap();
    let old = frame.current_page();
    frame
        .register_page_factory(
            SecondPage::TYPE,
            Rc::new(|| Err(NavigationError::FactoryFailed("construction failed".into()))),
        )
        .unwrap();
    assert!(frame.navigate(SecondPage::TYPE, None).is_err());
    assert_eq!(frame.current_page(), old);
    assert_eq!(frame.back_stack_depth(), 0);
    frame.navigation_failed(|args| {
        args.clone().set_handled(true);
    });
    assert!(!frame.navigate(SecondPage::TYPE, None).unwrap());
    frame.unregister_page_factory(SecondPage::TYPE);
    assert!(frame.navigate(SecondPage::TYPE, None).unwrap());
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
        frame.register_page_factory(Button::TYPE, Rc::new(|| Ok(FramePage::new()))),
        Err(NavigationError::NotAPage(_))
    ));
}

#[test]
fn disabled_journal_changes_content_without_committing_history_or_source_type() {
    let frame = frame();
    frame.navigate(FirstPage::TYPE, None).unwrap();
    let options = FrameNavigationOptions::new();
    options.set_is_navigation_stack_enabled(false);
    frame
        .navigate_to_type(SecondPage::TYPE, None, &options)
        .unwrap();
    assert_eq!(frame.current_page().unwrap().get_type(), SecondPage::TYPE);
    assert_eq!(frame.current_source_page_type(), Some(FirstPage::TYPE));
    assert_eq!(frame.back_stack_depth(), 0);
    frame.navigate(ThirdPage::TYPE, None).unwrap();
    assert_eq!(
        frame.back_stack().get(0).unwrap().source_page_type(),
        FirstPage::TYPE
    );
    assert_eq!(frame.back_stack_depth(), 1);
}

#[test]
fn page_and_journal_handles_do_not_keep_a_frame_alive() {
    let frame = frame();
    let weak = frame.downgrade();
    frame.navigate(FirstPage::TYPE, None).unwrap();
    let page = frame.current_page().unwrap();
    let stack = frame.back_stack();
    assert_eq!(page.frame(), Some(frame.clone()));
    drop(frame);
    assert!(weak.upgrade().is_none());
    assert!(page.frame().is_none());
    // A detached journal remains a valid observable collection with no dangling owner.
    stack
        .add(PageStackEntry::new(SecondPage::TYPE, None))
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
