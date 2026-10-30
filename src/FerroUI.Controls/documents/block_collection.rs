use super::{Block, IInlineHost};
use ferroui_base::collections::{
    FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs, ResetBehavior,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{IntoRef, Ref, StyledElement, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// An observable, shared collection of block elements. Clones retain identity.
#[derive(Clone)]
pub struct BlockCollection(Rc<BlockCollectionData>);

struct BlockCollectionData {
    list: FerroList<Ref<Block>>,
    logical_parent: RefCell<Option<WeakRef<StyledElement>>>,
    inline_host: RefCell<Option<Rc<dyn IInlineHost>>>,
    invalidated: Rc<HandlerList<dyn Fn()>>,
    update_depth: Cell<usize>,
    pending_invalidation: Cell<bool>,
}

impl PartialEq for BlockCollection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for BlockCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BlockCollection")
            .field(&self.count())
            .finish()
    }
}

impl Default for BlockCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for BlockCollection {
    type Target = FerroList<Ref<Block>>;
    fn deref(&self) -> &Self::Target {
        &self.0.list
    }
}

ferroui_base::ferro_markup_type!(class BlockCollection {
    namespace: "FerroUI.Controls.Documents",
    handles: [BlockCollection, Option<BlockCollection>],
    base: FerroList<Ref<Block>>,
    constructors: [() => BlockCollection::new],
    methods: [fn Add(Ref<Block>) => |blocks: &BlockCollection, block: Ref<Block>| blocks.add(block)],
});

