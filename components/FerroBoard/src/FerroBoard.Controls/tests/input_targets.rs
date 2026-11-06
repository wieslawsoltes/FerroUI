#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::input::*;
use ferroboard_core::{
    editor::*, engine::*, layouts::KeyboardResources, settings::KeyboardSettings, text::Utf16Text,
};
use ferroui_base::input::text_input::{
    TextInputMethodClient, TextInputMethodClientRequestedEventArgs, TextSelection,
};
use ferroui_base::{FerroObjectExtensions, data::BindingPriority, input::InputElement};
use ferroui_base::{
    Ref,
    input::text_input::{TextInputContentType, TextInputOptions},
    threading::Dispatcher,
};
use ferroui_controls::{Border, Control, StackPanel, TextBox, Window};
use ferroui_controls::{
    presenters::TextPresenter,
    templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions},
};
use std::cell::RefCell;
use std::{cell::Cell, rc::Rc};

fn selection_template(field: &Ref<TextBox>) {
    field.set_template(Some(FuncControlTemplate::for_type::<TextBox>(
        |owner, scope| {
            let object: &ferroui_base::FerroObject = owner;
            let presenter = TextPresenter::new();
            presenter.set_name(Some("PART_TextPresenter".into()));
            presenter.bind_typed(
                TextPresenter::text_property(),
                FerroObjectExtensions::get_observable(object, TextBox::text_property()),
                BindingPriority::Template,
            );
            presenter.register_in_name_scope(&**scope).upcast()
        },
    )));
    field.apply_template();
}
fn selection_client(field: &TextBox) -> Rc<dyn TextInputMethodClient> {
    let args = TextInputMethodClientRequestedEventArgs::new();
    args.set_routed_event(Some(
        InputElement::text_input_method_client_requested_event(),
    ));
    field.raise_event(&args);
    args.client()
        .expect("editable TextBox provides a native input client")
}
fn selection_field() -> Ref<TextBox> {
    let field = TextBox::new();
    field.set_text(Some("a😀bcdef"));
    selection_template(&field);
    Dispatcher::current_dispatcher().run_jobs(None);
    field
}

#[test]
fn framework_native_caret_and_range_operation_batches_a_complete_client_event() {
    headless::run(|| {
        for (start, end, caret) in [(1, 6, 6), (6, 1, 1), (3, 3, 3)] {
            let field = selection_field();
            let client = selection_client(&field);
            let observations = Rc::new(RefCell::new(Vec::new()));
            let recorded = observations.clone();
            let weak = field.downgrade();
            let subscription = client.selection_changed(Rc::new(move || {
                let field = weak.upgrade().unwrap();
                recorded.borrow_mut().push((
                    field.selection_start(),
                    field.selection_end(),
                    field.caret_index(),
                ));
            }));
            field.set_selection_and_caret(TextSelection::new(start, end), caret);
            assert_eq!(*observations.borrow(), [(start, end, caret)]);
            field.set_selection_and_caret(TextSelection::new(start, end), caret);
            assert_eq!(observations.borrow().len(), 1);
            subscription.dispose();
        }
    });
}

