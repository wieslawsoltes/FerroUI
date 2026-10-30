//! Attached input policies follow the semantic owner of a client of the exact-edit
//! connection for the lifetime of its binding, including inherited changes, reset and
//! synchronous backend callbacks. A legacy client keeps the upstream behaviour: its
//! options are published when it is installed and on a reset only.
use super::*;

#[test]
fn focused_input_options_change_without_replacing_the_client_or_connection() {
    let tree = ime_tree_with_connection();
    tree.focus(Some(&tree.editor));
    let connection = tree.client.input_method_connection().unwrap();
    tree.take_log();

    TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Email);
    TextInputOptions::set_return_key_type(&tree.editor, TextInputReturnKeyType::Search);
    TextInputOptions::set_multiline(&tree.editor, true);
    TextInputOptions::set_lowercase(&tree.editor, true);
    TextInputOptions::set_uppercase(&tree.editor, true);
    TextInputOptions::set_auto_capitalization(&tree.editor, true);
    TextInputOptions::set_is_sensitive(&tree.editor, true);
    TextInputOptions::set_show_suggestions(&tree.editor, Some(false));
    TextInputOptions::set_locale_hints(&tree.editor, Some(Rc::from(vec!["pl-PL".to_owned()])));

    assert_eq!(
        tree.im.options.borrow().as_ref(),
        Some(&TextInputOptions {
            content_type: TextInputContentType::Email,
            return_key_type: TextInputReturnKeyType::Search,
            multiline: true,
            lowercase: true,
            uppercase: true,
            auto_capitalization: true,
            is_sensitive: true,
            show_suggestions: Some(false),
            locale_hints: Some(Rc::from(vec!["pl-PL".to_owned()])),
        })
    );
    assert_eq!(tree.take_log(), vec!["im options Email"; 9]);
    assert!(Rc::ptr_eq(
        &connection,
        &tree.client.input_method_connection().unwrap()
    ));
    assert!(connection.is_current());
}

#[test]
fn focused_input_options_follow_inheritance_and_clearing_local_values() {
    let tree = ime_tree_with_connection();
    tree.focus(Some(&tree.editor));
    tree.take_log();

    TextInputOptions::set_content_type(&tree.root.root, TextInputContentType::Email);
    assert_eq!(tree.take_log(), ["im options Email"]);
    TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Number);
    assert_eq!(tree.take_log(), ["im options Number"]);
    // A local value masks later inherited changes. Unrelated properties and
    // assigning an equal effective value do not resend input policies either.
    TextInputOptions::set_content_type(&tree.root.root, TextInputContentType::Url);
    TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Number);
    tree.editor.set_opacity(0.5);
    assert!(tree.take_log().is_empty());

    tree.editor
        .clear_value(TextInputOptions::content_type_property());
    assert_eq!(tree.take_log(), ["im options Url"]);
    assert_eq!(
        tree.im.options.borrow().as_ref().unwrap().content_type,
        TextInputContentType::Url
    );
    TextInputOptions::set_show_suggestions(&tree.root.root, Some(false));
    assert_eq!(tree.take_log(), ["im options Url"]);
    assert_eq!(
        tree.im.options.borrow().as_ref().unwrap().show_suggestions,
        Some(false)
    );
}

#[test]
fn focused_input_options_continue_after_reset_and_stop_when_the_owner_is_replaced() {
    let tree = ime_tree_with_connection();
    let other = TestTextClient::connected("button", &tree.button);
    offer_client(&tree.button, &other);
    TextInputOptions::set_content_type(&tree.button, TextInputContentType::Search);
    tree.focus(Some(&tree.editor));
    let original = tree.client.input_method_connection().unwrap();

    tree.client.request_reset();
    let reset = tree.client.input_method_connection().unwrap();
    assert!(!original.is_current());
    assert!(reset.is_current());
    tree.take_log();
    TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Email);
    assert_eq!(tree.take_log(), ["im options Email"]);

    tree.focus(Some(&tree.button));
    assert!(!reset.is_current());
    tree.take_log();
    TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Url);
    assert!(tree.take_log().is_empty());
    TextInputOptions::set_show_suggestions(&tree.button, Some(false));
    assert_eq!(tree.take_log(), ["im options Search"]);
    assert_eq!(
        tree.im.options.borrow().as_ref().unwrap().show_suggestions,
        Some(false)
    );
}

