//! Not from the reference: what the drag and drop behaviors do without a drag session of the
//! system.
//!
//! The drop targets get the drag events raised on the elements of a headless window (what the drag
//! and drop device of the framework raises over an element); the drag sources get the pointer
//! events of a press and a move past their threshold, which starts a drag (the drag of the system
//! itself does not run in the unit test application); the context store and the helpers of the drop
//! handlers are tested directly.

use crate::drag_drop_context_store::DragDropContextStore;
use crate::tests::support::{
    add_behavior, border, context_drag_data, files_drag_data, is_text, raise_drag_event, run_jobs, show_window,
    start_app, text, text_drag_data, Mouse, ObservableCommand, TestDragHandler, TestDropHandler,
};
use crate::values::{boxed_object, drag_event_args_of, object_of, StorageItemValue};
use crate::{
    AddPreviewFilesAction, ContentControlFilesDropBehavior, ContextDragBehavior, ContextDragWithDirectionBehavior,
    ContextDropBehavior, DragDropCommandsBehavior, DropHandlerBase, FilesDropBehavior, FilesPreviewBehavior,
    PanelDragBehavior, PanelDropBehavior, TextDropBehavior, TypedDragBehavior,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueType;
use ferroui_base::input::{DataTransfer, DragDrop, DragDropEffects};
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::{BoxedValue, Point, Ref};
use ferroui_controls::{ContentControl, Control, ItemsControl, ItemsSource, StackPanel};
use std::rc::Rc;

// --- the context store -------------------------------------------------------------------------

#[test]
fn the_context_store_finds_a_value_by_its_key_until_it_is_removed() {
    let value = text("context");

    let key = DragDropContextStore::add(Some(&value));

    assert!(key.is_some());
    let found = DragDropContextStore::get(key.as_deref()).expect("the value");
    assert!(Rc::ptr_eq(&value, &found));

    DragDropContextStore::remove(key.as_deref());
    assert!(DragDropContextStore::get(key.as_deref()).is_none());
}

#[test]
fn the_context_store_holds_its_values_weakly_and_has_no_key_for_no_value() {
    assert!(DragDropContextStore::add(None).is_none());
    assert!(DragDropContextStore::get(None).is_none());

    let value = text("context");
    let key = DragDropContextStore::add(Some(&value));
    drop(value);

    assert!(DragDropContextStore::get(key.as_deref()).is_none());
}

#[test]
fn the_keys_of_the_context_store_are_unique() {
    let value = text("context");

    let first = DragDropContextStore::add(Some(&value));
    let second = DragDropContextStore::add(Some(&value));

    assert_ne!(first, second);
}

// --- the drop handler base ---------------------------------------------------------------------

/// A drop handler with the members of the base class only.
struct BaseDropHandler;

impl DropHandlerBase for BaseDropHandler {}

#[test]
fn the_drop_handler_base_moves_and_swaps_items() {
    let handler = BaseDropHandler;
    let items = FerroList::from_items([1, 2, 3, 4]);

    handler.move_item(&items, 0, 2);
    assert_eq!(vec![2, 3, 1, 4], items.to_vec());

    handler.move_item(&items, 3, 0);
    assert_eq!(vec![4, 2, 3, 1], items.to_vec());

    handler.swap_item(&items, 0, 3);
    assert_eq!(vec![1, 2, 3, 4], items.to_vec());

    let others = FerroList::from_items([10, 20]);
    handler.move_item_between(&items, &others, 0, 1);
    assert_eq!(vec![2, 3, 4], items.to_vec());
    assert_eq!(vec![10, 1, 20], others.to_vec());

    handler.swap_item_between(&items, &others, 0, 0);
    assert_eq!(vec![10, 3, 4], items.to_vec());
    assert_eq!(vec![2, 1, 20], others.to_vec());

    handler.insert_item(&items, 5, 3);
    assert_eq!(vec![10, 3, 4, 5], items.to_vec());
}

#[test]
fn the_drop_handler_base_accepts_what_it_validates() {
    let _app = start_app();
    let target = border(50.0, 50.0);
    let _window = show_window(target.clone(), 100.0, 100.0);
    let sender = boxed_object(&target);

    // The base class validates nothing.
    let e = raise_drag_event(&target, DragDrop::drag_enter_event(), DataTransfer::new(), Point::new(5.0, 5.0));
    DropHandlerBase::enter(&BaseDropHandler, Some(&sender), &e, None, None);
    assert_eq!(DragDropEffects::NONE, e.drag_effects());
    assert!(e.handled());

    let e = raise_drag_event(&target, DragDrop::drag_over_event(), DataTransfer::new(), Point::new(5.0, 5.0));
    let handler = TestDropHandler::default();
    DropHandlerBase::over(&handler, Some(&sender), &e, None, None);
    assert_eq!(DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK, e.drag_effects());
    assert!(e.handled());

    // A drop executes the handler.
    let e = raise_drag_event(&target, DragDrop::drop_event(), DataTransfer::new(), Point::new(5.0, 5.0));
    DropHandlerBase::drop(&BaseDropHandler, Some(&sender), &e, None, None);
    assert_eq!(DragDropEffects::NONE, e.drag_effects());
}

// --- the drop targets --------------------------------------------------------------------------

#[test]
fn a_drop_target_allows_drop_while_it_is_in_the_visual_tree() {
    let _app = start_app();
    let panel = StackPanel::new();
    let target = border(50.0, 50.0);
    panel.children().add(target.clone());
    add_behavior(&target, &ContextDropBehavior::new());
    let window = show_window(panel, 100.0, 100.0);

    assert!(DragDrop::get_allow_drop(&target));

    window.set_content(None);
    run_jobs();

    assert!(!DragDrop::get_allow_drop(&target));
}

#[test]
fn the_context_drop_behavior_hands_the_contexts_of_the_drag_to_its_handler() {
    let _app = start_app();
    let target = border(50.0, 50.0);
    let handler = Rc::new(TestDropHandler::default());
    let behavior = ContextDropBehavior::new();
    behavior.set_handler(Some(handler.clone()));
    behavior.set_context(Some(text("TargetContext")));
    add_behavior(&target, &behavior);
    let _window = show_window(target.clone(), 100.0, 100.0);

    let source_context = text("SourceContext");
    let (data, _remove) = context_drag_data(&source_context);

    let e = raise_drag_event(&target, DragDrop::drag_enter_event(), data.clone(), Point::new(5.0, 5.0));
    assert!(e.drag_effects().contains(DragDropEffects::MOVE));
    assert!(e.handled());

    raise_drag_event(&target, DragDrop::drop_event(), data, Point::new(5.0, 5.0));

    let last_source_context = handler.last_source_context.borrow().clone().expect("the source context");
    assert!(Rc::ptr_eq(&source_context, &last_source_context));
    assert!(is_text(handler.last_target_context.borrow().as_ref(), "TargetContext"));
}

#[test]
fn the_context_drop_behavior_targets_the_data_context_without_a_context() {
    let _app = start_app();
    let target = border(50.0, 50.0);
    target.set_data_context(Some(text("DataContext")));
    let handler = Rc::new(TestDropHandler::default());
    let behavior = ContextDropBehavior::new();
    behavior.set_handler(Some(handler.clone()));
    add_behavior(&target, &behavior);
    let _window = show_window(target.clone(), 100.0, 100.0);

    raise_drag_event(&target, DragDrop::drop_event(), DataTransfer::new(), Point::new(5.0, 5.0));

    assert!(handler.last_source_context.borrow().is_none());
    assert!(is_text(handler.last_target_context.borrow().as_ref(), "DataContext"));
}

#[test]
fn the_panel_drop_behavior_moves_the_dragged_control_to_its_panel() {
    let _app = start_app();
    let root = StackPanel::new();
    let left_panel = StackPanel::new();
    let right_panel = StackPanel::new();
    let rectangle = border(20.0, 20.0);
    let drag_behavior = PanelDragBehavior::new();
    add_behavior(&rectangle, &drag_behavior);
    left_panel.children().add(rectangle.clone());
    add_behavior(&left_panel, &PanelDropBehavior::new());
    add_behavior(&right_panel, &PanelDropBehavior::new());
    root.children().add(left_panel.clone());
    root.children().add(right_panel.clone());
    let _window = show_window(root, 200.0, 200.0);

    // The drag behavior drags its control.
    let context = drag_behavior.context().expect("the context of the drag behavior");
    assert!(object_of(Some(&context)).is_some_and(|object| object.ptr_eq(&rectangle)));
    let (data, _remove) = context_drag_data(&context);

    let e = raise_drag_event(&right_panel, DragDrop::drag_over_event(), data.clone(), Point::new(5.0, 5.0));
    assert!(e.drag_effects().contains(DragDropEffects::MOVE));

    raise_drag_event(&right_panel, DragDrop::drop_event(), data, Point::new(5.0, 5.0));

    let rectangle_control: Ref<Control> = rectangle.clone().upcast();
    assert!(!left_panel.children().contains(&rectangle_control));
    assert!(right_panel.children().contains(&rectangle_control));
}

#[test]
fn the_text_drop_behavior_executes_its_command_with_the_text() {
    let _app = start_app();
    let target = border(50.0, 50.0);
    let command = ObservableCommand::new();
    let behavior = TextDropBehavior::new();
    behavior.set_command(command.as_command());
    add_behavior(&target, &behavior);
    let _window = show_window(target.clone(), 100.0, 100.0);

    // Without text the drop is refused.
    let e = raise_drag_event(&target, DragDrop::drag_enter_event(), DataTransfer::new(), Point::new(5.0, 5.0));
    assert_eq!(DragDropEffects::NONE, e.drag_effects());

    raise_drag_event(&target, DragDrop::drop_event(), text_drag_data("dropped"), Point::new(5.0, 5.0));

    assert_eq!(1, command.execute_count());
    assert!(is_text(command.last_execute_parameter().as_ref(), "dropped"));
}

#[test]
fn the_files_drop_behavior_executes_its_command_with_the_files() {
    let _app = start_app();
    let target = border(50.0, 50.0);
    let command = ObservableCommand::new();
    let behavior = FilesDropBehavior::new();
    behavior.set_command(command.as_command());
    add_behavior(&target, &behavior);
    let _window = show_window(target.clone(), 100.0, 100.0);

    raise_drag_event(&target, DragDrop::drop_event(), files_drag_data(&["a.txt", "b.txt"]), Point::new(5.0, 5.0));

    let parameter = command.last_execute_parameter().expect("the files");
    let files = parameter.downcast_ref::<FerroList<Rc<dyn IStorageItem>>>().expect("a list of files");
    let names: Vec<String> = files.to_vec().iter().map(|file| file.name()).collect();
    assert_eq!(vec!["a.txt", "b.txt"], names);
}

#[test]
fn a_disabled_drop_behavior_does_not_execute_its_command() {
    let _app = start_app();
    let target = border(50.0, 50.0);
    let command = ObservableCommand::new();
    let behavior = TextDropBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_is_enabled(false);
    add_behavior(&target, &behavior);
    let _window = show_window(target.clone(), 100.0, 100.0);

    raise_drag_event(&target, DragDrop::drop_event(), text_drag_data("dropped"), Point::new(5.0, 5.0));

    assert_eq!(0, command.execute_count());
}

#[test]
fn the_drag_drop_commands_behavior_executes_the_command_of_each_drag_event() {
    let _app = start_app();
    let target = border(50.0, 50.0);
    let drag_enter_command = ObservableCommand::new();
    let drop_command = ObservableCommand::new();
    let behavior = DragDropCommandsBehavior::new();
    behavior.set_drag_enter_command(drag_enter_command.as_command());
    behavior.set_drop_command(drop_command.as_command());
    add_behavior(&target, &behavior);
    let _window = show_window(target.clone(), 100.0, 100.0);

    raise_drag_event(&target, DragDrop::drag_enter_event(), DataTransfer::new(), Point::new(5.0, 5.0));
    assert_eq!(1, drag_enter_command.execute_count());
    assert_eq!(0, drop_command.execute_count());

    let e = raise_drag_event(&target, DragDrop::drop_event(), DataTransfer::new(), Point::new(5.0, 5.0));
    assert_eq!(1, drop_command.execute_count());
    // The event args are passed to the command.
    let parameter = drag_event_args_of(drop_command.last_execute_parameter().as_ref()).expect("the drag event args");
    assert!(parameter.data_transfer().formats().is_empty());
    assert!(e.routed_event() == parameter.routed_event());

    behavior.set_pass_event_args_to_command(false);
    raise_drag_event(&target, DragDrop::drop_event(), DataTransfer::new(), Point::new(5.0, 5.0));
    assert_eq!(2, drop_command.execute_count());
    assert!(drop_command.last_execute_parameter().is_none());
}

#[test]
fn the_drag_drop_commands_behavior_listens_to_its_target_control() {
    let _app = start_app();
    let panel = StackPanel::new();
    let host = border(50.0, 50.0);
    let other = border(50.0, 50.0);
    panel.children().add(host.clone());
    panel.children().add(other.clone());
    let drop_command = ObservableCommand::new();
    let behavior = DragDropCommandsBehavior::new();
    behavior.set_drop_command(drop_command.as_command());
    add_behavior(&host, &behavior);
    let _window = show_window(panel, 100.0, 200.0);
    assert!(!DragDrop::get_allow_drop(&other));

    // As in the reference, the events of a new target control are attached when it is set (and
    // the ones of the associated control stay attached).
    behavior.set_target_control(Some(other.clone().upcast()));

    assert!(DragDrop::get_allow_drop(&other));
    raise_drag_event(&other, DragDrop::drop_event(), DataTransfer::new(), Point::new(5.0, 5.0));
    assert_eq!(1, drop_command.execute_count());

    // The old target control is detached when another one is set.
    behavior.set_target_control(None);
    let third = border(50.0, 50.0);
    behavior.set_target_control(Some(third.clone().upcast()));
    raise_drag_event(&third, DragDrop::drop_event(), DataTransfer::new(), Point::new(5.0, 5.0));
    assert_eq!(2, drop_command.execute_count());
}

#[test]
fn the_files_preview_behavior_previews_the_files_while_the_drag_is_over() {
    let _app = start_app();
    let target = border(50.0, 50.0);
    let behavior = FilesPreviewBehavior::new();
    add_behavior(&target, &behavior);
    let _window = show_window(target.clone(), 100.0, 100.0);
    let data = files_drag_data(&["a.txt", "b.txt"]);

    raise_drag_event(&target, DragDrop::drag_enter_event(), data.clone(), Point::new(5.0, 5.0));
    assert_eq!(2, behavior.preview_files().count());

    raise_drag_event(&target, DragDrop::drag_leave_event(), data.clone(), Point::new(5.0, 5.0));
    assert_eq!(0, behavior.preview_files().count());

    raise_drag_event(&target, DragDrop::drag_over_event(), data.clone(), Point::new(5.0, 5.0));
    assert_eq!(2, behavior.preview_files().count());

    raise_drag_event(&target, DragDrop::drop_event(), data, Point::new(5.0, 5.0));
    assert_eq!(0, behavior.preview_files().count());
}

#[test]
fn the_add_preview_files_action_fills_the_items_of_its_items_control() {
    let _app = start_app();
    let items_control = ItemsControl::new();
    let items = FerroList::<StorageItemValue>::new();
    items_control.set_items_source(Some(ItemsSource::new(Rc::new(items.clone()))));
    let target = border(50.0, 50.0);
    let _window = show_window(target.clone(), 100.0, 100.0);
    let action = AddPreviewFilesAction::new();
    action.set_items_control(Some(items_control.clone()));

    let e = raise_drag_event(&target, DragDrop::drop_event(), files_drag_data(&["a.txt"]), Point::new(5.0, 5.0));
    let parameter = crate::values::boxed_event_args(&e);
    let result = action.execute(Some(&boxed_object(&target)), Some(&parameter));

    assert!(result.is_some_and(|result| result.downcast_ref::<bool>() == Some(&true)));
    assert_eq!(vec!["a.txt".to_string()], items.to_vec().iter().map(|file| file.name()).collect::<Vec<_>>());

    // Without a drag event there is nothing to add.
    let result = action.execute(None, Some(&text("not drag event args")));
    assert!(result.is_some_and(|result| result.downcast_ref::<bool>() == Some(&false)));
}

#[test]
fn the_content_control_files_drop_behavior_shows_its_content_during_the_drag() {
    let _app = start_app();
    let content_control = ContentControl::new();
    let behavior = ContentControlFilesDropBehavior::new();
    behavior.set_content_during_drag(Some(text("Drop the files here")));
    add_behavior(&content_control, &behavior);
    let _window = show_window(content_control.clone(), 100.0, 100.0);
    let data = files_drag_data(&["a.txt"]);

    // Without files the drag shows nothing.
    raise_drag_event(&content_control, DragDrop::drag_enter_event(), DataTransfer::new(), Point::new(5.0, 5.0));
    assert!(content_control.content().is_none());

    raise_drag_event(&content_control, DragDrop::drag_enter_event(), data.clone(), Point::new(5.0, 5.0));
    assert!(is_text(content_control.content().as_ref(), "Drop the files here"));

    // The drop clears the values the drag set.
    raise_drag_event(&content_control, DragDrop::drop_event(), data, Point::new(5.0, 5.0));
    assert!(content_control.content().is_none());
}

// --- the drag sources --------------------------------------------------------------------------

#[test]
fn the_context_drag_behavior_starts_a_drag_of_the_data_context_past_its_threshold() {
    let _app = start_app();
    let source = border(100.0, 100.0);
    let data_context: BoxedValue = text("Item");
    source.set_data_context(Some(data_context.clone()));
    let handler = Rc::new(TestDragHandler::default());
    let behavior = ContextDragBehavior::new();
    behavior.set_handler(Some(handler.clone()));
    add_behavior(&source, &behavior);
    let _window = show_window(source.clone(), 200.0, 200.0);

    let mouse = Mouse::new();
    mouse.down(&source, Point::new(5.0, 5.0));

    // Within the threshold nothing starts.
    mouse.move_to(&source, Point::new(7.0, 7.0));
    assert!(!handler.before_called.get());

    mouse.move_to(&source, Point::new(20.0, 20.0));
    run_jobs();

    assert!(handler.before_called.get());
    let context = handler.last_context.borrow().clone().expect("the context");
    assert!(Rc::ptr_eq(&data_context, &context));
}

#[test]
fn the_context_drag_behavior_does_not_start_after_a_release() {
    let _app = start_app();
    let source = border(100.0, 100.0);
    let handler = Rc::new(TestDragHandler::default());
    let behavior = ContextDragBehavior::new();
    behavior.set_handler(Some(handler.clone()));
    add_behavior(&source, &behavior);
    let _window = show_window(source.clone(), 200.0, 200.0);

    let mouse = Mouse::new();
    mouse.down(&source, Point::new(5.0, 5.0));
    mouse.up(&source, Point::new(5.0, 5.0));
    mouse.move_to(&source, Point::new(50.0, 50.0));
    run_jobs();

    assert!(!handler.before_called.get());
}

#[test]
fn the_context_drag_with_direction_behavior_starts_a_drag_past_its_threshold() {
    let _app = start_app();
    let source = border(100.0, 100.0);
    let handler = Rc::new(TestDragHandler::default());
    let behavior = ContextDragWithDirectionBehavior::new();
    behavior.set_handler(Some(handler.clone()));
    behavior.set_context(Some(text("Context")));
    add_behavior(&source, &behavior);
    let _window = show_window(source.clone(), 200.0, 200.0);

    let mouse = Mouse::new();
    mouse.down(&source, Point::new(50.0, 50.0));
    mouse.move_to(&source, Point::new(50.0, 20.0));
    run_jobs();

    assert!(handler.before_called.get());
    assert!(is_text(handler.last_context.borrow().as_ref(), "Context"));
}

#[test]
fn the_typed_drag_behavior_drags_only_data_of_its_type() {
    let _app = start_app();
    let source = border(100.0, 100.0);
    source.set_data_context(Some(text("Item")));
    let handler = Rc::new(TestDragHandler::default());
    let behavior = TypedDragBehavior::new();
    behavior.set_handler(Some(handler.clone()));
    behavior.set_data_type(Some(ValueType::of::<i32>()));
    add_behavior(&source, &behavior);
    let _window = show_window(source.clone(), 200.0, 200.0);

    let mouse = Mouse::new();
    mouse.down(&source, Point::new(5.0, 5.0));
    mouse.move_to(&source, Point::new(20.0, 20.0));
    run_jobs();
    assert!(!handler.before_called.get());
    mouse.up(&source, Point::new(20.0, 20.0));

    behavior.set_data_type(Some(ValueType::of::<String>()));
    mouse.down(&source, Point::new(5.0, 5.0));
    mouse.move_to(&source, Point::new(20.0, 20.0));
    run_jobs();

    assert!(handler.before_called.get());
    assert!(is_text(handler.last_context.borrow().as_ref(), "Item"));
}
