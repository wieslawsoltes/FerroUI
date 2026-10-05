use super::{IInlineHost, Inline, InlineUIContainer, Run};
use crate::Control;
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs, ResetBehavior};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{IntoRef, Ref, StyledElement, WeakRef};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// A collection of [`Inline`]s.
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity.
#[derive(Clone)]
pub struct InlineCollection(Rc<InlineCollectionData>);

struct InlineCollectionData {
    list: FerroList<Ref<Inline>>,
    /// The element whose logical children mirror the collection.
    logical_children: RefCell<Option<WeakRef<StyledElement>>>,
    inline_host: RefCell<Option<Rc<dyn IInlineHost>>>,
    invalidated: Rc<HandlerList<dyn Fn()>>,
}

impl PartialEq for InlineCollection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for InlineCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InlineCollection({})", self.0.list.count())
    }
}

impl Deref for InlineCollection {
    type Target = FerroList<Ref<Inline>>;

    fn deref(&self) -> &Self::Target {
        &self.0.list
    }
}

impl InlineCollection {
    /// Initializes a new instance of the `InlineCollection` class.
    pub fn new() -> Self {
        let list = FerroList::new();
        list.set_reset_behavior(ResetBehavior::Remove);

        let collection = Self(Rc::new(InlineCollectionData {
            list,
            logical_children: RefCell::new(None),
            inline_host: RefCell::new(None),
            invalidated: Rc::new(HandlerList::new()),
        }));

        let weak: Weak<InlineCollectionData> = Rc::downgrade(&collection.0);
        collection.0.list.add_collection_changed(Rc::new(
            move |e: &NotifyCollectionChangedEventArgs<'_, Ref<Inline>>| {
                let Some(this) = weak.upgrade().map(InlineCollection) else { return };

                match e.action {
                    NotifyCollectionChangedAction::Add => {
                        for inline in e.new_items {
                            this.on_add(inline);
                        }
                    }
                    NotifyCollectionChangedAction::Remove => {
                        for inline in e.old_items {
                            this.on_remove(inline);
                        }
                    }
                    NotifyCollectionChangedAction::Replace => {
                        for inline in e.old_items {
                            this.on_remove(inline);
                        }
                        for inline in e.new_items {
                            this.on_add(inline);
                        }
                    }
                    NotifyCollectionChangedAction::Move => {}
                    NotifyCollectionChangedAction::Reset => panic!("Resetting the collection is not supported."),
                }
            },
        ));

        collection
    }

    /// The element whose logical children mirror the collection.
    pub(crate) fn logical_children(&self) -> Option<Ref<StyledElement>> {
        self.0.logical_children.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_logical_children(&self, value: Option<&StyledElement>) {
        let old_value = self.logical_children();
        let new_value = value.map(StyledElement::to_ref);

        *self.0.logical_children.borrow_mut() = new_value.as_ref().map(Ref::downgrade);

        self.on_parent_changed(old_value.as_ref(), new_value.as_ref());
    }

    /// The host of the inlines of the collection.
    pub(crate) fn inline_host(&self) -> Option<Rc<dyn IInlineHost>> {
        self.0.inline_host.borrow().clone()
    }

    pub(crate) fn set_inline_host(&self, value: Option<Rc<dyn IInlineHost>>) {
        *self.0.inline_host.borrow_mut() = value.clone();

        self.on_inline_host_changed(value);
    }

    /// Gets the text contained in this collection: the text of every inline,
    /// or `None` when the collection is empty.
    pub fn text(&self) -> Option<String> {
        if self.0.list.count() == 0 {
            return None;
        }

        let mut builder = String::new();

        for inline in self.0.list.snapshot().iter() {
            inline.append_text(&mut builder);
        }

        Some(builder)
    }

    /// Adds an inline (of any class) to the collection.
    ///
    /// The text of a hosting text block becomes the first run of the
    /// collection.
    pub fn add(&self, inline: impl IntoRef<Inline>) {
        if let Some(text_block) = self.inline_host().and_then(|host| host.as_text_block()) {
            if let Some(text) = text_block.text().filter(|text| !text.is_empty()) {
                self.0.list.add(Run::with_text(Some(&text)).upcast());

                text_block.clear_text_internal();
            }
        }

        self.0.list.add(inline.into_ref());
    }

    /// Adds a text inline to this collection.
    pub fn add_text(&self, text: &str) {
        match self.inline_host().and_then(|host| host.as_text_block()) {
            Some(text_block) if !text_block.has_complex_content() => {
                let mut current = text_block.text().unwrap_or_default();
                current.push_str(text);
                text_block.set_text(Some(&current));
            }
            _ => self.add(Run::with_text(Some(text))),
        }
    }

    /// Adds a control wrapped inside an `InlineUIContainer` to this
    /// collection.
    pub fn add_control(&self, control: impl IntoRef<Control>) {
        self.add(InlineUIContainer::with_child(control));
    }

    /// Raised when an inline in the collection changes.
    pub fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.0.invalidated.add(Rc::new(handler));
        let handlers = Rc::downgrade(&self.0.invalidated);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }

    /// Invalidates this collection.
    fn invalidate(&self) {
        if let Some(host) = self.inline_host() {
            host.invalidate();
        }

        for (_, handler) in self.0.invalidated.snapshot().iter() {
            handler();
        }
    }

    fn on_parent_changed(&self, old_value: Option<&Ref<StyledElement>>, new_value: Option<&Ref<StyledElement>>) {
        if old_value != new_value {
            for child in self.0.list.snapshot().iter() {
                let child: Ref<StyledElement> = child.clone().upcast();

                if let Some(old_value) = old_value {
                    old_value.logical_children().remove(&child);
                }

                if let Some(new_value) = new_value {
                    new_value.logical_children().add(child);
                }
            }
        }

        self.invalidate();
    }

    fn on_inline_host_changed(&self, new_value: Option<Rc<dyn IInlineHost>>) {
        for child in self.0.list.snapshot().iter() {
            child.set_inline_host(new_value.clone());
        }

        self.invalidate();
    }

    fn on_add(&self, inline: &Ref<Inline>) {
        inline.set_inline_host(self.inline_host());

        if let Some(logical_children) = self.logical_children() {
            logical_children.logical_children().add(inline.clone().upcast());
        }

        self.invalidate();
    }

    fn on_remove(&self, inline: &Ref<Inline>) {
        if let Some(logical_children) = self.logical_children() {
            logical_children.logical_children().remove(&inline.clone().upcast());
        }

        inline.set_inline_host(None);

        self.invalidate();
    }
}