/// Deliberately completes its old option write after invoking application code,
/// as a native setter that synchronously pumps callbacks can do.
struct ReentrantOptionsInputMethod {
    inner: Rc<TestInputMethod>,
    on_options: RefCell<Option<Box<dyn Fn()>>>,
}

impl ReentrantOptionsInputMethod {
    fn install(tree: &ImeTree) -> Rc<Self> {
        let backend = Rc::new(Self {
            inner: tree.im.clone(),
            on_options: RefCell::new(None),
        });
        *tree.root.host.input_method.borrow_mut() = Some(backend.clone());
        backend
    }
}

impl ITextInputMethodImpl for ReentrantOptionsInputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        self.inner.set_client(client);
    }
    fn set_cursor_rect(&self, rect: Rect) {
        self.inner.set_cursor_rect(rect);
    }
    fn set_options(&self, options: &TextInputOptions) {
        let callback = self.on_options.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        self.inner.set_options(options);
    }
    fn reset(&self) {
        self.inner.reset();
    }
}

#[test]
fn input_option_setter_reentry_does_not_restore_old_options_or_client() {
    for initial_binding in [false, true] {
        let tree = ime_tree_with_connection();
        let backend = ReentrantOptionsInputMethod::install(&tree);
        let other = TestTextClient::connected("button", &tree.button);
        offer_client(&tree.button, &other);
        TextInputOptions::set_content_type(&tree.button, TextInputContentType::Search);
        if !initial_binding {
            tree.focus(Some(&tree.editor));
        }
        tree.take_log();
        let keyboard = tree.keyboard.clone();
        let target = tree.button.clone().upcast::<InputElement>();
        *backend.on_options.borrow_mut() = Some(Box::new(move || {
            keyboard.set_focused_element(
                Some(&target),
                NavigationMethod::Unspecified,
                KeyModifiers::NONE,
            );
        }));

        TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Email);
        if initial_binding {
            tree.focus(Some(&tree.editor));
        }
        let log = tree.take_log();
        assert_eq!(log.last().map(String::as_str), Some("im options Search"));
        assert!(!log.iter().any(|entry| entry == "im client editor"));
        let installed = tree.im.client.borrow().clone().unwrap();
        assert!(std::ptr::addr_eq(
            Rc::as_ptr(&installed),
            Rc::as_ptr(&other)
        ));
        assert_eq!(
            tree.im.options.borrow().as_ref().unwrap().content_type,
            TextInputContentType::Search
        );
        assert!(tree.client.input_method_connection().is_none());
        assert!(other.input_method_connection().unwrap().is_current());

        TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Url);
        assert!(tree.take_log().is_empty());
        TextInputOptions::set_content_type(&tree.button, TextInputContentType::Digits);
        assert_eq!(tree.take_log(), ["im options Digits"]);
    }
}

#[test]
fn input_option_setter_reentry_keeps_the_latest_policies_of_the_same_owner() {
    let tree = ime_tree_with_connection();
    let backend = ReentrantOptionsInputMethod::install(&tree);
    tree.focus(Some(&tree.editor));
    let connection = tree.client.input_method_connection().unwrap();
    tree.take_log();
    let owner = tree.editor.clone();
    *backend.on_options.borrow_mut() = Some(Box::new(move || {
        TextInputOptions::set_content_type(&owner, TextInputContentType::Search);
        TextInputOptions::set_show_suggestions(&owner, Some(false));
    }));

    TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Email);
    let log = tree.take_log();
    assert_eq!(log.last().map(String::as_str), Some("im options Search"));
    assert!(log.iter().all(|entry| entry.starts_with("im options ")));
    let options = tree.im.options.borrow().clone().unwrap();
    assert_eq!(options.content_type, TextInputContentType::Search);
    assert_eq!(options.show_suggestions, Some(false));
    assert!(Rc::ptr_eq(
        &connection,
        &tree.client.input_method_connection().unwrap()
    ));
    assert!(connection.is_current());
}