#[test]
fn framework_selection_signal_is_available_before_template_and_in_readonly_fields() {
    headless::run(|| {
        for readonly in [false, true] {
            let field = TextBox::new();
            field.set_text(Some("a😀bcdef"));
            field.set_is_read_only(readonly);
            let observations = Rc::new(RefCell::new(Vec::new()));
            let recorded = observations.clone();
            let weak = field.downgrade();
            let token = field.selection_changed(move || {
                let field = weak.upgrade().unwrap();
                recorded
                    .borrow_mut()
                    .push((field.selection_start(), field.selection_end()));
            });
            for (start, end) in [(1, 6), (6, 1), (3, 3)] {
                observations.borrow_mut().clear();
                field.set_selection_and_caret(TextSelection::new(start, end), end);
                assert_eq!(*observations.borrow(), [(start, end)]);
                field.set_selection_and_caret(TextSelection::new(start, end), end);
                assert_eq!(observations.borrow().len(), 1);
            }
            observations.borrow_mut().clear();
            field.set_selection_start(0);
            field.set_selection_end(8);
            assert_eq!(*observations.borrow(), [(0, 3), (0, 8)]);
            observations.borrow_mut().clear();
            field.set_text(Some("xy"));
            // Coercing the previous caret 3 to 2 collapses the native selection.
            assert_eq!(*observations.borrow(), [(2, 2)]);
            assert_eq!(field.is_read_only(), readonly);
            token.dispose();
            observations.borrow_mut().clear();
            field.set_selection_and_caret(TextSelection::new(1, 1), 1);
            assert!(observations.borrow().is_empty());
            // Deliver the native TextChanged work queued by set_text before
            // checking that the disposed selection subscription retains no field.
            Dispatcher::current_dispatcher().run_jobs(None);
            let weak = field.downgrade();
            drop(field);
            assert!(weak.upgrade().is_none());
        }
    });
}

#[test]
fn keyboard_selection_write_notifies_other_adapters_only_after_the_range_is_complete() {
    headless::run(|| {
        for (start, length) in [(1, 5), (0, 0), (3, 2), (8, 0)] {
            let field = selection_field();
            field.set_caret_index(3);
            Dispatcher::current_dispatcher().run_jobs(None);
            let writer = TextBoxInputTarget::new(field.clone(), None).unwrap();
            let observer = TextBoxInputTarget::new(field.clone(), None).unwrap();
            let observations = Rc::new(RefCell::new(Vec::new()));
            let recorded = observations.clone();
            observer.changed().add(Rc::new(move |target| {
                recorded
                    .borrow_mut()
                    .push((target.selection_start(), target.selection_length()));
            }));
            let own_changes = Rc::new(Cell::new(0));
            let recorded = own_changes.clone();
            writer
                .changed()
                .add(Rc::new(move |_| recorded.set(recorded.get() + 1)));
            writer.select(start, length);
            assert_eq!(*observations.borrow(), [(start, length)]);
            assert_eq!(field.caret_index(), start + length);
            assert_eq!(own_changes.get(), 0);
            observer.detach();
            writer.detach();
        }
    });
}

#[test]
fn keyboard_selection_collapses_when_the_caret_does_not_move() {
    headless::run(|| {
        for rendered in [false, true] {
            let field = TextBox::new();
            field.set_text(Some("a😀bcdef"));
            if rendered {
                selection_template(&field);
            }
            let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
            target.select(1, 5);
            assert_eq!(field.caret_index(), 6);
            target.select(6, 0);
            assert_eq!(
                (target.selection_start(), target.selection_length()),
                (6, 0)
            );
            assert_eq!((field.selection_start(), field.selection_end()), (6, 6));
            target.detach();
        }
    });
}

#[test]
fn reentrant_observer_edits_the_complete_keyboard_selection() {
    headless::run(|| {
        let field = selection_field();
        let writer = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let observer = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let weak = field.downgrade();
        let once = Cell::new(true);
        observer.changed().add(Rc::new(move |_| {
            if once.replace(false) {
                weak.upgrade().unwrap().set_selected_text(Some("!"));
            }
        }));
        writer.select(1, 5);
        assert_eq!(field.text().as_deref(), Some("a!ef"));
        observer.detach();
        writer.detach();
    });
}

#[test]
fn selection_write_is_complete_without_an_active_native_text_view() {
    headless::run(|| {
        for readonly in [false, true] {
            for rendered in [false, true] {
                let field = TextBox::new();
                field.set_text(Some("a😀bcdef"));
                if rendered {
                    selection_template(&field);
                }
                field.set_is_read_only(readonly);
                Dispatcher::current_dispatcher().run_jobs(None);
                let writer = TextBoxInputTarget::new(field.clone(), None).unwrap();
                let observer = TextBoxInputTarget::new(field.clone(), None).unwrap();
                let observations = Rc::new(RefCell::new(Vec::new()));
                let recorded = observations.clone();
                observer.changed().add(Rc::new(move |target| {
                    recorded
                        .borrow_mut()
                        .push((target.selection_start(), target.selection_length()));
                }));
                writer.select(1, 5);
                assert_eq!(
                    *observations.borrow(),
                    [(1, 5)],
                    "rendered={rendered}, readonly={readonly}"
                );
                observer.detach();
                writer.detach();
            }
        }
    });
}

