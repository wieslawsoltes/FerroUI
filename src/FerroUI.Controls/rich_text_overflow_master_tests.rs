//! Source-derived ContentSource cache and framework link ownership contracts.
//! WinUI fe244ca6 resolves one predecessor in SetupLinkedBlockLayout, resets
//! continuation masters on attach/detach, and exposes GetMaster without layout.
//! Notification ordering and weak target lifetime are FerroUI adaptations.

use crate::documents::Paragraph;
use crate::flyouts::PopupFlyoutBase;
use crate::test_support::test_scope;
use crate::{
    Control, ControlImpl, Flyout, RichTextBlock, RichTextBlockImpl, RichTextBlockOverflow,
};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::metadata::{from_markup_value, into_markup_value};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{
    ElementRef, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref, Size, StaticType, StyledElementImpl, VisualImpl,
    ferro_class, ferro_impl_classes, instantiate,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PAGE: Size = Size::new(300.0, 100.0);

fn rich(text: &str) -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text(text));
    view
}

fn chain(root: &RichTextBlock) -> (Ref<RichTextBlockOverflow>, Ref<RichTextBlockOverflow>) {
    let first = RichTextBlockOverflow::new();
    let second = RichTextBlockOverflow::new();
    root.set_overflow_content_target(Some(first.clone()));
    first.set_overflow_content_target(Some(second.clone()));
    (first, second)
}

fn measure_chain(root: &RichTextBlock, first: &RichTextBlock, second: &RichTextBlock) {
    root.measure(PAGE);
    first.measure(PAGE);
    second.measure(PAGE);
}

#[test]
fn rich_overflow_master_getters_and_metadata_do_not_resolve_a_linked_chain() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    crate::register_types();
    ferroui_base::register_types();
    let root = rich("master");
    let (first, second) = chain(&root);
    // Registered direct properties already supply markup and binding metadata;
    // no duplicate plain-property declaration is needed.
    let property = RichTextBlockOverflow::TYPE
        .find_property("ContentSource")
        .unwrap();
    assert!(property.is_direct());
    assert!(property.is_read_only(), "ContentSource is read-only");
    assert_eq!(
        property,
        RichTextBlockOverflow::content_source_property().as_property()
    );

    let reflected = second.get_value_untyped(property);
    assert_eq!(
        reflected
            .downcast_ref::<Option<Ref<RichTextBlock>>>()
            .cloned(),
        Some(None)
    );
    assert!(first.content_source().is_none());
    assert!(
        second
            .get_direct_value(RichTextBlockOverflow::content_source_property())
            .is_none()
    );
    // Geometry can ask recursively for page breaks, but is not native linked
    // layout setup and must not fill either public master cache as a side effect.
    let _ = second.content_end();
    assert!(first.content_source().is_none());
    assert!(second.content_source().is_none());
    root.measure(PAGE);
    assert!(first.content_source().is_none());
    first.measure(PAGE);
    assert_eq!(first.content_source(), Some(root.clone()));
    assert!(second.content_source().is_none());
    second.measure(PAGE);
    let reflected = second.get_value_untyped(property);
    assert_eq!(
        reflected
            .downcast_ref::<Option<Ref<RichTextBlock>>>()
            .cloned(),
        Some(Some(root.clone())),
    );
    assert_eq!(
        second.get_direct_value(RichTextBlockOverflow::content_source_property()),
        Some(root)
    );
}