#[test]
fn stale_option_notification_does_not_publish_for_a_replaced_owner() {
    let tree = ime_tree_with_connection();
    let other = TestTextClient::connected("button", &tree.button);
    offer_client(&tree.button, &other);
    TextInputOptions::set_content_type(&tree.button, TextInputContentType::Search);
    let keyboard = tree.keyboard.clone();
    let target = tree.button.clone().upcast::<InputElement>();
    // This observer precedes the manager's observer in the event snapshot.
    let observer = tree.editor.property_changed(move |event| {
        if event.property().id() == TextInputOptions::content_type_property().as_property().id() {
            keyboard.set_focused_element(
                Some(&target),
                NavigationMethod::Unspecified,
                KeyModifiers::NONE,
            );
        }
    });
    tree.focus(Some(&tree.editor));
    tree.take_log();

    TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Email);
    let log = tree.take_log();
    assert!(!log.iter().any(|entry| entry == "im options Email"));
    assert_eq!(
        log.iter()
            .filter(|entry| *entry == "im options Search")
            .count(),
        1
    );
    assert_eq!(
        tree.im.options.borrow().as_ref().unwrap().content_type,
        TextInputContentType::Search
    );
    observer.dispose();
}

#[test]
fn disposing_the_input_manager_releases_the_owner_option_subscription() {
    let tree = ime_tree_with_connection();
    let manager = text_input::TextInputMethodManager::new();
    let weak = Rc::downgrade(&manager);
    let subscribers = tree.editor.property_changed_subscriber_count();
    manager.set_focused_element(Some(tree.editor.clone().upcast()));
    assert!(tree.editor.property_changed_subscriber_count() > subscribers);

    manager.dispose();
    assert_eq!(tree.editor.property_changed_subscriber_count(), subscribers);
    tree.take_log();
    TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Email);
    assert!(tree.take_log().is_empty());
    drop(manager);
    assert!(weak.upgrade().is_none());
}

#[test]
fn input_scope_snapshot_setter_reentry_keeps_latest_names_and_connection() {
    use crate::input::InputScopeNameValue as Scope;
    let tree = ime_tree_with_connection();
    let backend = ReentrantOptionsInputMethod::install(&tree);
    tree.focus(Some(&tree.editor));
    let connection = tree.client.input_method_connection().unwrap();
    let owner = tree.editor.clone();
    *backend.on_options.borrow_mut() = Some(Box::new(move || {
        TextInputOptions::set_input_scope_names(&owner, Some(Rc::from([Scope::Hiragana, Scope::Text])));
    }));
    tree.take_log();
    TextInputOptions::set_input_scope_names(&tree.editor, Some(Rc::from([Scope::Url])));
    // Both changes republish the options (the newer one again after the older call
    // returns); the backend reads the scopes through the connection, which sees the
    // latest names.
    let log = tree.take_log();
    assert!(log.len() >= 2 && log.iter().all(|entry| entry == "im options Normal"), "{log:?}");
    assert_eq!(connection.input_scope_names().as_deref(), Some(&[Scope::Hiragana, Scope::Text][..]));
    assert!(Rc::ptr_eq(&connection, &tree.client.input_method_connection().unwrap()));
    assert!(connection.is_current());
}

