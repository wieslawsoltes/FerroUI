use crate::{EditorAdornmentError, ITextViewConnect, TextArea, TextDocument, VisualLine};
use ferroui_base::{Rect, Ref, WeakRef};
use ferroui_controls::Control;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A formatted logical line in the current viewport. Bounds are in the
/// text area's coordinates and include its wrapped rows.
pub struct VisibleEditorLine {
    pub line: Rc<VisualLine>,
    pub bounds: Rect,
}

/// Optional editor callbacks for a native margin control.
pub trait ITextViewMargin: ITextViewConnect {
    /// Observes document attachment, replacement and detachment. Subscribe to
    /// the document itself when individual edits are needed.
    fn document_changed(&self, _previous: Option<&TextDocument>, _current: Option<&TextDocument>) {}

    /// Observes a change to scrolling, line layout or the size of the viewport.
    fn viewport_changed(&self, _view: &TextArea) {}
}

pub(crate) struct Margin {
    control: Ref<Control>,
    behavior: Option<Rc<dyn ITextViewMargin>>,
    view: RefCell<Option<WeakRef<TextArea>>>,
    host: RefCell<Option<WeakRef<TextArea>>>,
    explicit_view: Cell<bool>,
    changing: Cell<bool>,
    document: RefCell<Option<TextDocument>>,
}

struct MarginChange<'a>(&'a Cell<bool>);

impl Drop for MarginChange<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// A native control that follows a text view. The view can be assigned by the
/// application or supplied by a text area when it hosts the margin.
/// Clones share the association and hold the view weakly.
#[derive(Clone)]
pub struct TextViewMargin(Rc<Margin>);

/// Reuses a native control's view association without retaining the control
/// through the association it owns.
#[derive(Default)]
pub(crate) struct MarginBinding(RefCell<Weak<Margin>>);

impl MarginBinding {
    pub(crate) fn for_control(
        &self,
        control: Ref<Control>,
        behavior: impl FnOnce() -> Rc<dyn ITextViewMargin>,
    ) -> TextViewMargin {
        if let Some(binding) = self.0.borrow().upgrade() {
            return TextViewMargin(binding);
        }
        let margin = TextViewMargin::with_behavior(control, behavior());
        self.0.replace(Rc::downgrade(&margin.0));
        margin
    }

    pub(crate) fn text_view(&self) -> Option<Ref<TextArea>> {
        TextViewMargin(self.0.borrow().upgrade()?).text_view()
    }
}

impl TextViewMargin {
    pub fn new(control: Ref<Control>) -> Self {
        Self::create(control, None)
    }

    pub fn with_behavior(control: Ref<Control>, behavior: Rc<dyn ITextViewMargin>) -> Self {
        Self::create(control, Some(behavior))
    }

    fn create(control: Ref<Control>, behavior: Option<Rc<dyn ITextViewMargin>>) -> Self {
        Self(Rc::new(Margin {
            control,
            behavior,
            view: RefCell::new(None),
            host: RefCell::new(None),
            explicit_view: Cell::new(false),
            changing: Cell::new(false),
            document: RefCell::new(None),
        }))
    }

    pub fn control(&self) -> Ref<Control> {
        self.0.control.clone()
    }