#[test]
fn rich_overflow_master_reflection_binding_observes_cached_master_transitions() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    crate::register_types();
    ferroui_base::register_types();
    let a = rich("A");
    let b = rich("B");
    let (first, second) = chain(&a);
    let target = Control::new();
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = values.clone();
    let object: &ferroui_base::FerroObject = &target;
    let _subscription = FerroObjectExtensions::get_observable(object, Control::tag_property())
        .subscribe_fn(move |value| {
            observed.borrow_mut().push(
                from_markup_value::<Option<Ref<RichTextBlock>>>(&value)
                    .expect("the bound value is a nullable rich-text view"),
            );
        });
    // Resolve the public property by name through the real binding engine.
    // Tag stores the result without giving the primary a new visual parent.
    let binding =
        ReflectionBinding::new("ContentSource").with_source(into_markup_value(second.clone()));
    let _expression = target.bind_binding(Control::tag_property().as_property(), &*binding);
    assert!(target.tag().is_none());
    assert_eq!(*values.borrow(), vec![None]);

    measure_chain(&a, &first, &second);
    assert_eq!(
        from_markup_value::<Option<Ref<RichTextBlock>>>(&target.tag()),
        Some(Some(a.clone())),
    );
    a.set_overflow_content_target(None);
    assert!(target.tag().is_none());
    b.set_overflow_content_target(Some(first.clone()));
    assert!(
        target.tag().is_none(),
        "reparenting does not eagerly resolve the master"
    );
    b.measure(PAGE);
    first.measure(PAGE);
    assert!(
        target.tag().is_none(),
        "the bound later overflow still needs its own setup"
    );
    second.measure(PAGE);
    assert_eq!(
        from_markup_value::<Option<Ref<RichTextBlock>>>(&target.tag()),
        Some(Some(b.clone())),
    );
    assert_eq!(*values.borrow(), vec![None, Some(a), None, Some(b)]);
}

#[test]
fn rich_overflow_master_late_measure_waits_for_the_immediate_predecessor() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let root = rich("master");
    let (first, second) = chain(&root);
    // Prime only ordinary geometry caches. Measuring second must not turn the
    // recursive first.page_layout() query into first's native setup phase.
    let _ = root.page_layout();
    let _ = first.page_layout();
    second.measure(PAGE);
    assert!(second.is_measure_valid());
    assert!(first.content_source().is_none());
    assert!(second.content_source().is_none());

    first.measure(PAGE);
    assert_eq!(first.content_source(), Some(root.clone()));
    assert!(second.content_source().is_none());
    assert!(
        !second.is_measure_valid(),
        "a newly published preceding break schedules continuation measure"
    );
    second.measure(PAGE);
    assert_eq!(second.content_source(), Some(root));
}

#[test]
fn rich_overflow_master_cache_does_not_require_overflow_content() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for text in [None, Some("fits entirely in the primary")] {
        let root = RichTextBlock::new();
        if let Some(text) = text {
            root.blocks().add(Paragraph::with_text(text));
        }
        let first = RichTextBlockOverflow::new();
        root.set_overflow_content_target(Some(first.clone()));
        first.measure(PAGE);
        assert_eq!(first.content_source(), Some(root.clone()));
        assert!(!root.has_overflow_content());
        assert_eq!(first.content_start(), root.document().text_length());
    }
}

#[test]
fn rich_overflow_master_root_identity_keeps_sequential_predecessor_breaks() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let root = rich("first paragraph");
    root.blocks().add(Paragraph::with_text("second paragraph"));
    root.blocks().add(Paragraph::with_text("third paragraph"));
    root.set_max_lines(1);
    let (first, second) = chain(&root);
    first.set_max_lines(1);
    measure_chain(&root, &first, &second);

    assert_eq!(first.content_source(), Some(root.clone()));
    assert_eq!(second.content_source(), Some(root.clone()));
    assert_eq!(first.previous_flow_parent(), Some(root.clone()));
    assert_eq!(second.previous_flow_parent(), Some(first.clone().upcast()));
    assert_eq!(first.content_start(), root.content_end());
    assert_eq!(second.content_start(), first.content_end());
    assert!(root.content_end() < first.content_end());
    assert_eq!(second.content_end(), root.document().text_length());
    assert_eq!(second.document(), root.document());
}