#[test]
fn inactive_selection_observer_edits_the_completed_range() {
    headless::run(|| {
        let field = selection_field();
        let other = TextBox::new();
        let panel = StackPanel::new();
        panel.children().add(field.clone());
        panel.children().add(other.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        assert!(field.focus());
        let client = selection_client(&field);
        let writer = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let observer = TextBoxInputTarget::new(field.clone(), None).unwrap();
        assert!(other.focus());
        assert!(client.text_view_visual().is_none());
        let once = Cell::new(true);
        let weak = field.downgrade();
        observer.changed().add(Rc::new(move |_| {
            if once.replace(false) {
                weak.upgrade().unwrap().set_selected_text(Some("!"));
            }
        }));
        writer.select(1, 5);
        assert_eq!(field.text().as_deref(), Some("a!ef"));
        observer.detach();
        writer.detach();
        window.close();
    });
}

#[test]
fn native_batched_selection_changes_notify_only_the_complete_range() {
    headless::run(|| {
        for (start, end) in [(1, 6), (6, 1), (3, 3)] {
            let field = selection_field();
            let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
            let observations = Rc::new(RefCell::new(Vec::new()));
            let recorded = observations.clone();
            target.changed().add(Rc::new(move |target| {
                recorded
                    .borrow_mut()
                    .push((target.selection_start(), target.selection_length()))
            }));
            let client = selection_client(&field);
            assert!(client.text_view_visual().is_some());
            client.set_selection(TextSelection::new(start, end));
            assert_eq!(
                *observations.borrow(),
                [(start.min(end), (end - start).abs())]
            );
            target.detach();
        }
    });
}

#[test]
fn native_batched_selection_reentry_replaces_the_final_range() {
    headless::run(|| {
        for (start, end) in [(1, 6), (6, 1)] {
            let field = selection_field();
            let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
            let weak = field.downgrade();
            let once = Cell::new(true);
            target.changed().add(Rc::new(move |_| {
                if once.replace(false) {
                    weak.upgrade().unwrap().set_selected_text(Some("!"));
                }
            }));
            selection_client(&field).set_selection(TextSelection::new(start, end));
            assert_eq!(field.text().as_deref(), Some("a!ef"));
            assert_eq!(
                (target.selection_start(), target.selection_length()),
                (2, 0)
            );
            target.detach();
        }
    });
}

#[test]
fn native_batched_selection_activates_after_late_template_application() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_text(Some("a😀bcdef"));
        Dispatcher::current_dispatcher().run_jobs(None);
        let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let observations = Rc::new(RefCell::new(Vec::new()));
        let recorded = observations.clone();
        target.changed().add(Rc::new(move |target| {
            recorded
                .borrow_mut()
                .push((target.selection_start(), target.selection_length()))
        }));
        field.set_caret_index(3);
        assert!(
            !observations.borrow().is_empty(),
            "untemplated fields retain external-change notifications"
        );
        selection_template(&field);
        observations.borrow_mut().clear();
        selection_client(&field).set_selection(TextSelection::new(1, 6));
        assert_eq!(*observations.borrow(), [(1, 5)]);
        target.detach();
    });
}

