use crate::documents::CharacterFormat;
use crate::flyouts::{Flyout, FlyoutBaseImplExt, FlyoutShowMode};
use crate::platform::{IPopupImpl, ITopLevelImpl};
use crate::testing::{
    MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use crate::{Border, Control, RichEditBox, StackPanel, Window};
use ferroui_base::input::text_input::{
    ITextInputMethodImpl, TextInputMethodClient, TextInputMethodConnection, TextInputMethodError,
    TextInputOptions,
};
use ferroui_base::input::{InputElement, InputManager, TextInputEventArgs};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::FontWeight;
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{Rect, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct RecordingInputMethod {
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    connection: RefCell<Option<Rc<TextInputMethodConnection>>>,
    resets: Cell<usize>,
    installations: Cell<usize>,
    cursor: Cell<Rect>,
    options: RefCell<TextInputOptions>,
}

impl ITextInputMethodImpl for RecordingInputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        self.installations.set(self.installations.get() + 1);
        *self.connection.borrow_mut() = client
            .as_ref()
            .and_then(|client| client.input_method_connection());
        *self.client.borrow_mut() = client;
    }
    fn set_cursor_rect(&self, rect: Rect) {
        self.cursor.set(rect);
    }
    fn set_options(&self, options: &TextInputOptions) {
        *self.options.borrow_mut() = options.clone();
    }
    fn reset(&self) {
        self.resets.set(self.resets.get() + 1);
        let client = self.client.borrow().clone();
        if let Some(client) = client {
            client.set_preedit_text(None);
        }
    }
}

struct Fixture {
    window: Ref<Window>,
    owner: Ref<RichEditBox>,
    outside: Ref<Border>,
    native: Rc<RecordingInputMethod>,
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

impl Fixture {
    fn new(overlay: bool) -> Self {
        let native = Rc::new(RecordingInputMethod::default());
        let method = native.clone();
        let app =
            UnitTestApplication::start(TestServices::focusable_window().with_windowing_platform(
                MockWindowingPlatform::with_window_impl(move || {
                    let window = MockWindowingPlatform::create_window_mock();
                    window.setup_feature::<dyn ITextInputMethodImpl>(method.clone());
                    let weak = Rc::downgrade(&window);
                    window.setup_create_popup(move |_| {
                        if overlay {
                            None
                        } else {
                            let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
                            Some(MockWindowingPlatform::create_popup_mock(parent)
                                as Rc<dyn IPopupImpl>)
                        }
                    });
                    window
                }),
            ));
        let text = TextTestScope::new();
        let owner = RichEditBox::new();
        owner.set_template(Some(crate::rich_edit_box_tests::rich_edit_box_template()));
        owner.set_height(80.0);
        owner.set_text("abcd").unwrap();
        owner.select(1, 3);
        owner
            .apply_format(&CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..CharacterFormat::default()
            })
            .unwrap();
        owner.editor().clear_history();
        let outside = Border::new();
        outside.set_focusable(true);
        outside.set_height(30.0);
        let panel = StackPanel::new();
        panel.children().add(owner.clone());
        panel.children().add(outside.clone());
        let window = Window::new();
        window.set_width(260.0);
        window.set_height(140.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        owner.focus();
        assert!(owner.is_focused());
        let fixture = Self {
            window,
            owner,
            outside,
            native,
            _text: text,
            _app: app,
        };
        assert!(fixture.connection().is_current());
        fixture
    }

    fn connection(&self) -> Rc<TextInputMethodConnection> {
        self.native
            .connection
            .borrow()
            .clone()
            .expect("a manager-owned input connection")
    }