#[test]
fn rich_overflow_master_unlink_clears_the_entire_suffix_before_notifications() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let root = rich("master");
    let (first, second) = chain(&root);
    measure_chain(&root, &first, &second);
    let observed = Rc::new(RefCell::new(Vec::new()));
    let first_weak = first.downgrade();
    let second_weak = second.downgrade();
    let log = observed.clone();
    let _first = first.property_changed(move |change| {
        if change.property() == RichTextBlockOverflow::content_source_property().as_property() {
            assert!(first_weak.upgrade().unwrap().content_source().is_none());
            assert!(second_weak.upgrade().unwrap().content_source().is_none());
            log.borrow_mut().push("first");
        }
    });
    let second_weak = second.downgrade();
    let log = observed.clone();
    let _second = second.property_changed(move |change| {
        if change.property() == RichTextBlockOverflow::content_source_property().as_property() {
            assert!(second_weak.upgrade().unwrap().content_source().is_none());
            log.borrow_mut().push("second");
        }
    });

    root.set_overflow_content_target(None);
    assert_eq!(*observed.borrow(), vec!["first", "second"]);
    assert!(first.previous_flow_parent().is_none());
    assert_eq!(second.previous_flow_parent(), Some(first.clone().upcast()));
    second.measure(PAGE);
    assert!(
        second.content_source().is_none(),
        "an orphan predecessor is not a primary"
    );
}

#[test]
fn rich_overflow_master_reparent_requires_new_predecessor_setup() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let a = rich("A");
    let b = rich("B");
    let (first, second) = chain(&a);
    measure_chain(&a, &first, &second);
    a.set_overflow_content_target(None);
    b.set_overflow_content_target(Some(first.clone()));
    assert!(first.content_source().is_none());
    assert!(second.content_source().is_none());
    assert_eq!(second.document(), b.document());
    second.measure(PAGE);
    assert!(second.content_source().is_none());
    measure_chain(&b, &first, &second);
    assert_eq!(first.content_source(), Some(b.clone()));
    assert_eq!(second.content_source(), Some(b));
}

#[test]
fn rich_overflow_master_ordinary_layout_changes_preserve_the_cached_master() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let root = rich("master");
    let (first, second) = chain(&root);
    measure_chain(&root, &first, &second);
    let events = Rc::new(Cell::new(0));
    let observed = events.clone();
    let _subscription = second.property_changed(move |change| {
        if change.property() == RichTextBlockOverflow::content_source_property().as_property() {
            observed.set(observed.get() + 1);
        }
    });
    root.set_font_size(19.0);
    first.set_width(250.0);
    root.blocks().add(Paragraph::with_text("another paragraph"));
    assert_eq!(second.content_source(), Some(root.clone()));
    measure_chain(&root, &first, &second);
    assert_eq!(second.content_source(), Some(root));
    assert_eq!(events.get(), 0);
}

#[test]
fn rich_overflow_master_property_events_follow_cache_transitions_on_each_descendant() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let a = rich("A");
    let b = rich("B");
    let (first, second) = chain(&a);
    let first_events = Rc::new(RefCell::new(Vec::new()));
    let second_events = Rc::new(RefCell::new(Vec::new()));
    let documents = Rc::new(RefCell::new(Vec::new()));
    let mut subscriptions = Vec::new();
    for (view, log) in [
        (first.clone(), first_events.clone()),
        (second.clone(), second_events.clone()),
    ] {
        let documents = documents.clone();
        let view_weak = view.downgrade();
        subscriptions.push(view.property_changed(move |change| {
            if change.property() == RichTextBlockOverflow::content_source_property().as_property() {
                let values = change.get_old_and_new_value::<Option<Ref<RichTextBlock>>>();
                assert_eq!(view_weak.upgrade().unwrap().content_source(), values.1);
                log.borrow_mut().push(values);
            }
            if change.property() == RichTextBlock::document_property().as_property() {
                let values = change.get_old_and_new_value::<crate::documents::RichTextDocument>();
                assert_eq!(view_weak.upgrade().unwrap().document(), values.1);
                documents.borrow_mut().push(values);
            }
        }));
    }
    measure_chain(&a, &first, &second);
    a.set_overflow_content_target(None);
    b.set_overflow_content_target(Some(first.clone()));
    measure_chain(&b, &first, &second);
    let expected = vec![(None, Some(a.clone())), (Some(a), None), (None, Some(b))];
    assert_eq!(*first_events.borrow(), expected);
    assert_eq!(*second_events.borrow(), expected);
    assert_eq!(
        documents.borrow().len(),
        4,
        "both descendants observe detach and attach documents"
    );
    for subscription in subscriptions {
        subscription.dispose();
    }
}

