//! Markup metadata of the routed events of the grid (`RowDragStarted`, `RowDragOver`, `RowDrop`):
//! their typed routed event types and their arguments, which a document names when it attaches a
//! handler (`RowDragStarted="OnRowDragStarted"`).

use crate::{TreeDataGridRowDragEventArgs, TreeDataGridRowDragStartedEventArgs};
use ferroui_base::ferro_markup_type;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEvent};
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use std::rc::Rc;

/// The typed routed event type `RoutedEvent<TEventArgs>` of the events of the grid.
pub(crate) struct RoutedEventOf<TEventArgs>(std::marker::PhantomData<TEventArgs>);

ferro_markup_type!(class RoutedEventOf<TreeDataGridRowDragStartedEventArgs> as "RoutedEvent`1" {
    namespace: "FerroUI.Interactivity",
    handles: [RoutedEvent<TreeDataGridRowDragStartedEventArgs>, Option<RoutedEvent<TreeDataGridRowDragStartedEventArgs>>],
    base: RoutedEvent,
    generic: "RoutedEvent`1" [TreeDataGridRowDragStartedEventArgs],
});

ferro_markup_type!(class RoutedEventOf<TreeDataGridRowDragEventArgs> as "RoutedEvent`1" {
    namespace: "FerroUI.Interactivity",
    handles: [RoutedEvent<TreeDataGridRowDragEventArgs>, Option<RoutedEvent<TreeDataGridRowDragEventArgs>>],
    base: RoutedEvent,
    generic: "RoutedEvent`1" [TreeDataGridRowDragEventArgs],
});

ferro_markup_type!(class TreeDataGridRowDragStartedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [TreeDataGridRowDragStartedEventArgs],
    this: Rc<dyn IRoutedEventArgs>,
    base: Rc<dyn IRoutedEventArgs>,
});

ferro_markup_type!(class TreeDataGridRowDragEventArgs {
    namespace: "FerroUI.Controls",
    handles: [TreeDataGridRowDragEventArgs],
    this: Rc<dyn IRoutedEventArgs>,
    base: Rc<dyn IRoutedEventArgs>,
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <RoutedEventOf<TreeDataGridRowDragStartedEventArgs> as MarkupTyped>::MARKUP,
    <RoutedEventOf<TreeDataGridRowDragEventArgs> as MarkupTyped>::MARKUP,
    <TreeDataGridRowDragStartedEventArgs as MarkupTyped>::MARKUP,
    <TreeDataGridRowDragEventArgs as MarkupTyped>::MARKUP,
];