    fn flyout(&self, child: Ref<Control>) -> Ref<Flyout> {
        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(child)));
        flyout.set_show_mode(FlyoutShowMode::Standard);
        self.owner.set_context_flyout(flyout.clone());
        flyout
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[path = "rich_edit_exact_input_tests.rs"]
mod exact_tests;

#[path = "rich_edit_composition_event_tests.rs"]
mod composition_event_tests;

#[path = "rich_edit_input_options_tests.rs"]
mod input_options_tests;

#[path = "rich_edit_input_geometry_tests.rs"]
mod input_geometry_tests;

#[test]
fn rich_input_method_start_preserves_styled_selection_and_commit_is_one_undo_unit() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let revision = f.owner.document().revision();
    let session = f.connection().begin_composition().unwrap();
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(f.owner.document().revision(), revision);
    assert!(!f.owner.can_undo());
    session.update("候補", Some(1)).unwrap();
    assert_eq!(f.owner.text(), "a候補d");
    assert_eq!(
        f.owner.text_input_method_client().surrounding_text(),
        "abcd"
    );
    assert!(!f.owner.can_undo());
    assert!(session.commit("確定", 12).unwrap());
    assert_eq!(f.owner.text(), "a確定d");
    assert!(!session.is_current());
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
}

#[test]
fn rich_input_method_cancel_restores_format_without_history_and_empty_start_is_not_deletion() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let start = f.connection().begin_composition().unwrap();
    assert!(start.cancel().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    let session = f.connection().begin_composition().unwrap();
    session.update("preview", None).unwrap();
    assert!(session.cancel().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_input_method_intercepted_routed_commit_retains_preview_for_retry() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.update("preview", None).unwrap();
    let preview = f.owner.document().snapshot();
    let token = f.owner.add_handler_with(
        InputElement::text_input_event(),
        |_, event: &TextInputEventArgs| event.set_handled(true),
        RoutingStrategies::TUNNEL,
        false,
    );
    assert_eq!(
        session.commit("accepted", 1),
        Err(TextInputMethodError::Intercepted)
    );
    assert_eq!(f.owner.document().snapshot(), preview);
    assert!(session.is_current());
    assert!(!f.owner.can_undo());
    f.owner
        .remove_handler(InputElement::text_input_event(), token);
    assert!(session.commit("accepted", 2).unwrap());
    assert_eq!(f.owner.text(), "aacceptedd");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
}

#[test]
fn rich_input_method_raw_interception_preserves_preview_and_owner_handoff_rejects_stale_input() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.update("preview", None).unwrap();
    let input = InputManager::instance().unwrap();
    let subscription = input.pre_process().subscribe_fn(|event| {
        if event
            .downcast_ref::<ferroui_base::input::raw::RawTextInputEventArgs>()
            .is_some()
        {
            event.set_handled(true);
        }
    });
    assert_eq!(
        session.commit("blocked", 1),
        Err(TextInputMethodError::Intercepted)
    );
    assert!(f.owner.editor().is_composing());
    subscription.dispose();
    let outside = f.outside.downgrade();
    let subscription = input.pre_process().subscribe_fn(move |_| {
        outside.upgrade().unwrap().focus();
    });
    assert_eq!(
        session.commit("stale", 2),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(f.owner.text(), "abcd");
    assert!(!session.is_current());
    subscription.dispose();
}

#[test]
fn rich_input_method_external_selection_and_document_mutations_invalidate_sessions() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.update("preview", None).unwrap();
    f.owner.select(0, 0);
    let changed = f.owner.document().snapshot();
    assert_eq!(
        session.commit("stale", 0),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(f.owner.document().snapshot(), changed);
    let session = f.connection().begin_composition().unwrap();
    f.owner.document().replace_text(0..0, "external").unwrap();
    let changed = f.owner.document().snapshot();
    assert_eq!(
        session.update("stale", None),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(f.owner.document().snapshot(), changed);
}

#[test]
fn rich_input_method_preview_cursor_updates_preserve_committed_selection_notifications() {
    use ferroui_base::input::text_input::TextSelection;

    let f = Fixture::new(true);
    let input = f.owner.text_input_method_client();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let recorded = seen.clone();
    let weak_input = Rc::downgrade(&input);
    let subscription = input.selection_changed(Rc::new(move || {
        recorded
            .borrow_mut()
            .push(weak_input.upgrade().unwrap().selection());
    }));
    let session = f.connection().begin_composition().unwrap();
    for cursor in [1, 3, 0, 4] {
        session.update("候😀補", Some(cursor)).unwrap();
        assert_eq!(f.owner.text(), "a候😀補d");
        assert_eq!(input.selection(), TextSelection::new(1, 3));
        assert!(session.is_current());
        assert!(seen.borrow().is_empty());
    }
    assert!(session.cancel().unwrap());
    assert_eq!(f.owner.text(), "abcd");
    assert_eq!(input.selection(), TextSelection::new(1, 3));
    assert!(seen.borrow().is_empty());
    assert!(!f.owner.can_undo());

    let session = f.connection().begin_composition().unwrap();
    session.update("候😀補", Some(3)).unwrap();
    assert!(session.commit("確😀", 1).unwrap());
    assert_eq!(&*seen.borrow(), &[TextSelection::new(4, 4)]);
    assert!(f.owner.undo().unwrap());
    assert_eq!(
        &*seen.borrow(),
        &[TextSelection::new(4, 4), TextSelection::new(1, 3)]
    );
    assert!(f.owner.redo().unwrap());
    assert_eq!(
        &*seen.borrow(),
        &[
            TextSelection::new(4, 4),
            TextSelection::new(1, 3),
            TextSelection::new(4, 4),
        ]
    );
    subscription.dispose();
}

#[test]
fn rich_input_method_external_caret_round_trip_invalidates_preview_without_selection_events() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.update("preview", Some(3)).unwrap();
    let input = f.owner.text_input_method_client();
    let committed = input.selection();
    let visible = (f.owner.selection_start(), f.owner.selection_end());
    let snapshot = f.owner.document().snapshot();
    let count = Rc::new(Cell::new(0));
    let recorded = count.clone();
    let subscription = input.selection_changed(Rc::new(move || {
        recorded.set(recorded.get() + 1);
    }));

    // Do not query the session or its revision between the two changes. Merely
    // comparing the final selection with the last queried one misses this case.
    f.owner.select(0, 0);
    f.owner.select(visible.0, visible.1);

    assert_eq!(input.selection(), committed);
    assert_eq!(count.get(), 0);
    assert!(!session.is_current());
    assert_eq!(
        session.update("stale", None),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(
        session.commit("stale", 0),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(f.owner.document().snapshot(), snapshot);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), visible);
    subscription.dispose();
}

#[test]
fn rich_input_method_reset_and_new_composition_retire_captured_sessions() {
    let f = Fixture::new(true);
    let connection = f.connection();
    let first = connection.begin_composition().unwrap();
    first.update("first", None).unwrap();
    let second = connection.begin_composition().unwrap();
    assert!(!first.is_current());
    assert_eq!(f.owner.text(), "abcd");
    second.update("second", None).unwrap();
    f.owner.text_input_method_client().request_reset();
    assert!(!connection.is_current());
    assert!(!second.is_current());
    assert_eq!(f.owner.text(), "abcd");
    assert!(f.connection().is_current());
    assert!(!Rc::ptr_eq(&connection, &f.connection()));
}

#[test]
fn rich_input_method_owned_flyout_keeps_transport_and_routes_commit_to_semantic_owner() {
    for overlay in [false, true] {
        let f = Fixture::new(overlay);
        let connection = f.connection();
        let session = connection.begin_composition().unwrap();
        session.update("preview", None).unwrap();
        let child = Border::new();
        child.set_focusable(true);
        child.set_width(60.0);
        child.set_height(20.0);
        let flyout = f.flyout(child.clone().upcast());
        let resets = f.native.resets.get();
        let installations = f.native.installations.get();
        flyout.show_at(&f.owner);
        assert!(child.is_focused());
        assert_eq!(f.native.resets.get(), resets);
        assert_eq!(f.native.installations.get(), installations);
        assert!(Rc::ptr_eq(&connection, &f.connection()));
        assert!(session.commit("owner", 0).unwrap());
        assert_eq!(f.owner.text(), "aownerd");
        f.outside.focus();
        assert!(!connection.is_current());
        flyout.hide();
    }
}

#[test]
fn rich_input_method_popup_editor_claims_its_own_connection() {
    let f = Fixture::new(true);
    let old = f.connection();
    let session = old.begin_composition().unwrap();
    session.update("preview", None).unwrap();
    let other = RichEditBox::new();
    other.set_template(Some(crate::rich_edit_box_tests::rich_edit_box_template()));
    other.set_width(100.0);
    other.set_height(35.0);
    other.set_text("other").unwrap();
    let flyout = f.flyout(other.clone().upcast());
    flyout.show_at(&f.owner);
    other.focus();
    assert!(other.is_focused());
    assert!(!old.is_current());
    assert_eq!(f.owner.text(), "abcd");
    assert_eq!(
        session.commit("stale", 0),
        Err(TextInputMethodError::StaleConnection)
    );
    assert_eq!(other.text(), "other");
    flyout.hide();
}

#[test]
fn rich_input_method_tunnel_document_edit_is_preserved_and_rejects_original_commit() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    let owner = f.owner.downgrade();
    let token = f.owner.add_handler_with(
        InputElement::text_input_event(),
        move |_, _: &TextInputEventArgs| {
            owner.upgrade().unwrap().set_text("external").unwrap();
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    assert_eq!(
        session.commit("stale", 0),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(f.owner.text(), "external");
    f.owner
        .remove_handler(InputElement::text_input_event(), token);
}

#[test]
fn rich_input_method_composition_callbacks_observe_accepted_text_and_invalidate_older_input() {
    for started in [true, false] {
        let f = Fixture::new(true);
        let owner = f.owner.downgrade();
        let event = if started {
            RichEditBox::text_composition_started_event()
        } else {
            RichEditBox::text_composition_changed_event()
        };
        f.owner.add_handler(event, move |_, args| {
            let owner = owner.upgrade().unwrap();
            assert_eq!(args.text, if started { "" } else { "preview" });
            assert_eq!(owner.text(), if started { "abcd" } else { "apreviewd" });
            owner.set_text("external").unwrap();
        });

        if started {
            assert!(matches!(
                f.connection().begin_composition(),
                Err(TextInputMethodError::ReentrantEdit)
            ));
        } else {
            let session = f.connection().begin_composition().unwrap();
            assert_eq!(
                session.update("preview", None),
                Err(TextInputMethodError::ReentrantEdit)
            );
            assert!(!session.is_current());
            assert_eq!(
                session.commit("stale", 0),
                Err(TextInputMethodError::StateChanged)
            );
        }
        assert_eq!(f.owner.text(), "external");
    }
}

#[test]
fn rich_input_method_composition_events_report_the_filtered_and_length_limited_preview() {
    let f = Fixture::new(true);
    f.owner.set_max_length(4);
    f.owner.set_accepts_return(false);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let started = seen.clone();
    f.owner.add_handler(
        RichEditBox::text_composition_started_event(),
        move |_, args| {
            started.borrow_mut().push(("started", args.text.clone()));
        },
    );
    let changed = seen.clone();
    f.owner.add_handler(
        RichEditBox::text_composition_changed_event(),
        move |_, args| {
            changed.borrow_mut().push(("changed", args.text.clone()));
        },
    );

    let session = f.connection().begin_composition().unwrap();
    session.update("x\nyz", None).unwrap();

    assert_eq!(f.owner.text(), "axyd");
    assert_eq!(
        &*seen.borrow(),
        &[("started", String::new()), ("changed", "xy".to_owned())]
    );
    assert!(session.is_current());
}

#[test]
fn rich_input_method_connection_is_weak_and_read_only_handoff_blocks_callbacks() {
    let f = Fixture::new(true);
    let connection = f.connection();
    let session = connection.begin_composition().unwrap();
    session.update("preview", None).unwrap();
    f.owner.set_is_read_only(true);
    assert!(!connection.is_current());
    assert_eq!(f.owner.text(), "abcd");
    assert_eq!(
        session.commit("stale", 0),
        Err(TextInputMethodError::StaleConnection)
    );
    let owner = f.owner.downgrade();
    f.window.close();
    drop(f);
    assert!(owner.upgrade().is_none());
    assert!(connection.owner().is_none());
}

#[path = "rich_edit_input_range_geometry_tests.rs"]
mod input_range_geometry_tests;