#[test]
fn rich_overflow_master_weak_root_expiry_does_not_retain_the_owner() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let root = rich("master");
    let (first, second) = chain(&root);
    measure_chain(&root, &first, &second);
    let weak = root.downgrade();
    drop(root);
    assert!(weak.upgrade().is_none());
    assert!(first.content_source().is_none());
    assert!(second.content_source().is_none());
    assert!(first.previous_flow_parent().is_none());
    assert!(first.cached_page_layout().is_none());
    assert!(second.cached_page_layout().is_none());
}

#[test]
fn rich_overflow_master_weak_intermediate_expiry_resets_surviving_suffix() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let root = rich("master");
    let (first, second) = chain(&root);
    measure_chain(&root, &first, &second);
    let weak = first.downgrade();
    drop(first);
    assert!(weak.upgrade().is_none());
    assert!(root.overflow_content_target().is_none());
    assert!(second.previous_flow_parent().is_none());
    assert!(
        second.content_source().is_none(),
        "the still-live primary cannot bridge a dead predecessor"
    );
    assert!(second.cached_page_layout().is_none());
    assert!(!second.is_measure_valid());
    second.measure(PAGE);
    assert!(second.content_source().is_none());
}

#[test]
fn rich_overflow_master_primary_drop_notifies_the_previous_document_without_retention() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for measured in [false, true] {
        let root = rich("master");
        let first = RichTextBlockOverflow::new();
        let orphan_document = first.document();
        let second = RichTextBlockOverflow::new();
        root.set_overflow_content_target(Some(first.clone()));
        first.set_overflow_content_target(Some(second.clone()));
        if measured {
            measure_chain(&root, &first, &second);
        }
        let previous_document = root.document();
        let events = Rc::new(Cell::new(0));
        let mut subscriptions = Vec::new();
        for view in [first.clone(), second.clone()] {
            let previous_document = previous_document.clone();
            let orphan_document = orphan_document.clone();
            let observed = events.clone();
            let weak = view.downgrade();
            subscriptions.push(view.property_changed(move |change| {
                if change.property() == RichTextBlock::document_property().as_property() {
                    let (old, new) =
                        change.get_old_and_new_value::<crate::documents::RichTextDocument>();
                    assert_eq!(old, previous_document);
                    assert_eq!(new, orphan_document);
                    assert_eq!(weak.upgrade().unwrap().document(), new);
                    observed.set(observed.get() + 1);
                }
            }));
        }

        let weak = root.downgrade();
        drop(root);
        assert!(weak.upgrade().is_none());
        assert_eq!(
            events.get(),
            2,
            "both suffix documents change even before master setup"
        );
        assert_eq!(first.document(), orphan_document);
        assert_eq!(second.document(), orphan_document);
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
}