#[test]
fn native_batched_selection_readonly_transition_and_disposal_do_not_retain_targets() {
    headless::run(|| {
        let field = selection_field();
        field.set_is_read_only(true);
        let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let observations = Rc::new(RefCell::new(Vec::new()));
        let recorded = observations.clone();
        target.changed().add(Rc::new(move |target| {
            recorded
                .borrow_mut()
                .push((target.selection_start(), target.selection_length()))
        }));
        field.set_caret_index(3);
        assert!(!observations.borrow().is_empty());
        field.set_is_read_only(false);
        let client = selection_client(&field);
        observations.borrow_mut().clear();
        client.set_selection(TextSelection::new(1, 6));
        assert_eq!(*observations.borrow(), [(1, 5)]);
        target.detach();
        field.set_is_read_only(true);
        field.set_is_read_only(false);
        observations.borrow_mut().clear();
        client.set_selection(TextSelection::new(6, 1));
        assert!(observations.borrow().is_empty());
        let weak_target = Rc::downgrade(&target);
        drop(target);
        assert!(weak_target.upgrade().is_none());
        let weak_field = field.downgrade();
        drop(field);
        assert!(
            weak_field.upgrade().is_none(),
            "retained native client must not retain its field"
        );
    });
}

#[test]
fn native_batched_selection_keeps_keyboard_edits_suppressed() {
    headless::run(|| {
        let field = selection_field();
        let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let notifications = Rc::new(Cell::new(0));
        let count = notifications.clone();
        target
            .changed()
            .add(Rc::new(move |_| count.set(count.get() + 1)));
        target.select(1, 5);
        assert_eq!(
            (
                field.caret_index(),
                target.selection_start(),
                target.selection_length()
            ),
            (6, 1, 5)
        );
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(notifications.get(), 0);
        target.try_replace(1, 5, &"!".into()).unwrap();
        assert_eq!(target.text(), Utf16Text::from("a!ef"));
        assert_eq!(field.caret_index(), 2);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(notifications.get(), 0);
        selection_client(&field).set_selection(TextSelection::new(0, 2));
        assert_eq!(notifications.get(), 1);
        target.detach();
    });
}

#[test]
fn native_batched_selection_coercion_finishes_before_external_text_notification() {
    headless::run(|| {
        let field = selection_field();
        selection_client(&field).set_selection(TextSelection::new(6, 7));
        let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let observations = Rc::new(RefCell::new(Vec::new()));
        let recorded = observations.clone();
        target.changed().add(Rc::new(move |target| {
            recorded.borrow_mut().push((
                target.text(),
                target.selection_start(),
                target.selection_length(),
            ))
        }));
        field.set_text(Some("xy"));
        assert_eq!(*observations.borrow(), [(Utf16Text::from("xy"), 2, 0)]);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(
            *observations.borrow(),
            [(Utf16Text::from("xy"), 2, 0), (Utf16Text::from("xy"), 2, 0)]
        );
        target.detach();
    });
}