    pub fn text_view(&self) -> Option<Ref<TextArea>> {
        self.0.view.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub fn document(&self) -> Option<TextDocument> {
        self.text_view()?.editor().map(|editor| editor.document())
    }

    /// Assigns the view independently of the control's visual parent. Clearing
    /// the assignment restores the host view while the margin is mounted.
    pub fn set_text_view(&self, view: Option<&TextArea>) -> Result<(), EditorAdornmentError> {
        let _change = self.0.begin_change()?;
        let host = self.0.host.borrow().as_ref().and_then(WeakRef::upgrade);
        if let (Some(host), Some(view)) = (&host, view) {
            if !host.ptr_eq(&view.to_ref()) {
                return Err(EditorAdornmentError::DifferentTextView);
            }
        }
        self.0.explicit_view.set(view.is_some());
        self.change_view(view.or(host.as_deref()));
        Ok(())
    }

    pub(crate) fn mount(
        &self,
        view: &TextArea,
        attach_control: impl FnOnce(),
    ) -> Result<(), EditorAdornmentError> {
        let _change = self.0.begin_change()?;
        let host = view.to_ref();
        if self
            .text_view()
            .is_some_and(|current| !current.ptr_eq(&host))
        {
            return Err(EditorAdornmentError::DifferentTextView);
        }
        self.0.host.replace(Some(host.downgrade()));
        attach_control();
        self.change_view(Some(view));
        Ok(())
    }

    pub(crate) fn unmount(
        &self,
        detach_control: impl FnOnce(),
    ) -> Result<(), EditorAdornmentError> {
        let _change = self.0.begin_change()?;
        detach_control();
        self.0.host.take();
        if !self.0.explicit_view.get() {
            self.change_view(None);
        }
        Ok(())
    }

    fn change_view(&self, next: Option<&TextArea>) {
        let previous = self.text_view();
        if match (&previous, next) {
            (Some(previous), Some(next)) => previous.ptr_eq(&next.to_ref()),
            (None, None) => true,
            _ => false,
        } {
            return;
        }
        self.0
            .view
            .replace(next.map(|view| view.to_ref().downgrade()));
        if next.is_none() {
            self.0.update_document(None);
        }
        if let Some(previous) = previous {
            previous.untrack_margin(Rc::as_ptr(&self.0));
            if let Some(behavior) = &self.0.behavior {
                behavior.remove_from_text_view(&previous);
            }
        }
        if let Some(next) = next {
            next.track_margin(Rc::downgrade(&self.0));
            if let Some(behavior) = &self.0.behavior {
                behavior.add_to_text_view(next);
            }
            self.0
                .update_document(next.editor().map(|editor| editor.document()));
        }
        self.0.control.invalidate_visual();
    }
}

impl Margin {
    fn begin_change(&self) -> Result<MarginChange<'_>, EditorAdornmentError> {
        if self.changing.replace(true) {
            Err(EditorAdornmentError::Busy)
        } else {
            Ok(MarginChange(&self.changing))
        }
    }

    fn follows(&self, view: &TextArea) -> bool {
        self.view
            .borrow()
            .as_ref()
            .is_some_and(|bound| bound.ptr_eq(&view.to_weak()))
    }

    fn update_document(&self, current: Option<TextDocument>) {
        if let Some(behavior) = &self.behavior {
            if *self.document.borrow() != current {
                let previous = self.document.replace(current.clone());
                behavior.document_changed(previous.as_ref(), current.as_ref());
            }
        }
    }

    pub(crate) fn view_document_changed(&self, view: &TextArea) {
        let Ok(_change) = self.begin_change() else {
            return;
        };
        if self.follows(view) {
            self.update_document(view.editor().map(|editor| editor.document()));
        }
    }

    pub(crate) fn update_viewport(&self, view: &TextArea) {
        let Ok(_change) = self.begin_change() else {
            return;
        };
        if !self.follows(view) {
            return;
        }
        self.update_document(view.editor().map(|editor| editor.document()));
        self.control.invalidate_visual();
        if let Some(behavior) = &self.behavior {
            behavior.viewport_changed(view);
        }
    }

    pub(crate) fn view_dropped(&self, view: &TextArea) {
        if !self.follows(view) {
            return;
        }
        let _change = self.begin_change();
        self.view.take();
        self.host.take();
        self.explicit_view.set(false);
        self.update_document(None);
        self.control.invalidate_visual();
        if let Some(behavior) = &self.behavior {
            behavior.remove_from_text_view(view);
        }
    }
}

impl Drop for Margin {
    fn drop(&mut self) {
        if let Some(view) = self.view.get_mut().take().and_then(|view| view.upgrade()) {
            view.untrack_margin(self);
            self.update_document(None);
            if let Some(behavior) = &self.behavior {
                behavior.remove_from_text_view(&view);
            }
        }
    }
}