#[test]
fn rich_overflow_master_intermediate_drop_notifies_the_root_document_for_its_suffix() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for measured in [false, true] {
        let root = rich("master");
        let first = RichTextBlockOverflow::new();
        let second = RichTextBlockOverflow::new();
        let orphan_document = second.document();
        let third = RichTextBlockOverflow::new();
        root.set_overflow_content_target(Some(first.clone()));
        first.set_overflow_content_target(Some(second.clone()));
        second.set_overflow_content_target(Some(third.clone()));
        if measured {
            measure_chain(&root, &first, &second);
            third.measure(PAGE);
        }
        let previous_document = root.document();
        let events = Rc::new(Cell::new(0));
        let mut subscriptions = Vec::new();
        for view in [second.clone(), third.clone()] {
            let previous_document = previous_document.clone();
            let orphan_document = orphan_document.clone();
            let observed = events.clone();
            let weak = view.downgrade();
            subscriptions.push(view.property_changed(move |change| {
                if change.property() == RichTextBlock::document_property().as_property() {
                    let (old, new) =
                        change.get_old_and_new_value::<crate::documents::RichTextDocument>();
                    assert_eq!(old, previous_document);
                    assert_eq!(new, orphan_document);
                    assert_eq!(weak.upgrade().unwrap().document(), new);
                    observed.set(observed.get() + 1);
                }
            }));
        }

        let weak = first.downgrade();
        drop(first);
        assert!(weak.upgrade().is_none());
        assert_eq!(events.get(), 2);
        assert!(root.overflow_content_target().is_none());
        assert!(second.content_source().is_none());
        assert!(third.content_source().is_none());
        assert_eq!(second.document(), orphan_document);
        assert_eq!(third.document(), orphan_document);
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
}

#[repr(C)]
struct ReentrantView {
    base: RichTextBlock,
    before_change: RefCell<Option<Box<dyn FnOnce(&ReentrantView)>>>,
}

ferro_class!(ReentrantView: RichTextBlock);
ferro_impl_classes!(ReentrantView:
    StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl,
    InputElementImpl, ControlImpl, RichTextBlockImpl);

impl FerroObjectImpl for ReentrantView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == RichTextBlock::overflow_content_target_property().as_property() {
            let callback = this.before_change.borrow_mut().take();
            if let Some(callback) = callback {
                callback(this);
            }
        }
        Self::parent_on_property_changed(this, change);
    }
}

#[test]
fn rich_overflow_master_nested_property_override_reconciles_the_installed_edge() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let root: Ref<ReentrantView> = instantiate(ReentrantView {
        base: RichTextBlock::construct(),
        before_change: RefCell::new(None),
    });
    root.blocks().add(Paragraph::with_text("master"));
    let old = RichTextBlockOverflow::new();
    let outer = RichTextBlockOverflow::new();
    let newer = RichTextBlockOverflow::new();
    root.set_overflow_content_target(Some(old.clone()));
    root.measure(PAGE);
    old.measure(PAGE);
    let replacement = newer.clone();
    *root.before_change.borrow_mut() = Some(Box::new(move |this| {
        this.set_overflow_content_target(Some(replacement));
    }));
    root.set_overflow_content_target(Some(outer.clone()));

    assert_eq!(root.overflow_content_target(), Some(newer.clone()));
    assert!(old.previous_flow_parent().is_none());
    assert!(old.content_source().is_none());
    assert!(outer.previous_flow_parent().is_none());
    assert_eq!(newer.previous_flow_parent(), Some(root.clone().upcast()));
    newer.measure(PAGE);
    assert_eq!(newer.content_source(), Some(root.upcast()));
}

