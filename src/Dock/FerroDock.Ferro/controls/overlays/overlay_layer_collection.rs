// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::IOverlayLayer;
use ferroui_base::collections::FerroList;
use std::ops::Deref;
use std::rc::Rc;
/// Source observable layer list, retaining the shared framework's collection engine.
#[derive(Clone, PartialEq)]
pub struct OverlayLayerCollection(FerroList<Option<Rc<dyn IOverlayLayer>>>);
impl Default for OverlayLayerCollection {
    fn default() -> Self {
        Self::new()
    }
}
impl Deref for OverlayLayerCollection {
    type Target = FerroList<Option<Rc<dyn IOverlayLayer>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl OverlayLayerCollection {
    /// Creates an empty native list.
    pub fn new() -> Self {
        Self(FerroList::new())
    }
    /// Returns a lazy, repeatable stable ordering query, retaining this list.
    pub fn get_ordered_layers(&self) -> OrderedOverlayLayers {
        OrderedOverlayLayers(self.clone())
    }
}
/// Repeatable enumeration of source z-order followed by insertion order.
#[derive(Clone)]
pub struct OrderedOverlayLayers(OverlayLayerCollection);
impl OrderedOverlayLayers {
    /// Gets a fresh iterator. Source buffering/key computation starts at first next.
    pub fn iter(&self) -> OrderedOverlayLayerIterator {
        OrderedOverlayLayerIterator {
            source: Some(self.0.clone()),
            ordered: None,
        }
    }
}
type OrderedEntry = (i32, usize, Rc<dyn IOverlayLayer>);

/// Native lazy iterator retaining the original layer reference identities.
pub struct OrderedOverlayLayerIterator {
    source: Option<OverlayLayerCollection>,
    ordered: Option<std::vec::IntoIter<OrderedEntry>>,
}
impl Iterator for OrderedOverlayLayerIterator {
    type Item = Rc<dyn IOverlayLayer>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(source) = self.source.take() {
            // LINQ buffers the source before calling any sort-key getter. A
            // getter can mutate the original list without changing that buffer.
            let values = source.snapshot().as_ref().clone();
            let mut ordered = Vec::with_capacity(values.len());
            for (index, layer) in values.into_iter().enumerate() {
                let layer = layer.expect("source layer cannot be null while ordering");
                let key = layer.z_index();
                ordered.push((key, index, layer));
            }
            ordered.sort_by_key(|(z, index, _)| (*z, *index));
            self.ordered = Some(ordered.into_iter());
        }
        self.ordered.as_mut()?.next().map(|(_, _, layer)| layer)
    }
}
impl IntoIterator for OrderedOverlayLayers {
    type Item = Rc<dyn IOverlayLayer>;
    type IntoIter = OrderedOverlayLayerIterator;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

ferroui_base::ferro_markup_type!(class OverlayLayerCollection {
    this:OverlayLayerCollection,
    handles:[OverlayLayerCollection,Option<OverlayLayerCollection>],
    constructors:[()=>OverlayLayerCollection::new],
    methods:[fn Add(Option<Rc<dyn IOverlayLayer>>)=>|this:&OverlayLayerCollection,value|this.add(value)]
});
