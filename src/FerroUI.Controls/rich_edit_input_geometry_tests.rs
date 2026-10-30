// Public input-manager integration regressions. The custom client intentionally
// invokes application code from a geometry query, as framework clients may do
// while resolving their current layout.
use super::*;
use ferroui_base::input::text_input::{
    TextInputMethodClientEvents, TextInputMethodClientRequeryRequestedEventArgs, TextSelection,
};
use ferroui_base::{Visual, WeakRef};

struct CursorQueryClient {
    events: TextInputMethodClientEvents,
    owner: WeakRef<InputElement>,
    cursor: Cell<Rect>,
    query: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl CursorQueryClient {
    fn new(owner: &InputElement, cursor: Rect) -> Rc<Self> {
        Rc::new(Self {
            events: TextInputMethodClientEvents::new(),
            owner: owner.to_ref().downgrade(),
            cursor: Cell::new(cursor),
            query: RefCell::new(None),
        })
    }
}

impl TextInputMethodClient for CursorQueryClient {
    fn events(&self) -> &TextInputMethodClientEvents {
        &self.events
    }
    fn input_method_owner(&self) -> Option<Ref<InputElement>> {
        self.owner.upgrade()
    }
    fn text_view_visual(&self) -> Option<Ref<Visual>> {
        self.owner.upgrade().map(Ref::upcast)
    }
    fn supports_preedit(&self) -> bool {
        false
    }
    fn supports_surrounding_text(&self) -> bool {
        false
    }
    fn surrounding_text(&self) -> String {
        String::new()
    }
    fn cursor_rectangle(&self) -> Rect {
        let rect = self.cursor.get();
        let query = self.query.borrow_mut().take();
        if let Some(query) = query {
            query();
        }
        rect
    }
    fn selection(&self) -> TextSelection {
        TextSelection::default()
    }
    fn set_selection(&self, _: TextSelection) {}
}

fn provide_client(
    owner: &InputElement,
    client: Rc<CursorQueryClient>,
) -> Rc<RefCell<Rc<dyn TextInputMethodClient>>> {
    let current = Rc::new(RefCell::new(client as Rc<dyn TextInputMethodClient>));
    let requested = current.clone();
    owner.add_handler(
        InputElement::text_input_method_client_requested_event(),
        move |_, event| {
            event.set_client(Some(requested.borrow().clone()));
        },
    );
    owner.raise_event(&TextInputMethodClientRequeryRequestedEventArgs::with_event());
    current
}

fn in_root(owner: &InputElement, rect: Rect) -> Rect {
    let root = owner.visual_root().unwrap();
    rect.transform_to_aabb(owner.transform_to_visual(&root).unwrap())
}

#[test]
fn rich_input_cursor_getter_cannot_overwrite_a_replacement_owners_caret() {
    let f = Fixture::new(true);
    let first = CursorQueryClient::new(&f.owner, Rect::new(11.0, 13.0, 1.0, 17.0));
    provide_client(&f.owner, first.clone());
    let second = CursorQueryClient::new(&f.outside, Rect::new(3.0, 7.0, 2.0, 19.0));
    provide_client(&f.outside, second.clone());
    let old_connection = f.connection();
    let expected = in_root(&f.outside, second.cursor.get());
    assert_ne!(expected, in_root(&f.owner, first.cursor.get()));
    let outside = f.outside.clone();
    first.query.replace(Some(Box::new(move || {
        assert!(outside.focus());
    })));
    first.raise_cursor_rectangle_changed();
    assert!(f.outside.is_focused());
    assert!(!old_connection.is_current());
    assert_eq!(
        f.connection().owner(),
        Some(f.outside.clone().upcast::<InputElement>())
    );
    let expected_client: Rc<dyn TextInputMethodClient> = second;
    assert!(Rc::ptr_eq(
        f.native.client.borrow().as_ref().unwrap(),
        &expected_client
    ));
    assert_eq!(f.native.cursor.get(), expected);
    assert_eq!(f.owner.text(), "abcd");
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_input_cursor_getter_cannot_overwrite_a_same_owner_client_replacement() {
    let f = Fixture::new(true);
    let first = CursorQueryClient::new(&f.owner, Rect::new(11.0, 13.0, 1.0, 17.0));
    let current = provide_client(&f.owner, first.clone());
    let second = CursorQueryClient::new(&f.owner, Rect::new(31.0, 27.0, 2.0, 19.0));
    let expected = in_root(&f.owner, second.cursor.get());
    let old_connection = f.connection();
    let expected_client: Rc<dyn TextInputMethodClient> = second;
    let replacement = expected_client.clone();
    let owner = f.owner.clone();
    first.query.replace(Some(Box::new(move || {
        current.replace(replacement);
        owner.raise_event(&TextInputMethodClientRequeryRequestedEventArgs::with_event());
    })));
    first.raise_cursor_rectangle_changed();
    assert!(f.owner.is_focused());
    assert!(!old_connection.is_current());
    assert_eq!(
        f.connection().owner(),
        Some(f.owner.clone().upcast::<InputElement>())
    );
    assert!(Rc::ptr_eq(
        f.native.client.borrow().as_ref().unwrap(),
        &expected_client
    ));
    assert_eq!(f.native.cursor.get(), expected);
}

#[test]
fn rich_input_cursor_reset_focus_roundtrip_and_nested_geometry_keep_the_latest_caret() {
    for transition in 0..3 {
        let f = Fixture::new(true);
        let client = CursorQueryClient::new(&f.owner, Rect::new(11.0, 13.0, 1.0, 17.0));
        provide_client(&f.owner, client.clone());
        let old_connection = f.connection();
        let latest = Rect::new(31.0, 27.0, 2.0, 19.0);
        let expected = in_root(&f.owner, latest);
        let callback_client = Rc::downgrade(&client);
        let owner = f.owner.clone();
        let outside = f.outside.clone();
        client.query.replace(Some(Box::new(move || {
            let client = callback_client.upgrade().unwrap();
            client.cursor.set(latest);
            match transition {
                0 => client.request_reset(),
                1 => {
                    assert!(outside.focus());
                    assert!(owner.focus());
                }
                _ => client.raise_cursor_rectangle_changed(),
            }
        })));
        client.raise_cursor_rectangle_changed();
        assert!(f.owner.is_focused());
        let expected_client: Rc<dyn TextInputMethodClient> = client;
        assert!(Rc::ptr_eq(
            f.native.client.borrow().as_ref().unwrap(),
            &expected_client
        ));
        assert_eq!(f.native.cursor.get(), expected, "transition {transition}");
        if transition == 2 {
            assert!(old_connection.is_current());
            assert!(Rc::ptr_eq(&old_connection, &f.connection()));
        } else {
            assert!(!old_connection.is_current());
            assert!(!Rc::ptr_eq(&old_connection, &f.connection()));
        }
    }
}
