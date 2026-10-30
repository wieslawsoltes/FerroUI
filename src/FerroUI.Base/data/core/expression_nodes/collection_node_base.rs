use super::ExpressionNode;
use crate::collections::NotifyCollectionChangedAction;
use crate::data::core::plugins::InpcPropertyAccessor;
use crate::data::model::{CollectionChange, ModelTypes};
use crate::data::BindingError;
use crate::BoxedValue;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The members a node that reads from a collection provides to
/// [`CollectionNodeBase`].
pub trait CollectionNode: ExpressionNode {
    fn collection_base(&self) -> &CollectionNodeBase;

    /// Whether a property change notification of the source for
    /// `property_name` affects the value of the node.
    fn should_update(&self, source: &BoxedValue, property_name: &str) -> bool;

    /// The first argument of the node as an integer, if it is one.
    fn try_get_first_argument_as_int(&self) -> Option<i32>;

    /// Reads the value from the source. An error is what the getter of a
    /// managed indexer throws.
    fn update_value(&self, source: &BoxedValue) -> Result<(), BindingError>;
}

/// The part shared by nodes that read from a collection: following the
/// collection change and property change notifications of the source.
#[derive(Default)]
pub struct CollectionNodeBase {
    collection_token: Cell<Option<u64>>,
    property_token: Cell<Option<u64>>,
}

impl CollectionNodeBase {
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribes `node` to the new source and updates its value.
    pub fn on_source_changed<N: CollectionNode>(this: &Weak<N>, node: &N, source: Option<&BoxedValue>) {
        if !node.state().validate_non_null_source(source) {
            return;
        }
        let Some(source) = source else { return };
        node.collection_base().subscribe(this, source);
        Self::update_value_or_set_error(node, source);
    }

    /// Unsubscribes from the previous source.
    pub fn unsubscribe(&self, source: &BoxedValue) {
        let collection_token = self.collection_token.take();
        let property_token = self.property_token.take();
        if collection_token.is_none() && property_token.is_none() {
            return;
        }
        let model = ModelTypes::find(&**source);
        let incc = model.as_ref().and_then(|model| model.as_notify_collection_changed(&**source));
        if let (Some(token), Some(incc)) = (collection_token, incc) {
            incc.collection_changed().remove(token);
        }
        if let (Some(token), Some(inpc)) = (property_token, InpcPropertyAccessor::find_notifier(&**source)) {
            inpc.property_changed().remove(token);
        }
    }

    fn should_update_for_collection_change<N: CollectionNode>(node: &N, source: &BoxedValue, e: &CollectionChange) -> bool {
        let is_list = ModelTypes::find(&**source).is_some_and(|m| m.as_list(&**source).is_some());
        if let (true, Some(index)) = (is_list, node.try_get_first_argument_as_int()) {
            let new_end = e.new_starting_index + e.new_count as i32;
            let old_end = e.old_starting_index + e.old_count as i32;
            return match e.action {
                NotifyCollectionChangedAction::Add => index >= e.new_starting_index,
                NotifyCollectionChangedAction::Remove => index >= e.old_starting_index,
                NotifyCollectionChangedAction::Replace => index >= e.new_starting_index && index < new_end,
                NotifyCollectionChangedAction::Move => {
                    (index >= e.new_starting_index && index < new_end)
                        || (index >= e.old_starting_index && index < old_end)
                }
                NotifyCollectionChangedAction::Reset => true,
            };
        }

        // Implementation defined meaning for the index, so just try to update anyway.
        true
    }

    fn subscribe<N: CollectionNode>(&self, this: &Weak<N>, source: &BoxedValue) {
        // Collection changes are declared in binding metadata; property
        // changes also in markup metadata.
        let model = ModelTypes::find(&**source);
        if let Some(incc) = model.as_ref().and_then(|model| model.as_notify_collection_changed(&**source)) {
            let weak = this.clone();
            let token = incc.collection_changed().add(Rc::new(move |e: &CollectionChange| {
                let Some(node) = weak.upgrade() else { return };
                let Some(source) = node.state().source() else { return };
                if Self::should_update_for_collection_change(&*node, &source, e) {
                    Self::update_value_or_set_error(&*node, &source);
                }
            }));
            self.collection_token.set(Some(token));
        }
        if let Some(inpc) = InpcPropertyAccessor::find_notifier(&**source) {
            let weak = this.clone();
            let token = inpc.property_changed().add(Rc::new(move |property_name: &str| {
                let Some(node) = weak.upgrade() else { return };
                let Some(source) = node.state().source() else { return };
                if node.should_update(&source, property_name) {
                    Self::update_value_or_set_error(&*node, &source);
                }
            }));
            self.property_token.set(Some(token));
        }
    }

    fn update_value_or_set_error<N: CollectionNode>(node: &N, source: &BoxedValue) {
        if let Err(e) = node.update_value(source) {
            node.state().set_error(&e.to_string());
        }
    }
}