#[test]
fn rich_overflow_master_reset_handler_preserves_a_newer_reparent_and_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let a = rich("A");
    let b = rich("B");
    let (first, second) = chain(&a);
    measure_chain(&a, &first, &second);
    let after = Rc::new(RefCell::new(None));
    let saved = after.clone();
    let first_weak = first.downgrade();
    let second_weak = second.downgrade();
    let b_weak = b.downgrade();
    let once = Cell::new(false);
    let _subscription = first.property_changed(move |change| {
        if change.property() != RichTextBlockOverflow::content_source_property().as_property() {
            return;
        }
        let (_, value) = change.get_old_and_new_value::<Option<Ref<RichTextBlock>>>();
        if value.is_none() && !once.replace(true) {
            let first = first_weak.upgrade().unwrap();
            let second = second_weak.upgrade().unwrap();
            let b = b_weak.upgrade().unwrap();
            b.set_overflow_content_target(Some(first.clone()));
            measure_chain(&b, &first, &second);
            *saved.borrow_mut() = second.cached_page_layout();
        }
    });
    let stale = Rc::new(Cell::new(0));
    let observed = stale.clone();
    let b_weak = b.downgrade();
    let _second = second.property_changed(move |change| {
        if change.property() == RichTextBlockOverflow::content_source_property().as_property() {
            let (_, value) = change.get_old_and_new_value::<Option<Ref<RichTextBlock>>>();
            if value != b_weak.upgrade() {
                observed.set(observed.get() + 1);
            }
        }
    });
    a.set_overflow_content_target(None);
    assert_eq!(first.content_source(), Some(b.clone()));
    assert_eq!(second.content_source(), Some(b.clone()));
    assert_eq!(second.document(), b.document());
    assert_eq!(
        stale.get(),
        0,
        "retired reset wave must not publish an old None after nested setup"
    );
    assert!(Rc::ptr_eq(
        after.borrow().as_ref().unwrap(),
        &second.cached_page_layout().unwrap()
    ));
}

#[repr(C)]
struct InvalidatingOverflow {
    base: RichTextBlockOverflow,
    on_invalidate: RefCell<Option<Box<dyn FnOnce()>>>,
}

ferro_class!(InvalidatingOverflow: RichTextBlockOverflow);
ferro_impl_classes!(InvalidatingOverflow:
    FerroObjectImpl, StyledElementImpl, VisualImpl, InteractiveImpl,
    InputElementImpl, ControlImpl, RichTextBlockImpl);

impl LayoutableImpl for InvalidatingOverflow {
    fn on_measure_invalidated(this: &Self) {
        Self::parent_on_measure_invalidated(this);
        let callback = this.on_invalidate.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
    }
}

#[test]
fn rich_overflow_master_invalidation_callback_sees_atomic_reset_and_keeps_new_owner() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let a = rich("A");
    let b = rich("B");
    let first: Ref<InvalidatingOverflow> = instantiate(InvalidatingOverflow {
        base: RichTextBlockOverflow::construct(),
        on_invalidate: RefCell::new(None),
    });
    let second = RichTextBlockOverflow::new();
    a.set_overflow_content_target(Some(first.clone().upcast()));
    first.set_overflow_content_target(Some(second.clone()));
    measure_chain(&a, &first, &second);
    let first_weak = first.downgrade();
    let second_weak = second.downgrade();
    let b_weak = b.downgrade();
    let called = Rc::new(Cell::new(false));
    let observed = called.clone();
    *first.on_invalidate.borrow_mut() = Some(Box::new(move || {
        let first = first_weak.upgrade().unwrap();
        let second = second_weak.upgrade().unwrap();
        assert!(first.content_source().is_none());
        assert!(second.content_source().is_none());
        observed.set(true);
        let b = b_weak.upgrade().unwrap();
        b.set_overflow_content_target(Some(first.clone().upcast()));
        measure_chain(&b, &first, &second);
    }));
    a.set_overflow_content_target(None);
    assert!(called.get());
    assert_eq!(first.content_source(), Some(b.clone()));
    assert_eq!(second.content_source(), Some(b.clone()));
    assert!(second.is_measure_valid());
}