impl BlockCollection {
    pub fn new() -> Self {
        let list = FerroList::new();
        list.set_reset_behavior(ResetBehavior::Remove);
        let collection = Self(Rc::new(BlockCollectionData {
            list,
            logical_parent: RefCell::new(None),
            inline_host: RefCell::new(None),
            invalidated: Rc::new(HandlerList::new()),
            update_depth: Cell::new(0),
            pending_invalidation: Cell::new(false),
        }));
        let weak = Rc::downgrade(&collection.0);
        collection.0.list.add_collection_changed(Rc::new(
            move |event: &NotifyCollectionChangedEventArgs<'_, Ref<Block>>| {
                let Some(collection) = weak.upgrade().map(BlockCollection) else {
                    return;
                };
                let _update = collection.begin_update();
                match event.action {
                    NotifyCollectionChangedAction::Add => {
                        for block in event.new_items {
                            collection.attach(block);
                        }
                    }
                    NotifyCollectionChangedAction::Remove => {
                        for block in event.old_items {
                            collection.detach(block);
                        }
                    }
                    NotifyCollectionChangedAction::Replace => {
                        for block in event.old_items {
                            collection.detach(block);
                        }
                        for block in event.new_items {
                            collection.attach(block);
                        }
                    }
                    NotifyCollectionChangedAction::Reset => {
                        for block in event.old_items {
                            collection.detach(block);
                        }
                        for block in event.new_items {
                            collection.attach(block);
                        }
                    }
                    NotifyCollectionChangedAction::Move => {}
                }
                collection.invalidate();
            },
        ));
        collection
    }

    pub fn add(&self, block: impl IntoRef<Block>) {
        self.0.list.add(block.into_ref());
    }

    pub fn insert(&self, index: usize, block: impl IntoRef<Block>) {
        self.0.list.insert(index, block.into_ref());
    }

    pub fn remove(&self, block: impl IntoRef<Block>) -> bool {
        self.0.list.remove(&block.into_ref())
    }

    pub fn set(&self, index: usize, block: impl IntoRef<Block>) {
        self.0.list.set(index, block.into_ref());
    }

    pub fn text(&self) -> String {
        let mut text = String::new();
        for (index, block) in self.snapshot().iter().enumerate() {
            if index != 0 {
                text.push('\n');
            }
            text.push_str(&block.text());
        }
        text
    }

    pub fn text_length(&self) -> usize {
        let blocks = self.snapshot();
        blocks.iter().map(|b| b.text_length()).sum::<usize>() + blocks.len().saturating_sub(1)
    }

    pub fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.0.invalidated.add(Rc::new(handler));
        let weak = Rc::downgrade(&self.0.invalidated);
        Disposable::create(move || {
            if let Some(handlers) = weak.upgrade() {
                handlers.remove(token);
            }
        })
    }

    pub(crate) fn set_logical_parent(&self, parent: Option<&StyledElement>) {
        let _update = self.begin_update();
        let old = self.logical_parent();
        let new = parent.map(StyledElement::to_ref);
        if old == new {
            return;
        }
        *self.0.logical_parent.borrow_mut() = new.as_ref().map(Ref::downgrade);
        for block in self.snapshot().iter() {
            let child: Ref<StyledElement> = block.clone().upcast();
            if let Some(old) = &old {
                old.logical_children().remove(&child);
            }
            if let Some(new) = &new {
                new.logical_children().add(child);
            }
        }
        self.invalidate();
    }

    pub(crate) fn set_inline_host(&self, host: Option<Rc<dyn IInlineHost>>) {
        let _update = self.begin_update();
        let blocks = self.snapshot();
        // Detach while the old forwarding destination is still available.
        for block in blocks.iter() {
            block.set_inline_host(None);
        }
        *self.0.inline_host.borrow_mut() = host;
        for block in blocks.iter() {
            block.set_inline_host(Some(self.make_host()));
        }
        self.invalidate();
    }

    pub(crate) fn logical_parent(&self) -> Option<Ref<StyledElement>> {
        self.0
            .logical_parent
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
    }

    fn make_host(&self) -> Rc<dyn IInlineHost> {
        Rc::new(BlockInlineHost(Rc::downgrade(&self.0)))
    }

    fn attach(&self, block: &Ref<Block>) {
        if let Some(parent) = self.logical_parent() {
            parent.logical_children().add(block.clone().upcast());
        }
        block.set_inline_host(Some(self.make_host()));
    }

    fn detach(&self, block: &Ref<Block>) {
        block.set_inline_host(None);
        if let Some(parent) = self.logical_parent() {
            parent.logical_children().remove(&block.clone().upcast());
        }
    }

    fn invalidate(&self) {
        if self.0.update_depth.get() != 0 {
            self.0.pending_invalidation.set(true);
            return;
        }
        for (_, handler) in self.0.invalidated.snapshot().iter() {
            handler();
        }
        let host = self.0.inline_host.borrow().clone();
        if let Some(host) = host {
            host.invalidate();
        }
    }

    fn begin_update(&self) -> BlockCollectionUpdate {
        self.0.update_depth.set(self.0.update_depth.get() + 1);
        BlockCollectionUpdate(self.clone())
    }
}

struct BlockCollectionUpdate(BlockCollection);
impl Drop for BlockCollectionUpdate {
    fn drop(&mut self) {
        let depth = self.0 .0.update_depth.get().saturating_sub(1);
        self.0 .0.update_depth.set(depth);
        if depth == 0 && self.0 .0.pending_invalidation.replace(false) {
            self.0.invalidate();
        }
    }
}

struct BlockInlineHost(Weak<BlockCollectionData>);

impl IInlineHost for BlockInlineHost {
    fn invalidate(&self) {
        if let Some(collection) = self.0.upgrade().map(BlockCollection) {
            collection.invalidate();
        }
    }

    fn invalidate_layout(&self) {
        let host = self
            .0
            .upgrade()
            .and_then(|data| data.inline_host.borrow().clone());
        if let Some(host) = host {
            host.invalidate_layout();
        }
    }

    fn with_visual_children(&self, f: &mut dyn FnMut(&FerroList<Ref<Visual>>)) {
        let host = self
            .0
            .upgrade()
            .and_then(|data| data.inline_host.borrow().clone());
        if let Some(host) = host {
            host.with_visual_children(f);
        }
    }

    fn invalidate_render(&self) {
        let host = self
            .0
            .upgrade()
            .and_then(|data| data.inline_host.borrow().clone());
        if let Some(host) = host {
            host.invalidate_render();
        }
    }
}