#[test]
fn input_scope_snapshot_inheritance_and_clear_reach_the_focused_backend() {
    use crate::input::InputScopeNameValue as Scope;
    let tree = ime_tree_with_connection();
    tree.focus(Some(&tree.editor));
    let connection = tree.client.input_method_connection().unwrap();
    tree.take_log();
    TextInputOptions::set_input_scope_names(&tree.root.root, Some(Rc::from([Scope::EmailSmtpAddress])));
    assert_eq!(tree.take_log(), ["im options Normal"]);
    assert_eq!(connection.effective_input_scope(), Some(Scope::EmailSmtpAddress));
    let options = tree.im.options.borrow().clone().unwrap();
    assert_eq!(connection.effective_content_type(&options), TextInputContentType::Email);
    TextInputOptions::set_input_scope_names(&tree.editor, Some(Rc::from([Scope::Url])));
    TextInputOptions::set_input_scope_names(&tree.root.root, Some(Rc::from([Scope::ChatWithoutEmoji])));
    // The inherited change is masked by the local value and publishes nothing.
    assert_eq!(tree.take_log(), ["im options Normal"]);
    assert_eq!(connection.effective_input_scope(), Some(Scope::Url));
    tree.editor.clear_value(TextInputOptions::input_scope_names_property());
    assert_eq!(tree.take_log(), ["im options Normal"]);
    assert_eq!(connection.effective_input_scope(), Some(Scope::ChatWithoutEmoji));
    assert_eq!(TextInputOptions::get_effective_input_scope(&tree.editor), Some(Scope::ChatWithoutEmoji));
    assert!(connection.is_current());
}

/// A legacy client (one without the exact-edit connection) keeps the upstream manager:
/// no connection, and its options are published when it is installed and on a reset,
/// not when they change.
#[test]
fn legacy_clients_keep_the_upstream_option_publication() {
    let tree = ime_tree();
    tree.focus(Some(&tree.editor));
    assert!(tree.client.input_method_connection().is_none());
    tree.take_log();

    TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Email);
    TextInputOptions::set_input_scope_names(&tree.editor, Some(Rc::from([crate::input::InputScopeNameValue::Url])));
    assert!(tree.take_log().is_empty());

    tree.client.request_reset();
    assert_eq!(
        tree.take_log(),
        ["im reset", "im options Email", "im client editor", "im cursor 21,32 3x4"]
    );
    assert!(tree.client.input_method_connection().is_none());
}

/// Focus that moves into the editing UI of a connection client (its menu, a flyout)
/// keeps the client, its owner and its connection. A legacy client is replaced as
/// upstream (`focusing_an_element_without_a_client_disconnects_the_input_method`).
#[test]
fn connection_clients_keep_their_binding_while_focus_is_in_their_editing_ui() {
    let tree = ime_tree_with_connection();
    tree.client.retained.borrow_mut().push(tree.button.clone().upcast());
    tree.focus(Some(&tree.editor));
    let connection = tree.client.input_method_connection().unwrap();
    tree.take_log();

    tree.focus(Some(&tree.button));
    assert!(tree.take_log().is_empty());
    assert!(connection.is_current());
    assert_eq!(connection.owner(), Some(tree.editor.clone().upcast()));

    // The options of the owner, not of the focused element, are published.
    TextInputOptions::set_content_type(&tree.button, TextInputContentType::Search);
    TextInputOptions::set_content_type(&tree.editor, TextInputContentType::Email);
    assert_eq!(tree.take_log(), ["im options Email"]);
}

/// A connection client is bound only while the owner it names can take input.
#[test]
fn connection_clients_are_not_bound_to_a_detached_owner() {
    let tree = ime_tree_with_connection();
    let detached = TestControl::focusable("detached");
    *tree.client.owner.borrow_mut() = Some(detached.clone().upcast());
    tree.focus(Some(&tree.editor));
    assert!(tree.im.client.borrow().is_none());
    assert!(tree.client.input_method_connection().is_none());

    *tree.client.owner.borrow_mut() = Some(tree.editor.clone().upcast());
    tree.client.request_reset();
    tree.focus(Some(&tree.button));
    tree.focus(Some(&tree.editor));
    assert!(tree.im.client.borrow().is_some());
    assert!(tree.client.input_method_connection().unwrap().is_current());
}