#[test]
fn rich_overflow_master_setup_callback_cannot_restore_a_replaced_master() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let a = rich("A");
    let b = rich("B");
    let first = RichTextBlockOverflow::new();
    a.set_overflow_content_target(Some(first.clone()));
    let first_weak = first.downgrade();
    let a_weak = a.downgrade();
    let b_weak = b.downgrade();
    let once = Cell::new(false);
    let _subscription = first.property_changed(move |change| {
        if change.property() == RichTextBlockOverflow::content_source_property().as_property()
            && !once.replace(true)
        {
            a_weak.upgrade().unwrap().set_overflow_content_target(None);
            let b = b_weak.upgrade().unwrap();
            let first = first_weak.upgrade().unwrap();
            b.set_overflow_content_target(Some(first.clone()));
            b.measure(PAGE);
            first.measure(PAGE);
        }
    });
    first.measure(PAGE);
    assert!(a.overflow_content_target().is_none());
    assert_eq!(b.overflow_content_target(), Some(first.clone()));
    assert_eq!(first.content_source(), Some(b.clone()));
    assert_eq!(first.document(), b.document());
}

#[test]
fn rich_overflow_master_drop_notification_can_reparent_the_surviving_view() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let a = rich("A");
    let b = rich("B");
    let (first, second) = chain(&a);
    measure_chain(&a, &first, &second);
    let second_weak = second.downgrade();
    let b_weak = b.downgrade();
    let once = Cell::new(false);
    let _subscription = second.property_changed(move |change| {
        if change.property() == RichTextBlockOverflow::content_source_property().as_property() {
            let (_, value) = change.get_old_and_new_value::<Option<Ref<RichTextBlock>>>();
            if value.is_none() && !once.replace(true) {
                let second = second_weak.upgrade().unwrap();
                let b = b_weak.upgrade().unwrap();
                b.set_overflow_content_target(Some(second.clone()));
                b.measure(PAGE);
                second.measure(PAGE);
            }
        }
    });
    let weak = first.downgrade();
    drop(first);
    assert!(weak.upgrade().is_none());
    assert!(a.overflow_content_target().is_none());
    assert_eq!(second.previous_flow_parent(), Some(b.clone()));
    assert_eq!(second.content_source(), Some(b.clone()));
    assert!(second.cached_page_layout().is_some());
}

#[test]
fn rich_overflow_master_invalid_markup_target_does_not_detach_another_owner() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let a = rich("A");
    let b = rich("B");
    let old = RichTextBlockOverflow::new();
    let other = RichTextBlockOverflow::new();
    a.set_overflow_content_target(Some(old.clone()));
    b.set_overflow_content_target(Some(other.clone()));
    a.set_value(
        RichTextBlock::overflow_content_target_property(),
        Some(ElementRef::new(&other)),
    );
    assert!(a.overflow_content_target().is_none());
    assert!(old.previous_flow_parent().is_none());
    assert_eq!(b.overflow_content_target(), Some(other.clone()));
    assert_eq!(other.previous_flow_parent(), Some(b));
}

#[test]
fn rich_overflow_master_context_lookup_uses_the_same_cache_as_content_source() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let root = rich("master");
    let (first, second) = chain(&root);
    let root_flyout = Flyout::new();
    let intermediate = Flyout::new();
    root.set_context_flyout(&root_flyout);
    first.set_context_flyout(&intermediate);
    assert!(PopupFlyoutBase::effective_context_flyout(&second).is_none());
    first.measure(PAGE);
    assert!(PopupFlyoutBase::effective_context_flyout(&second).is_none());
    second.measure(PAGE);
    assert_eq!(
        PopupFlyoutBase::effective_context_flyout(&second),
        Some(root_flyout.clone().upcast())
    );
    assert_eq!(second.content_source(), Some(root.clone()));
    root.set_overflow_content_target(None);
    assert!(PopupFlyoutBase::effective_context_flyout(&second).is_none());
    let own = Flyout::new();
    second.set_context_flyout(&own);
    assert_eq!(
        PopupFlyoutBase::effective_context_flyout(&second),
        Some(own.upcast())
    );
    second.clear_value(Control::context_flyout_property());
    assert!(PopupFlyoutBase::effective_context_flyout(&second).is_none());
}
