// Not from upstream: the reference has no tests of the integration. The CoreText
// callbacks of the adapter driven with test doubles of the edit context and of the
// event arguments: a client without the exact-edit connection keeps the results and
// notifications of the reference, and a client of the connection (a RichEditBox)
// edits through it (`docs/porting/winui-rich-text.md`, decision D5).
mod core_text_tests {
    use super::*;
    use ferroui_base::input::text_input::{TextInputContentType, TextInputMethodClientEvents};
    use ferroui_base::input::InputScopeNameValue;
    use ferroui_base::media::text_formatting::testing::TextTestScope;
    use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope};
    use ferroui_controls::{Control, RichEditBox, Window};
    use ferroui_microcom::{make_com, Guid};
    use ferroui_win32::win_rt::NativeWinRTMethods;

    #[derive(Clone, Debug, PartialEq)]
    enum Native {
        Scope(CoreTextInputScope),
        TextChanged(CoreTextRange, i32, CoreTextRange),
        SelectionChanged(CoreTextRange),
        LayoutChanged,
        FocusEnter,
        FocusLeave,
    }

    type Calls = Rc<RefCell<Vec<Native>>>;

    macro_rules! inspectable {
        ($type:ty) => {
            impl IInspectableImpl for $type {
                fn get_iids(&self, _: *mut u32) -> Result<*mut Guid, HResult> {
                    Err(HResult::NOTIMPL)
                }
                fn get_runtime_class_name(&self) -> Result<isize, HResult> {
                    Err(HResult::NOTIMPL)
                }
                fn get_trust_level(&self) -> Result<TrustLevel, HResult> {
                    Ok(TrustLevel::BaseTrust)
                }
            }
        };
    }

    /// A `CoreTextEditContext` that records the notifications and scopes it is given.
    struct FakeEditContext(Calls);

    inspectable!(FakeEditContext);

    const TOKEN: EventRegistrationToken = EventRegistrationToken { value: 1 };

    impl ICoreTextEditContextImpl for FakeEditContext {
        fn name(&self) -> Result<isize, HResult> {
            Ok(0)
        }
        fn set_name(&self, _: isize) -> Result<(), HResult> {
            Ok(())
        }
        fn input_scope(&self) -> Result<CoreTextInputScope, HResult> {
            Err(HResult::NOTIMPL)
        }
        fn set_input_scope(&self, value: CoreTextInputScope) -> Result<(), HResult> {
            self.0.borrow_mut().push(Native::Scope(value));
            Ok(())
        }
        fn is_read_only(&self) -> Result<u8, HResult> {
            Ok(0)
        }
        fn set_is_read_only(&self, _: u8) -> Result<(), HResult> {
            Ok(())
        }
        fn input_pane_display_policy(&self) -> Result<CoreTextInputPaneDisplayPolicy, HResult> {
            Ok(CoreTextInputPaneDisplayPolicy::Automatic)
        }
        fn set_input_pane_display_policy(&self, _: CoreTextInputPaneDisplayPolicy) -> Result<(), HResult> {
            Ok(())
        }
        fn add_text_requested(&self, _: Option<&TypedEventHandlerTextRequested>) -> Result<EventRegistrationToken, HResult> {
            Ok(TOKEN)
        }
        fn remove_text_requested(&self, _: EventRegistrationToken) -> Result<(), HResult> {
            Ok(())
        }
        fn add_selection_requested(&self, _: Option<&TypedEventHandlerSelectionRequested>) -> Result<EventRegistrationToken, HResult> {
            Ok(TOKEN)
        }
        fn remove_selection_requested(&self, _: EventRegistrationToken) -> Result<(), HResult> {
            Ok(())
        }
        fn add_layout_requested(&self, _: Option<&TypedEventHandlerLayoutRequested>) -> Result<EventRegistrationToken, HResult> {
            Ok(TOKEN)
        }
        fn remove_layout_requested(&self, _: EventRegistrationToken) -> Result<(), HResult> {
            Ok(())
        }
        fn add_text_updating(&self, _: Option<&TypedEventHandlerTextUpdating>) -> Result<EventRegistrationToken, HResult> {
            Ok(TOKEN)
        }
        fn remove_text_updating(&self, _: EventRegistrationToken) -> Result<(), HResult> {
            Ok(())
        }
        fn add_selection_updating(&self, _: Option<&TypedEventHandlerSelectionUpdating>) -> Result<EventRegistrationToken, HResult> {
            Ok(TOKEN)
        }
        fn remove_selection_updating(&self, _: EventRegistrationToken) -> Result<(), HResult> {
            Ok(())
        }
        fn add_format_updating(&self, _: Option<&TypedEventHandlerFormatUpdating>) -> Result<EventRegistrationToken, HResult> {
            Ok(TOKEN)
        }
        fn remove_format_updating(&self, _: EventRegistrationToken) -> Result<(), HResult> {
            Ok(())
        }
        fn add_composition_started(&self, _: Option<&TypedEventHandlerCompositionStarted>) -> Result<EventRegistrationToken, HResult> {
            Ok(TOKEN)
        }
        fn remove_composition_started(&self, _: EventRegistrationToken) -> Result<(), HResult> {
            Ok(())
        }
        fn add_composition_completed(&self, _: Option<&TypedEventHandlerCompositionCompleted>) -> Result<EventRegistrationToken, HResult> {
            Ok(TOKEN)
        }
        fn remove_composition_completed(&self, _: EventRegistrationToken) -> Result<(), HResult> {
            Ok(())
        }
        fn add_focus_removed(&self, _: Option<&TypedEventHandlerFocusRemoved>) -> Result<EventRegistrationToken, HResult> {
            Ok(TOKEN)
        }
        fn remove_focus_removed(&self, _: EventRegistrationToken) -> Result<(), HResult> {
            Ok(())
        }
        fn notify_focus_enter(&self) -> Result<(), HResult> {
            self.0.borrow_mut().push(Native::FocusEnter);
            Ok(())
        }
        fn notify_focus_leave(&self) -> Result<(), HResult> {
            self.0.borrow_mut().push(Native::FocusLeave);
            Ok(())
        }
        fn notify_text_changed(&self, modified_range: CoreTextRange, new_length: i32, new_selection: CoreTextRange) -> Result<(), HResult> {
            self.0.borrow_mut().push(Native::TextChanged(modified_range, new_length, new_selection));
            Ok(())
        }
        fn notify_selection_changed(&self, selection: CoreTextRange) -> Result<(), HResult> {
            self.0.borrow_mut().push(Native::SelectionChanged(selection));
            Ok(())
        }
        fn notify_layout_changed(&self) -> Result<(), HResult> {
            self.0.borrow_mut().push(Native::LayoutChanged);
            Ok(())
        }
    }

    /// `CoreTextTextUpdatingEventArgs`: the replacement of `range` by `text` with the
    /// selection after it. The result starts as the system's default, `Succeeded`.
    struct FakeTextUpdating {
        range: CoreTextRange,
        text: String,
        new_selection: CoreTextRange,
        result: Cell<CoreTextTextUpdatingResult>,
    }

    inspectable!(FakeTextUpdating);

    impl ICoreTextTextUpdatingEventArgsImpl for FakeTextUpdating {
        fn range(&self) -> Result<CoreTextRange, HResult> {
            Ok(self.range)
        }
        fn text(&self) -> Result<isize, HResult> {
            // A new handle the caller owns, as the system returns it.
            NativeWinRTMethods::windows_create_string(&self.text)
        }
        fn new_selection(&self) -> Result<CoreTextRange, HResult> {
            Ok(self.new_selection)
        }
        fn input_language(&self) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            Ok(None)
        }
        fn result(&self) -> Result<CoreTextTextUpdatingResult, HResult> {
            Ok(self.result.get())
        }
        fn set_result(&self, value: CoreTextTextUpdatingResult) -> Result<(), HResult> {
            self.result.set(value);
            Ok(())
        }
        fn is_canceled(&self) -> Result<u8, HResult> {
            Ok(0)
        }
        fn get_deferral(&self) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            Ok(None)
        }
    }

    struct FakeSelectionUpdating {
        selection: CoreTextRange,
        result: Cell<CoreTextSelectionUpdatingResult>,
    }

    inspectable!(FakeSelectionUpdating);

    impl ICoreTextSelectionUpdatingEventArgsImpl for FakeSelectionUpdating {
        fn selection(&self) -> Result<CoreTextRange, HResult> {
            Ok(self.selection)
        }
        fn result(&self) -> Result<CoreTextSelectionUpdatingResult, HResult> {
            Ok(self.result.get())
        }
        fn set_result(&self, value: CoreTextSelectionUpdatingResult) -> Result<(), HResult> {
            self.result.set(value);
            Ok(())
        }
        fn is_canceled(&self) -> Result<u8, HResult> {
            Ok(0)
        }
        fn get_deferral(&self) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            Ok(None)
        }
    }

    struct FakeFormatUpdating {
        result: Cell<CoreTextFormatUpdatingResult>,
    }

    inspectable!(FakeFormatUpdating);

    impl ICoreTextFormatUpdatingEventArgsImpl for FakeFormatUpdating {
        fn range(&self) -> Result<CoreTextRange, HResult> {
            Ok(range(0, 0))
        }
        fn text_color(&self) -> Result<*mut ::core::ffi::c_void, HResult> {
            Ok(std::ptr::null_mut())
        }
        fn background_color(&self) -> Result<*mut ::core::ffi::c_void, HResult> {
            Ok(std::ptr::null_mut())
        }
        fn underline_color(&self) -> Result<*mut ::core::ffi::c_void, HResult> {
            Ok(std::ptr::null_mut())
        }
        fn underline_type(&self) -> Result<*mut ::core::ffi::c_void, HResult> {
            Ok(std::ptr::null_mut())
        }
        fn reason(&self) -> Result<i32, HResult> {
            Ok(0)
        }
        fn result(&self) -> Result<CoreTextFormatUpdatingResult, HResult> {
            Ok(self.result.get())
        }
        fn set_result(&self, value: CoreTextFormatUpdatingResult) -> Result<(), HResult> {
            self.result.set(value);
            Ok(())
        }
        fn is_canceled(&self) -> Result<u8, HResult> {
            Ok(0)
        }
        fn get_deferral(&self) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            Ok(None)
        }
    }

    struct FakeCompositionStarted;

    inspectable!(FakeCompositionStarted);

    impl ICoreTextCompositionStartedEventArgsImpl for FakeCompositionStarted {
        fn is_canceled(&self) -> Result<u8, HResult> {
            Ok(0)
        }
        fn get_deferral(&self) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            Ok(None)
        }
    }

    struct FakeCompositionCompleted;

    inspectable!(FakeCompositionCompleted);

    impl ICoreTextCompositionCompletedEventArgsImpl for FakeCompositionCompleted {
        fn is_canceled(&self) -> Result<u8, HResult> {
            Ok(0)
        }
        fn composition_segments(&self) -> Result<*mut ::core::ffi::c_void, HResult> {
            Ok(std::ptr::null_mut())
        }
        fn get_deferral(&self) -> Result<Option<ComPtr<IInspectable>>, HResult> {
            Ok(None)
        }
    }

    fn text_updating(start: i32, end: i32, text: &str, caret: i32) -> (Rc<FakeTextUpdating>, ComPtr<ICoreTextTextUpdatingEventArgs>) {
        let args = Rc::new(FakeTextUpdating {
            range: range(start, end),
            text: text.to_owned(),
            new_selection: range(caret, caret),
            result: Cell::new(CoreTextTextUpdatingResult::Succeeded),
        });
        let com = make_com(args.clone());
        (args, com)
    }

    fn selection_updating(start: i32, end: i32) -> (Rc<FakeSelectionUpdating>, ComPtr<ICoreTextSelectionUpdatingEventArgs>) {
        let args = Rc::new(FakeSelectionUpdating {
            selection: range(start, end),
            result: Cell::new(CoreTextSelectionUpdatingResult::Succeeded),
        });
        let com = make_com(args.clone());
        (args, com)
    }

    fn format_updating() -> (Rc<FakeFormatUpdating>, ComPtr<ICoreTextFormatUpdatingEventArgs>) {
        let args = Rc::new(FakeFormatUpdating {
            result: Cell::new(CoreTextFormatUpdatingResult::Succeeded),
        });
        let com = make_com(args.clone());
        (args, com)
    }

    /// An adapter with a recording edit context and no panel or input sink.
    fn method_with_context() -> (Rc<WinUITextInputMethod>, ComPtr<ICoreTextEditContext>, Calls) {
        let method = WinUITextInputMethod::new(Weak::new(), Box::new(|| None), Box::new(|| None), Box::new(|| None));
        let calls = Calls::default();
        let context: ComPtr<ICoreTextEditContext> = make_com(FakeEditContext(calls.clone()));
        method.edit_context.replace(Some(context.clone()));
        (method, context, calls)
    }

    /// A text client without the exact-edit connection, as every TextBox.
    struct LegacyClient {
        events: TextInputMethodClientEvents,
        text: RefCell<String>,
        selection: Cell<TextSelection>,
        preedit: RefCell<Vec<Option<String>>>,
    }

    impl LegacyClient {
        fn new(text: &str, caret: i32) -> Rc<Self> {
            Rc::new(Self {
                events: TextInputMethodClientEvents::default(),
                text: RefCell::new(text.to_owned()),
                selection: Cell::new(TextSelection::new(caret, caret)),
                preedit: RefCell::new(Vec::new()),
            })
        }
    }

    impl TextInputMethodClient for LegacyClient {
        fn events(&self) -> &TextInputMethodClientEvents {
            &self.events
        }
        fn text_view_visual(&self) -> Option<Ref<Visual>> {
            None
        }
        fn supports_preedit(&self) -> bool {
            true
        }
        fn supports_surrounding_text(&self) -> bool {
            true
        }
        fn surrounding_text(&self) -> String {
            self.text.borrow().clone()
        }
        fn cursor_rectangle(&self) -> FerroRect {
            FerroRect::default()
        }
        fn selection(&self) -> TextSelection {
            self.selection.get()
        }
        fn set_selection(&self, selection: TextSelection) {
            self.selection.set(selection);
        }
        fn set_preedit_text(&self, text: Option<&str>) {
            self.preedit.borrow_mut().push(text.map(str::to_owned));
        }
    }

    #[test]
    fn legacy_client_is_synchronized_with_the_whole_text_as_the_reference() {
        let (method, _context, calls) = method_with_context();
        method.set_client(Some(LegacyClient::new("abc", 3)));
        assert!(!method.exact_client.get());
        assert!(method.connection.borrow().is_none());
        // `SyncFromClient(notifyServer: true)`: the whole text, then the layout.
        assert_eq!(*calls.borrow(), [Native::TextChanged(range(0, i32::MAX), 3, range(3, 3)), Native::LayoutChanged]);
        calls.borrow_mut().clear();
        method.set_cursor_rect(FerroRect::new(1.0, 2.0, 1.0, 16.0));
        assert_eq!(*calls.borrow(), [Native::LayoutChanged]);
    }

    #[test]
    fn legacy_text_and_selection_updates_succeed_as_the_reference() {
        let (method, context, _calls) = method_with_context();
        let client = LegacyClient::new("abc", 3);
        method.set_client(Some(client.clone()));

        // Outside a composition, the text goes to the input sink (none here) and the
        // update is acknowledged.
        let (args, com) = text_updating(3, 3, "d", 4);
        args.result.set(CoreTextTextUpdatingResult::Failed);
        method.on_text_updating(&context, &com);
        assert_eq!(args.result.get(), CoreTextTextUpdatingResult::Succeeded);
        assert_eq!(String::from_utf16_lossy(&method.ime_text.borrow()), "abcd");

        let (args, com) = selection_updating(1, 2);
        args.result.set(CoreTextSelectionUpdatingResult::Failed);
        method.on_selection_updating(&context, &com);
        assert_eq!(args.result.get(), CoreTextSelectionUpdatingResult::Succeeded);
        assert_eq!(client.selection.get(), TextSelection::new(1, 2));

        // A format request is acknowledged without being applied.
        let (args, com) = format_updating();
        args.result.set(CoreTextFormatUpdatingResult::Failed);
        method.on_format_updating(&context, &com);
        assert_eq!(args.result.get(), CoreTextFormatUpdatingResult::Succeeded);
    }

    #[test]
    fn legacy_composition_survives_the_system_focus_removal_as_the_reference() {
        let (method, context, _calls) = method_with_context();
        let client = LegacyClient::new("abc", 3);
        method.set_client(Some(client.clone()));
        let started: ComPtr<ICoreTextCompositionStartedEventArgs> = make_com(FakeCompositionStarted);
        method.on_composition_started(&context, &started);
        let (_, com) = text_updating(3, 3, "x", 4);
        method.on_text_updating(&context, &com);
        assert_eq!(*client.preedit.borrow(), [Some("x".to_owned())]);
        method.on_focus_removed(&context, None);
        assert!(method.is_composing.get());
        assert_eq!(*client.preedit.borrow(), [Some("x".to_owned())]);
    }

    #[test]
    fn legacy_options_map_as_the_reference() {
        let (method, _context, calls) = method_with_context();
        method.set_client(Some(LegacyClient::new("", 0)));
        calls.borrow_mut().clear();
        for content_type in [TextInputContentType::Pin, TextInputContentType::Name, TextInputContentType::Email] {
            method.set_options(&TextInputOptions { content_type, ..TextInputOptions::default() });
        }
        assert_eq!(
            *calls.borrow(),
            [
                Native::Scope(CoreTextInputScope::Text),
                Native::Scope(CoreTextInputScope::Text),
                Native::Scope(CoreTextInputScope::EmailAddress),
            ]
        );
    }

    /// A focused RichEditBox, a client of the exact-edit connection, on the adapter.
    struct RichFixture {
        window: Ref<Window>,
        owner: Ref<RichEditBox>,
        method: Rc<WinUITextInputMethod>,
        context: ComPtr<ICoreTextEditContext>,
        calls: Calls,
        _text: TextTestScope,
        _app: UnitTestApplicationScope,
    }

    impl RichFixture {
        fn new(configure: impl FnOnce(&Ref<RichEditBox>)) -> Self {
            let (method, context, calls) = method_with_context();
            let im = method.clone();
            let app = UnitTestApplication::start(TestServices::focusable_window().with_windowing_platform(
                MockWindowingPlatform::with_window_impl(move || {
                    let window = MockWindowingPlatform::create_window_mock();
                    window.setup_feature::<dyn ITextInputMethodImpl>(im.clone());
                    window
                }),
            ));
            let text = TextTestScope::new();
            let owner = RichEditBox::new();
            configure(&owner);
            let window = Window::new();
            window.set_content(Some(Control::boxed(owner.clone())));
            window.show();
            owner.focus();
            assert!(owner.is_focused());
            assert!(method.exact_client.get());
            Self { window, owner, method, context, calls, _text: text, _app: app }
        }

        fn type_text(&self, start: i32, text: &str) -> CoreTextTextUpdatingResult {
            let caret = start + text.encode_utf16().count() as i32;
            let (args, com) = text_updating(start, start, text, caret);
            self.method.on_text_updating(&self.context, &com);
            args.result.get()
        }
    }

    impl Drop for RichFixture {
        fn drop(&mut self) {
            self.window.close();
        }
    }

    #[test]
    fn connection_client_text_updates_are_exact_edits_and_typing_is_one_undo_unit() {
        let f = RichFixture::new(|_| {});
        for (index, character) in ["a", "b", "c"].into_iter().enumerate() {
            assert_eq!(f.type_text(index as i32, character), CoreTextTextUpdatingResult::Succeeded);
        }
        assert_eq!(f.owner.text(), "abc");
        assert_eq!(String::from_utf16_lossy(&f.method.ime_text.borrow()), "abc");
        // #243 review M6: the characters typed outside a composition are one undo unit.
        assert!(f.owner.undo().unwrap());
        assert_eq!(f.owner.text(), "");
        assert!(!f.owner.undo().unwrap());
    }

    #[test]
    fn connection_client_composition_edits_exactly_and_commits_one_undo_unit() {
        let f = RichFixture::new(|owner| owner.set_text("ab").unwrap());
        f.owner.select(2, 2);
        f.owner.editor().clear_history();
        let started: ComPtr<ICoreTextCompositionStartedEventArgs> = make_com(FakeCompositionStarted);
        f.method.on_composition_started(&f.context, &started);
        assert!(f.method.is_composing.get());
        assert!(f.method.exact_composition.borrow().is_some());
        for (start, end, text, caret) in [(2, 2, "k", 3), (2, 3, "ka", 4), (2, 4, "\u{304B}", 3)] {
            let (args, com) = text_updating(start, end, text, caret);
            f.method.on_text_updating(&f.context, &com);
            assert_eq!(args.result.get(), CoreTextTextUpdatingResult::Succeeded, "{text}");
        }
        assert_eq!(f.owner.text(), "ab\u{304B}");
        assert!(!f.owner.can_undo());
        let completed: ComPtr<ICoreTextCompositionCompletedEventArgs> = make_com(FakeCompositionCompleted);
        f.method.on_composition_completed(&f.context, &completed);
        assert!(!f.method.is_composing.get());
        assert_eq!(f.owner.text(), "ab\u{304B}");
        assert!(f.owner.undo().unwrap());
        assert_eq!(f.owner.text(), "ab");
    }

    #[test]
    fn connection_client_rejects_an_update_against_a_stale_state() {
        let f = RichFixture::new(|_| {});
        assert_eq!(f.type_text(0, "a"), CoreTextTextUpdatingResult::Succeeded);
        // An edit of the text that CoreText was not told about yet.
        f.owner.set_text("xyz").unwrap();
        let (args, com) = text_updating(1, 1, "b", 2);
        f.method.synchronization.borrow_mut().update(TextInputMethodState {
            text: "a".to_owned(),
            selection: TextSelection::new(1, 1),
        });
        f.method.on_text_updating(&f.context, &com);
        assert_eq!(args.result.get(), CoreTextTextUpdatingResult::Failed);
        assert_eq!(f.owner.text(), "xyz");
    }

    #[test]
    fn connection_client_format_updates_are_not_acknowledged() {
        let f = RichFixture::new(|_| {});
        let (args, com) = format_updating();
        f.method.on_format_updating(&f.context, &com);
        assert_eq!(args.result.get(), CoreTextFormatUpdatingResult::Failed);
    }

    #[test]
    fn connection_client_selection_updates_select_exactly() {
        let f = RichFixture::new(|owner| owner.set_text("abcd").unwrap());
        let (args, com) = selection_updating(3, 1);
        f.method.on_selection_updating(&f.context, &com);
        assert_eq!(args.result.get(), CoreTextSelectionUpdatingResult::Succeeded);
        assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (3, 1));
    }

    #[test]
    fn connection_client_scopes_and_content_types_reach_the_edit_context() {
        let f = RichFixture::new(|owner| {
            TextInputOptions::set_input_scope_names(owner, Some(Rc::from([InputScopeNameValue::Hiragana])));
        });
        assert_eq!(f.calls.borrow().iter().rev().find(|call| matches!(call, Native::Scope(_))), Some(&Native::Scope(CoreTextInputScope::Hiragana)));
        f.calls.borrow_mut().clear();
        // Without a scope, `Pin` has the native scope of its own for a connection client.
        TextInputOptions::set_input_scope_names(&f.owner, None);
        TextInputOptions::set_content_type(&f.owner, TextInputContentType::Pin);
        assert_eq!(f.calls.borrow().iter().rev().find(|call| matches!(call, Native::Scope(_))), Some(&Native::Scope(CoreTextInputScope::PinNumeric)));
    }
}