#[test]
fn native_batched_selection_resumes_after_focus_reactivation() {
    headless::run(|| {
        let field = selection_field();
        let other = TextBox::new();
        let panel = StackPanel::new();
        panel.children().add(field.clone());
        panel.children().add(other.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(field.focus());
        let client = selection_client(&field);
        let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let observations = Rc::new(RefCell::new(Vec::new()));
        let recorded = observations.clone();
        target.changed().add(Rc::new(move |target| {
            recorded
                .borrow_mut()
                .push((target.selection_start(), target.selection_length()))
        }));
        assert!(other.focus());
        assert!(client.text_view_visual().is_none());
        observations.borrow_mut().clear();
        field.set_caret_index(3);
        assert!(
            !observations.borrow().is_empty(),
            "inactive fields retain complete native selection notifications"
        );
        assert!(field.focus());
        assert!(client.text_view_visual().is_some());
        observations.borrow_mut().clear();
        client.set_selection(TextSelection::new(1, 6));
        assert_eq!(*observations.borrow(), [(1, 5)]);
        target.detach();
        window.close();
    });
}

#[test]
fn native_selection_subscription_does_not_request_or_enable_an_input_client() {
    headless::run(|| {
        let field = selection_field();
        field.set_is_read_only(true);
        let requests = Rc::new(Cell::new(0));
        let count = requests.clone();
        let request_token = field.add_handler(
            InputElement::text_input_method_client_requested_event(),
            move |_, _| {
                count.set(count.get() + 1);
            },
        );
        let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
        let observations = Rc::new(RefCell::new(Vec::new()));
        let recorded = observations.clone();
        target.changed().add(Rc::new(move |target| {
            recorded
                .borrow_mut()
                .push((target.selection_start(), target.selection_length()))
        }));
        field.set_selection_and_caret(TextSelection::new(1, 6), 6);
        assert_eq!(*observations.borrow(), [(1, 5)]);
        assert!(field.is_read_only());
        field.set_is_read_only(false);
        field.set_is_read_only(true);
        assert_eq!(requests.get(), 0);
        target.detach();
        observations.borrow_mut().clear();
        field.set_selection_and_caret(TextSelection::new(3, 3), 3);
        assert!(observations.borrow().is_empty());
        field.remove_handler(
            InputElement::text_input_method_client_requested_event(),
            request_token,
        );
    });
}
fn show(text_box: &Ref<TextBox>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(500.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(text_box.clone())));
    window.show();
    Dispatcher::current_dispatcher().run_jobs(None);
    window
}
#[test]
fn native_text_edit_suppression_external_changes_and_disposal() {
    headless::run(|| {
        let text_box = TextBox::new();
        let window = show(&text_box);
        let target = TextBoxInputTarget::new(text_box.clone(), None).unwrap();
        let first = target.as_input_target();
        let second = target.as_input_target();
        assert!(Rc::ptr_eq(&first, &second));
        let changed = Rc::new(Cell::new(0));
        let count = changed.clone();
        target
            .changed()
            .add(Rc::new(move |_| count.set(count.get() + 1)));
        target.try_replace(0, 0, &"hello 😀".into()).unwrap();
        assert_eq!(target.text(), Utf16Text::from("hello 😀"));
        assert_eq!(target.selection_start(), 8);
        assert_eq!(text_box.caret_index(), 8);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(changed.get(), 0);
        target.select(1, 3);
        assert_eq!(target.selection_start(), 1);
        assert_eq!(target.selection_length(), 3);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(changed.get(), 0);
        text_box.set_caret_index(2);
        assert!(changed.get() > 0);
        let before = changed.get();
        text_box.set_text(Some("external"));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(changed.get() > before);
        target.detach();
        let before = changed.get();
        text_box.set_caret_index(0);
        text_box.set_text(Some("after"));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(changed.get(), before);
        window.close();
    });
}
#[test]
fn native_engine_typing_read_only_max_length_disabled_ancestor_and_encoding_boundary() {
    headless::run(|| {
        let text_box = TextBox::new();
        let border = Border::new();
        border.set_child(text_box.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(border.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        let target = TextBoxInputTarget::new(text_box.clone(), None).unwrap();
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_glide_typing(false);
        settings.set_auto_correct(false);
        let engine = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        engine.attach(target.as_input_target()).unwrap();
        engine.type_text(&"hi".into()).unwrap();
        assert_eq!(text_box.text().as_deref(), Some("Hi"));
        text_box.set_is_read_only(true);
        engine.input_text(&"!".into()).unwrap();
        assert_eq!(target.text(), Utf16Text::from("Hi"));
        text_box.set_is_read_only(false);
        text_box.set_max_length(3);
        engine.input_text(&"long".into()).unwrap();
        assert_eq!(target.text(), Utf16Text::from("Hil"));
        text_box.set_max_length(0);
        border.set_is_enabled(false);
        engine.input_text(&"x".into()).unwrap();
        assert_eq!(target.text(), Utf16Text::from("Hil"));
        border.set_is_enabled(true);
        assert_eq!(
            target.try_replace(0, 0, &Utf16Text::from_units(vec![0xd800])),
            Err(NativeInputError::UnpairedSurrogate)
        );
        assert_eq!(target.text(), Utf16Text::from("Hil"));
        engine.dispose().unwrap();
        target.detach();
        Dispatcher::current_dispatcher().run_jobs(None);
        window.close();
    });
}
#[test]
fn native_attributes_outer_overrides_and_enable_depth() {
    headless::run(|| {
        let text_box = TextBox::new();
        let outer = Border::new();
        outer.set_child(text_box.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(outer.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        TextInputOptions::set_content_type(&text_box, TextInputContentType::Email);
        assert_eq!(
            TextBoxInputTarget::compute_attributes(&text_box)
                .unwrap()
                .kind(),
            InputKind::Email
        );
        ScreenKeyboardInput::set_input_kind(&outer, Some(Rc::new("Phone".to_owned())));
        let attributes = TextBoxInputTarget::compute_attributes(&text_box).unwrap();
        assert_eq!(attributes.kind(), InputKind::Phone);
        assert!(attributes.allow_suggestions());
        ScreenKeyboardInput::set_is_private(&outer, true);
        assert!(
            !TextBoxInputTarget::compute_attributes(&text_box)
                .unwrap()
                .is_private()
        );
        ScreenKeyboardInput::set_is_private(&text_box, true);
        ScreenKeyboardInput::set_suggestions_enabled(&text_box, false);
        let attributes = TextBoxInputTarget::compute_attributes(&text_box).unwrap();
        assert!(attributes.is_private());
        assert!(!attributes.allow_suggestions());
        assert!(!attributes.allow_auto_correct());
        ScreenKeyboardInput::set_is_enabled(&outer, false);
        assert!(!ScreenKeyboardInput::is_enabled_for(&text_box));
        ScreenKeyboardInput::set_is_enabled(&outer, true);
        assert!(ScreenKeyboardInput::is_enabled_for(&text_box));
        window.close();
    });
}
#[test]
fn native_enter_moves_focus_within_window_and_handler_can_handle() {
    headless::run(|| {
        let first = TextBox::new();
        let second = TextBox::new();
        let panel = StackPanel::new();
        panel.children().add(&first);
        panel.children().add(&second);
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(first.focus());
        let target = TextBoxInputTarget::new(first.clone(), None).unwrap();
        assert!(target.perform_enter_action(EnterAction::Next));
        assert!(second.is_focused());
        assert!(target.perform_enter_action(EnterAction::Previous));
        assert!(first.is_focused());
        target.detach();
        let target = TextBoxInputTarget::new(first.clone(), Some(Rc::new(|_| true))).unwrap();
        assert!(target.perform_enter_action(EnterAction::Next));
        assert!(first.is_focused());
        target.detach();
        window.close();
    });
}
#[test]
fn native_password_caret_history_attributes_external_change_and_numeric_kind() {
    headless::run(|| {
        let text_box = TextBox::new();
        assert!(matches!(
            PasswordBoxInputTarget::new(text_box.clone(), None),
            Err(NativeInputError::PasswordMaskRequired)
        ));
        text_box.set_password_char('●');
        TextInputOptions::set_content_type(&text_box, TextInputContentType::Pin);
        let window = show(&text_box);
        let target = PasswordBoxInputTarget::new(text_box.clone(), None).unwrap();
        assert_eq!(target.attributes().kind(), InputKind::NumericPassword);
        assert!(target.attributes().is_private());
        assert!(!target.attributes().allow_suggestions());
        let changes = Rc::new(Cell::new(0));
        let count = changes.clone();
        target
            .changed()
            .add(Rc::new(move |_| count.set(count.get() + 1)));
        target.try_replace(0, 0, &"1234".into()).unwrap();
        assert_eq!(changes.get(), 0);
        target.select(2, 100);
        assert_eq!(target.selection_start(), 2);
        assert_eq!(target.selection_length(), 0);
        target.try_replace(2, 0, &"x".into()).unwrap();
        assert_eq!(target.text(), Utf16Text::from("12x34"));
        text_box.set_text(Some("external"));
        assert!(changes.get() > 0);
        assert_eq!(target.selection_start(), 8);
        target.detach();
        window.close();
    });
}

#[test]
fn native_override_depth_and_enum_parser_boundaries_remain_explicit() {
    headless::run(|| {
        let text_box = TextBox::new();
        let mut child = text_box.clone().upcast::<Control>();
        let mut ancestors = Vec::new();
        for _ in 0..9 {
            let border = Border::new();
            border.set_child(child);
            child = border.clone().upcast();
            ancestors.push(border);
        }
        let window = Window::new();
        window.set_content(Some(Control::boxed(child)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        ScreenKeyboardInput::set_input_kind(&ancestors[5], Some(Rc::new("Email".to_owned())));
        assert_eq!(
            TextBoxInputTarget::compute_attributes(&text_box)
                .unwrap()
                .kind(),
            InputKind::Text
        );
        ScreenKeyboardInput::set_input_kind(&ancestors[4], Some(Rc::new("Phone".to_owned())));
        assert_eq!(
            TextBoxInputTarget::compute_attributes(&text_box)
                .unwrap()
                .kind(),
            InputKind::Phone
        );
        ScreenKeyboardInput::set_input_kind(&text_box, Some(Rc::new(" Email, Uri ".to_owned())));
        assert_eq!(
            TextBoxInputTarget::compute_attributes(&text_box)
                .unwrap()
                .kind(),
            InputKind::Password
        );
        ScreenKeyboardInput::set_input_kind(&text_box, Some(Rc::new("999".to_owned())));
        assert_eq!(
            TextBoxInputTarget::compute_attributes(&text_box)
                .unwrap()
                .kind(),
            InputKind(999)
        );
        ScreenKeyboardInput::set_input_kind(&text_box, Some(Rc::new(true)));
        assert_eq!(
            TextBoxInputTarget::compute_attributes(&text_box)
                .unwrap()
                .kind(),
            InputKind::Phone
        );
        // The same CLR Enum.TryParse path is measured by the unchanged theme reference corpus.
        for (text, kind) in [
            ("\u{a0}1", InputKind::Number),
            ("1\0", InputKind::Number),
            ("1\t\0", InputKind::Number),
            ("1\0\t", InputKind::Phone),
            ("1\u{a0}", InputKind::Phone),
            ("Text\u{a0}", InputKind::Text),
        ] {
            ScreenKeyboardInput::set_input_kind(&text_box, Some(Rc::new(text.to_owned())));
            assert_eq!(
                TextBoxInputTarget::compute_attributes(&text_box)
                    .unwrap()
                    .kind(),
                kind,
                "{text:?}"
            );
        }
        ScreenKeyboardInput::set_is_enabled(&ancestors[7], false);
        assert!(ScreenKeyboardInput::is_enabled_for(&text_box));
        ScreenKeyboardInput::set_is_enabled(&ancestors[6], false);
        assert!(!ScreenKeyboardInput::is_enabled_for(&text_box));
        window.close();
    });
}
#[test]
fn native_pending_edits_preserve_live_reentrancy_without_retaining_released_targets() {
    headless::run(|| {
        let text_box = TextBox::new();
        let window = show(&text_box);
        let target = TextBoxInputTarget::new(text_box.clone(), None).unwrap();
        let changed = Rc::new(Cell::new(0));
        let count = changed.clone();
        let weak = Rc::downgrade(&target);
        target.changed().add(Rc::new(move |_| {
            count.set(count.get() + 1);
            if count.get() == 1 {
                weak.upgrade()
                    .unwrap()
                    .try_replace(0, 0, &"!".into())
                    .unwrap();
            }
        }));
        target.try_replace(0, 0, &"internal".into()).unwrap();
        text_box.set_text(Some("external"));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(changed.get() > 0);
        assert!(target.text().to_utf8().unwrap().starts_with('!'));
        target.try_replace(0, 0, &"queued".into()).unwrap();
        target.detach();
        let weak = Rc::downgrade(&target);
        drop(target);
        assert!(weak.upgrade().is_none());
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(weak.upgrade().is_none());
        window.close();
    });
}
#[test]
fn masked_native_text_is_private_even_with_generic_adapter_and_attribute_snapshots() {
    headless::run(|| {
        let text_box = TextBox::new();
        text_box.set_password_char('●');
        let window = show(&text_box);
        let target = TextBoxInputTarget::new(text_box.clone(), None).unwrap();
        assert!(target.attributes().is_private());
        assert!(target.attributes().is_password());
        assert!(!target.attributes().supports_nlp());
        ScreenKeyboardInput::set_input_kind(&text_box, Some(Rc::new(InputKind::Email)));
        assert_eq!(target.attributes().kind(), InputKind::Password);
        assert_eq!(
            TextBoxInputTarget::compute_attributes(&text_box)
                .unwrap()
                .kind(),
            InputKind::Email
        );
        target.detach();
        window.close();
    });
}

#[test]
fn nullable_replacement_retains_source_errors_and_native_masked_edit_policy() {
    use serde_json::{Value, json};
    enum Adapter {
        Text(Rc<TextBoxInputTarget>),
        Password(Rc<PasswordBoxInputTarget>),
    }
    headless::run(|| {
        let mut matched = 0;
        let mut native_policy_cases = 0;
        for line in include_str!("../../../tests/fixtures/nullable-target.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let password = row["password"].as_bool().unwrap();
            let readonly = row["readOnly"].as_bool().unwrap();
            let enabled = row["enabled"].as_bool().unwrap();
            let max = row["max"].as_i64().unwrap() as i32;
            let start = row["start"].as_i64().unwrap() as i32;
            let length = row["length"].as_i64().unwrap() as i32;
            let field = TextBox::new();
            field.set_text(Some("abc"));
            field.set_selection_start(1);
            field.set_selection_end(2);
            field.set_max_length(max);
            field.set_is_read_only(readonly);
            field.set_is_enabled(enabled);
            if password {
                field.set_password_char('●')
            };
            Dispatcher::current_dispatcher().run_jobs(None);
            let owner = if password {
                Adapter::Password(PasswordBoxInputTarget::new(field.clone(), None).unwrap())
            } else {
                Adapter::Text(TextBoxInputTarget::new(field.clone(), None).unwrap())
            };
            let target: Rc<dyn ITextInputTarget> = match &owner {
                Adapter::Text(v) => v.as_input_target(),
                Adapter::Password(v) => v.clone(),
            };
            let events = Rc::new(Cell::new(0));
            let trace = events.clone();
            target
                .changed()
                .add(Rc::new(move |_| trace.set(trace.get() + 1)));
            let (error, param) = match target.replace(start, length, None) {
                Ok(()) => (None, None),
                Err(TargetReplaceError::NullReference) => (Some("NullReferenceException"), None),
                Err(TargetReplaceError::ArgumentNull(p)) => {
                    (Some("ArgumentNullException"), Some(p))
                }
                Err(e) => panic!("unmeasured target failure {e:?}"),
            };
            // WinUI PasswordBox has no read-only property. FerroUI's masked
            // TextBox deliberately follows its native read-only/enabled policy.
            // Keep the reference corpus unchanged and account for this migration.
            let mut expected = row.clone();
            if password && (readonly || !enabled) {
                expected["error"] = Value::Null;
                expected["param"] = Value::Null;
                native_policy_cases += 1;
            }
            assert_eq!(
                json!({"password":password,"readOnly":readonly,"enabled":enabled,"max":max,"start":start,"length":length,"error":error,"param":param,"text":target.text().to_utf8().unwrap(),"caret":target.selection_start(),"selection":target.selection_length(),"events":events.get()}),
                expected
            );
            match owner {
                Adapter::Text(v) => v.detach(),
                Adapter::Password(v) => v.detach(),
            };
            matched += 1;
        }
        assert_eq!(matched, 96);
        assert_eq!(native_policy_cases, 36);
    });
}
