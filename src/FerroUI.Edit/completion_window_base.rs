use crate::TextEditor;
use ferroui_base::input::{
    IScrollable, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, AttachedProperty,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, VisualImpl, WeakRef,
};
use ferroui_controls::primitives::{ILogicalScrollable, Popup};
use ferroui_controls::{Control, ControlImpl, PlacementMode, TopLevel, WindowBase};
use ferroui_edit_core::document::{
    AnchorMovementType, DocumentChangeEventArgs, DocumentError, SimpleSegment, TextDocument,
};
use ferroui_edit_core::utils::DocumentSubscription;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Shared lifecycle for editor assistance popups. A weak editor reference and
/// explicitly disposed subscriptions prevent a closed popup from retaining its
/// editor or continuing to react to document input.
#[repr(C)]
pub struct CompletionWindowBase {
    base: Popup,
    editor: RefCell<Option<WeakRef<TextEditor>>>,
    document: RefCell<Option<TextDocument>>,
    start_offset: Cell<usize>,
    end_offset: Cell<usize>,
    expect_insertion_before_start: Cell<bool>,
    closed: Cell<bool>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    document_subscriptions: RefCell<Vec<DocumentSubscription>>,
}
ferro_class! {
    CompletionWindowBase: Popup, virtuals CompletionWindowBaseImpl: ControlImpl {
        fn on_editor_updated(this);
        fn close_on_focus_lost(this) -> bool;
        fn on_closed(this);
    }
}
ferroui_base::ferro_class_info!(CompletionWindowBase {
    new: CompletionWindowBase::new
});
ferro_impl_classes!(CompletionWindowBase:
    StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ControlImpl
);
impl FerroObjectImpl for CompletionWindowBase {}
impl InputElementImpl for CompletionWindowBase {
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        Self::parent_on_key_down(this, event);
        if !event.handled() && event.key == Key::Escape && !this.is_closed() {
            this.close_window();
            event.set_handled(true);
        }
    }
}
impl CompletionWindowBaseImpl for CompletionWindowBase {
    fn on_editor_updated(_this: &Self) {}
    fn close_on_focus_lost(_this: &Self) -> bool {
        true
    }
    fn on_closed(_this: &Self) {}
}
ferro_properties! { impl CompletionWindowBase {
    pub fn active_windows_property() -> AttachedProperty<Vec<Ref<CompletionWindowBase>>> {
        FerroProperty::register_attached::<CompletionWindowBase, TextEditor, _>("ActiveWindows", Vec::new())
    }
} }
impl CompletionWindowBase {
    pub fn construct() -> Self {
        Self {
            base: Popup::construct(),
            editor: RefCell::new(None),
            document: RefCell::new(None),
            start_offset: Cell::new(0),
            end_offset: Cell::new(0),
            expect_insertion_before_start: Cell::new(false),
            closed: Cell::new(true),
            subscriptions: RefCell::new(Vec::new()),
            document_subscriptions: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn editor(&self) -> Option<Ref<TextEditor>> {
        self.editor.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub fn text_area(&self) -> Option<Ref<crate::TextArea>> {
        self.editor().map(|editor| editor.text_area())
    }
    pub fn start_offset(&self) -> usize {
        self.start_offset.get()
    }
    pub fn end_offset(&self) -> usize {
        self.end_offset.get()
    }
    pub fn segment(&self) -> SimpleSegment {
        SimpleSegment::new(
            self.start_offset(),
            self.end_offset().saturating_sub(self.start_offset()),
        )
    }
    pub fn set_segment(&self, start: usize, end: usize) -> Result<(), DocumentError> {
        if let Some(document) = self.document.borrow().as_ref() {
            if start > end {
                return Err(DocumentError::InvalidRange {
                    offset: start,
                    length: 0,
                    text_length: document.text_length(),
                });
            }
            document.get_text(start, end - start)?;
        } else if start > end {
            return Err(DocumentError::InvalidRange {
                offset: start,
                length: 0,
                text_length: 0,
            });
        }
        self.start_offset.set(start);
        self.end_offset.set(end);
        Ok(())
    }
    pub fn set_start_offset(&self, offset: usize) -> Result<(), DocumentError> {
        self.set_segment(offset, self.end_offset())
    }
    pub fn set_end_offset(&self, offset: usize) -> Result<(), DocumentError> {
        self.set_segment(self.start_offset(), offset)
    }
    pub fn expect_insertion_before_start(&self) -> bool {
        self.expect_insertion_before_start.get()
    }
    pub fn set_expect_insertion_before_start(&self, value: bool) {
        self.expect_insertion_before_start.set(value);
    }
    pub fn is_closed(&self) -> bool {
        self.closed.get()
    }

    /// Attaches a new assistance session. Opening another window of the same
    /// concrete type closes the previous one. Different assistance types can
    /// coexist, and the most recently attached popup receives editor keys first.
    pub fn attach_to_editor(
        &self,
        editor: &Ref<TextEditor>,
        start: usize,
        end: usize,
    ) -> Result<(), DocumentError> {
        let document = editor.document();
        if start > end {
            return Err(DocumentError::InvalidRange {
                offset: start,
                length: 0,
                text_length: document.text_length(),
            });
        }
        document.get_text(start, end - start)?;
        self.dismiss();
        let previous = editor.get_value(Self::active_windows_property());
        for window in previous {
            if std::ptr::eq(window.get_type(), self.get_type()) {
                window.dismiss();
            }
        }
        *self.editor.borrow_mut() = Some(editor.downgrade());
        *self.document.borrow_mut() = Some(document.clone());
        self.start_offset.set(start);
        self.end_offset.set(end);
        self.closed.set(false);
        self.expect_insertion_before_start.set(false);
        let area = editor.text_area();
        self.set_placement_target(area.clone());
        self.set_popup_parent(Some(&editor.clone().upcast::<Control>()));
        self.set_placement(PlacementMode::BottomEdgeAlignedLeft);
        self.set_should_use_overlay_layer(true);
        self.set_is_light_dismiss_enabled(self.close_on_focus_lost());

        let weak = self.to_ref().downgrade();
        self.document_subscriptions
            .borrow_mut()
            .push(document.changing(move |change| {
                if let Some(window) = weak.upgrade() {
                    window.document_changing(change);
                }
            }));
        for event in [
            TextEditor::text_changed_event(),
            TextEditor::selection_changed_event(),
        ] {
            let weak = self.to_ref().downgrade();
            self.subscriptions
                .borrow_mut()
                .push(editor.add_disposable_handler(event, move |_, _| {
                    if let Some(window) = weak.upgrade() {
                        window.refresh_from_editor();
                    }
                }));
        }
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(editor.property_changed(move |change| {
                if change.property() == TextEditor::document_property().as_property() {
                    if let Some(window) = weak.upgrade() {
                        window.dismiss();
                    }
                }
            }));
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(area.scroll_invalidated(Rc::new(move || {
                if let Some(window) = weak.upgrade() {
                    window.update_position();
                }
            })));
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(
                area.add_disposable_handler(InputElement::lost_focus_event(), move |_, _| {
                    if let Some(window) = weak.upgrade() {
                        window.queue_focus_check();
                    }
                }),
            );
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(
                self.add_disposable_handler(InputElement::lost_focus_event(), move |_, _| {
                    if let Some(window) = weak.upgrade() {
                        window.queue_focus_check();
                    }
                }),
            );
        for event in [InputElement::key_down_event(), InputElement::key_up_event()] {
            let weak = self.to_ref().downgrade();
            self.subscriptions
                .borrow_mut()
                .push(area.add_disposable_handler_with(
                    event,
                    move |_, key: &KeyEventArgs| {
                        if let Some(window) = weak.upgrade() {
                            window.forward_key(key);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                ));
        }
        let weak = self.to_ref().downgrade();
        self.subscriptions.borrow_mut().push(self.closed(move || {
            if let Some(window) = weak.upgrade() {
                window.detach();
            }
        }));
        if let Some(top_level) = TopLevel::get_top_level(Some(&area)) {
            let weak = self.to_ref().downgrade();
            self.subscriptions
                .borrow_mut()
                .push(top_level.closed(move || {
                    if let Some(window) = weak.upgrade() {
                        window.dismiss();
                    }
                }));
            if let Some(parent) = top_level.cast::<WindowBase>() {
                let weak = self.to_ref().downgrade();
                self.subscriptions
                    .borrow_mut()
                    .push(parent.deactivated(move || {
                        if let Some(window) = weak.upgrade() {
                            window.dismiss();
                        }
                    }));
                let weak = self.to_ref().downgrade();
                self.subscriptions
                    .borrow_mut()
                    .push(parent.position_changed(move |_| {
                        if let Some(window) = weak.upgrade() {
                            window.update_position();
                        }
                    }));
            }
        }
        let mut windows = editor.get_value(Self::active_windows_property());
        windows.retain(|window| !window.is_closed());
        windows.push(self.to_ref());
        editor.set_value(Self::active_windows_property(), windows);
        self.refresh_from_editor();
        Ok(())
    }
    pub fn show(&self) {
        if !self.is_closed() && self.update_position() {
            self.open();
        }
    }
    pub(crate) fn track_subscription(&self, subscription: Rc<dyn IDisposable>) {
        if self.is_closed() {
            subscription.dispose();
        } else {
            self.subscriptions.borrow_mut().push(subscription);
        }
    }
    pub fn hide(&self) {
        self.close_window();
    }
    pub fn close_window(&self) {
        let editor = self.editor();
        self.dismiss();
        if let Some(editor) = editor {
            editor.text_area().focus();
        }
    }
    /// Dismisses without stealing focus from the control the user just selected.
    pub fn dismiss(&self) {
        self.close();
        self.detach();
    }
    fn detach(&self) {
        if self.closed.replace(true) {
            return;
        }
        let editor = self.editor();
        let subscriptions = self.subscriptions.take();
        let document_subscriptions = self.document_subscriptions.take();
        for subscription in subscriptions {
            subscription.dispose();
        }
        for subscription in document_subscriptions {
            subscription.dispose();
        }
        if let Some(editor) = editor {
            let mut windows = editor.get_value(Self::active_windows_property());
            windows.retain(|window| window != &self.to_ref() && !window.is_closed());
            if windows.is_empty() {
                editor.clear_value(Self::active_windows_property());
            } else {
                editor.set_value(Self::active_windows_property(), windows);
            }
        }
        self.on_closed();
        self.document.borrow_mut().take();
        self.editor.borrow_mut().take();
    }
    fn document_changing(&self, change: &DocumentChangeEventArgs) {
        if self.is_closed() {
            return;
        }
        let start = self.start_offset();
        if change.removal_length() > 0
            && change.offset.saturating_add(change.removal_length()) == start
        {
            self.dismiss();
            return;
        }
        let movement = if change.offset == start
            && change.removal_length() == 0
            && self.expect_insertion_before_start.replace(false)
        {
            AnchorMovementType::AfterInsertion
        } else {
            AnchorMovementType::BeforeInsertion
        };
        self.start_offset
            .set(change.get_new_offset(start, movement));
        self.end_offset
            .set(change.get_new_offset(self.end_offset(), AnchorMovementType::AfterInsertion));
    }
    pub fn refresh_from_editor(&self) {
        if self.is_closed() {
            return;
        }
        let Some(editor) = self.editor() else {
            self.dismiss();
            return;
        };
        if self
            .document
            .borrow()
            .as_ref()
            .is_none_or(|document| document != &editor.document())
        {
            self.dismiss();
            return;
        }
        if self.update_position() {
            self.on_editor_updated();
        }
    }
    /// Anchors at the tracked segment start, keeping placement independent of
    /// caret navigation. The framework popup positioner applies screen bounds.
    pub fn update_position(&self) -> bool {
        if self.is_closed() {
            return false;
        }
        let Some(area) = self.text_area() else {
            self.dismiss();
            return false;
        };
        let viewport = area.viewport();
        if viewport.width <= 0.0 || viewport.height <= 0.0 {
            self.set_placement_rect(Some(area.cursor_rectangle()));
            return true;
        }
        if let Some(rectangle) = area.rectangle_for_offset(self.start_offset()) {
            self.set_placement_rect(Some(rectangle));
            true
        } else {
            self.dismiss();
            false
        }
    }
    pub(crate) fn owns_editor_input(&self) -> bool {
        self.editor().is_some_and(|editor| {
            editor
                .get_value(Self::active_windows_property())
                .iter()
                .rev()
                .find(|window| !window.is_closed())
                .is_some_and(|window| window == &self.to_ref())
        })
    }
    fn forward_key(&self, key: &KeyEventArgs) {
        if key.handled()
            || self.is_closed()
            || !self.owns_editor_input()
            || key.key == Key::DeadCharProcessed
        {
            return;
        }
        // Routed event clones share routing state; create fresh args so the
        // editor's source/route is not changed while it is being delivered.
        let mut forwarded = KeyEventArgs::new();
        forwarded.key = key.key;
        forwarded.key_modifiers = key.key_modifiers;
        forwarded.physical_key = key.physical_key;
        forwarded.key_symbol = key.key_symbol.clone();
        forwarded.key_device_type = key.key_device_type;
        forwarded.set_routed_event(key.routed_event().as_ref());
        self.raise_event(&forwarded);
        if forwarded.handled() {
            key.set_handled(true);
        }
    }
    fn queue_focus_check(&self) {
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(window) = weak.upgrade() {
                    window.close_if_focus_lost();
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    pub fn close_if_focus_lost(&self) {
        if self.is_closed() || !self.close_on_focus_lost() {
            return;
        }
        let editor_focused = self
            .text_area()
            .is_some_and(|area| area.is_keyboard_focus_within());
        let popup_focused = self
            .child()
            .is_some_and(|child| child.is_keyboard_focus_within());
        if !editor_focused && !popup_focused {
            self.dismiss();
        }
    }
}
impl Drop for CompletionWindowBase {
    fn drop(&mut self) {
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        for subscription in self.document_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}
