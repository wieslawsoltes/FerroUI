use std::sync::Arc;

use super::dispatcher_operation::OperationCore;
use super::DispatcherPriority;

/// Slot value stored in an operation that is not queued.
pub(crate) const NOT_QUEUED: usize = usize::MAX;

/// One chain per priority value, `INVALID..=MAX_VALUE`.
const CHAIN_COUNT: usize = (DispatcherPriority::MAX_VALUE.value() - DispatcherPriority::INVALID.value() + 1) as usize;

#[inline]
fn chain_index(priority: DispatcherPriority) -> usize {
    (priority.value() - DispatcherPriority::INVALID.value()) as usize
}

#[inline]
fn chain_priority(index: usize) -> DispatcherPriority {
    if index == 0 {
        DispatcherPriority::INVALID
    } else {
        DispatcherPriority::from_value(index as i32 + DispatcherPriority::INVALID.value())
    }
}

#[derive(Clone, Copy, Default)]
struct PriorityChain {
    count: usize,
    head: Option<usize>,
    tail: Option<usize>,
}

#[derive(Default)]
struct Node {
    item: Option<Arc<OperationCore>>,
    sequential_prev: Option<usize>,
    sequential_next: Option<usize>,
    priority_prev: Option<usize>,
    priority_next: Option<usize>,
    chain: Option<usize>,
}

/// The dispatcher work queue.
///
/// Items are linked into two intrusive lists at once: a "sequential" list in
/// arrival order and one list per priority. The highest priority chain is
/// served first; within a chain the arrival order is kept, even after the
/// priority of an item has been changed.
///
/// The links live in a slab indexed by the slot stored in each operation, and
/// the per-priority chains in a fixed array with a bitmask of non-empty
/// chains, so queueing does not allocate once the slab has grown.
pub(crate) struct DispatcherPriorityQueue {
    nodes: Vec<Node>,
    free: Vec<usize>,
    priority_chains: [PriorityChain; CHAIN_COUNT],
    non_empty_chains: u32,
    head: Option<usize>,
    tail: Option<usize>,
}

impl DispatcherPriorityQueue {
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free: Vec::new(),
            priority_chains: [PriorityChain::default(); CHAIN_COUNT],
            non_empty_chains: 0,
            head: None,
            tail: None,
        }
    }

    fn max_chain(&self) -> Option<usize> {
        if self.non_empty_chains == 0 {
            None
        } else {
            Some(31 - self.non_empty_chains.leading_zeros() as usize)
        }
    }

    pub(crate) fn max_priority(&self) -> DispatcherPriority {
        match self.max_chain() {
            Some(index) => chain_priority(index),
            None => DispatcherPriority::INVALID,
        }
    }

    pub(crate) fn enqueue(&mut self, priority: DispatcherPriority, item: Arc<OperationCore>) {
        debug_assert_eq!(item.queue_slot(), NOT_QUEUED, "item must not already be queued");

        let slot = match self.free.pop() {
            Some(slot) => slot,
            None => {
                self.nodes.push(Node::default());
                self.nodes.len() - 1
            }
        };
        item.set_queue_slot(slot);
        self.nodes[slot].item = Some(item);

        // Find the existing chain for this priority.
        let chain = chain_index(priority);

        // Step 1: Append this to the end of the "sequential" linked list.
        self.insert_item_in_sequential_chain(slot, self.tail);

        // Step 2: Append the item into the priority chain.
        let after = self.priority_chains[chain].tail;
        self.insert_item_in_priority_chain_after(slot, chain, after);
    }

    #[allow(dead_code)]
    pub(crate) fn dequeue(&mut self) -> Arc<OperationCore> {
        // Get the max-priority chain.
        match self.peek() {
            Some(item) => {
                self.remove_item(&item);
                item
            }
            None => panic!("The dispatcher queue is empty"),
        }
    }

    pub(crate) fn peek(&self) -> Option<Arc<OperationCore>> {
        // Get the max-priority chain.
        let chain = self.max_chain()?;
        let head = self.priority_chains[chain].head;
        debug_assert!(head.is_some(), "a priority item should exist");
        head.and_then(|slot| self.nodes[slot].item.clone())
    }

    pub(crate) fn remove_item(&mut self, item: &Arc<OperationCore>) {
        let slot = item.queue_slot();
        debug_assert!(slot != NOT_QUEUED, "a chain should exist");
        if slot == NOT_QUEUED {
            return;
        }

        // Step 1: Remove the item from its priority chain.
        self.remove_item_from_priority_chain(slot);

        // Step 2: Remove the item from the sequential chain.
        self.remove_item_from_sequential_chain(slot);

        item.set_queue_slot(NOT_QUEUED);
        self.nodes[slot].item = None;
        self.free.push(slot);
    }

    pub(crate) fn change_item_priority(&mut self, item: &Arc<OperationCore>, priority: DispatcherPriority) {
        // Remove the item from its current priority and insert it into
        // the new priority chain.  Note that this does not change the
        // sequential ordering.
        let slot = item.queue_slot();
        if slot == NOT_QUEUED {
            return;
        }

        // Step 1: Remove the item from the priority chain.
        self.remove_item_from_priority_chain(slot);

        // Step 2: Insert the item into the new priority chain.
        self.insert_item_in_priority_chain(slot, chain_index(priority));
    }

    fn insert_item_in_priority_chain(&mut self, slot: usize, chain: usize) {
        // Scan along the sequential chain, in the previous direction,
        // looking for an item that is already in the new chain.  We will
        // insert ourselves after the item we found.  We can short-circuit
        // this search if the new chain is empty.
        if self.priority_chains[chain].head.is_none() {
            debug_assert!(self.priority_chains[chain].tail.is_none());
            self.insert_item_in_priority_chain_after(slot, chain, None);
        } else {
            debug_assert!(self.priority_chains[chain].tail.is_some());

            // Search backwards along the sequential chain looking for an
            // item already in this list.
            let mut after = self.nodes[slot].sequential_prev;
            while let Some(candidate) = after {
                if self.nodes[candidate].chain == Some(chain) {
                    break;
                }
                after = self.nodes[candidate].sequential_prev;
            }

            self.insert_item_in_priority_chain_after(slot, chain, after);
        }
    }

    fn insert_item_in_priority_chain_after(&mut self, slot: usize, chain: usize, after: Option<usize>) {
        debug_assert!(
            self.nodes[slot].chain.is_none()
                && self.nodes[slot].priority_prev.is_none()
                && self.nodes[slot].priority_next.is_none(),
            "item must not already be in a priority chain"
        );

        self.nodes[slot].chain = Some(chain);

        match after {
            None => {
                // Note: passing None for after means insert at the head.
                match self.priority_chains[chain].head {
                    Some(head) => {
                        debug_assert!(self.priority_chains[chain].tail.is_some());
                        self.nodes[head].priority_prev = Some(slot);
                        self.nodes[slot].priority_next = Some(head);
                        self.priority_chains[chain].head = Some(slot);
                    }
                    None => {
                        debug_assert!(self.priority_chains[chain].tail.is_none());
                        self.priority_chains[chain].head = Some(slot);
                        self.priority_chains[chain].tail = Some(slot);
                    }
                }
            }
            Some(after) => {
                self.nodes[slot].priority_prev = Some(after);

                match self.nodes[after].priority_next {
                    Some(next) => {
                        self.nodes[slot].priority_next = Some(next);
                        self.nodes[next].priority_prev = Some(slot);
                        self.nodes[after].priority_next = Some(slot);
                    }
                    None => {
                        debug_assert_eq!(self.priority_chains[chain].tail, Some(after));
                        self.nodes[after].priority_next = Some(slot);
                        self.priority_chains[chain].tail = Some(slot);
                    }
                }
            }
        }

        self.priority_chains[chain].count += 1;
        self.non_empty_chains |= 1 << chain;
    }

    fn remove_item_from_priority_chain(&mut self, slot: usize) {
        let Some(chain) = self.nodes[slot].chain else {
            debug_assert!(false, "a chain should exist");
            return;
        };
        let prev = self.nodes[slot].priority_prev;
        let next = self.nodes[slot].priority_next;

        // Step 1: Fix up the previous link
        match prev {
            Some(prev) => {
                debug_assert_ne!(self.priority_chains[chain].head, Some(slot));
                self.nodes[prev].priority_next = next;
            }
            None => {
                debug_assert_eq!(self.priority_chains[chain].head, Some(slot));
                self.priority_chains[chain].head = next;
            }
        }

        // Step 2: Fix up the next link
        match next {
            Some(next) => {
                debug_assert_ne!(self.priority_chains[chain].tail, Some(slot));
                self.nodes[next].priority_prev = prev;
            }
            None => {
                debug_assert_eq!(self.priority_chains[chain].tail, Some(slot));
                self.priority_chains[chain].tail = prev;
            }
        }

        // Step 3: cleanup
        self.nodes[slot].priority_prev = None;
        self.nodes[slot].priority_next = None;
        self.priority_chains[chain].count -= 1;
        if self.priority_chains[chain].count == 0 {
            self.non_empty_chains &= !(1 << chain);
        }

        self.nodes[slot].chain = None;
    }

    fn insert_item_in_sequential_chain(&mut self, slot: usize, after: Option<usize>) {
        debug_assert!(
            self.nodes[slot].sequential_prev.is_none() && self.nodes[slot].sequential_next.is_none(),
            "item must not already be in the sequential chain"
        );

        match after {
            None => {
                // Note: passing None for after means insert at the head.
                match self.head {
                    Some(head) => {
                        debug_assert!(self.tail.is_some());
                        self.nodes[head].sequential_prev = Some(slot);
                        self.nodes[slot].sequential_next = Some(head);
                        self.head = Some(slot);
                    }
                    None => {
                        debug_assert!(self.tail.is_none());
                        self.head = Some(slot);
                        self.tail = Some(slot);
                    }
                }
            }
            Some(after) => {
                self.nodes[slot].sequential_prev = Some(after);

                match self.nodes[after].sequential_next {
                    Some(next) => {
                        self.nodes[slot].sequential_next = Some(next);
                        self.nodes[next].sequential_prev = Some(slot);
                        self.nodes[after].sequential_next = Some(slot);
                    }
                    None => {
                        debug_assert_eq!(self.tail, Some(after));
                        self.nodes[after].sequential_next = Some(slot);
                        self.tail = Some(slot);
                    }
                }
            }
        }
    }

    fn remove_item_from_sequential_chain(&mut self, slot: usize) {
        let prev = self.nodes[slot].sequential_prev;
        let next = self.nodes[slot].sequential_next;

        // Step 1: Fix up the previous link
        match prev {
            Some(prev) => {
                debug_assert_ne!(self.head, Some(slot));
                self.nodes[prev].sequential_next = next;
            }
            None => {
                debug_assert_eq!(self.head, Some(slot));
                self.head = next;
            }
        }

        // Step 2: Fix up the next link
        match next {
            Some(next) => {
                debug_assert_ne!(self.tail, Some(slot));
                self.nodes[next].sequential_prev = prev;
            }
            None => {
                debug_assert_eq!(self.tail, Some(slot));
                self.tail = prev;
            }
        }

        // Step 3: cleanup
        self.nodes[slot].sequential_prev = None;
        self.nodes[slot].sequential_next = None;
    }

    /// All queued items in arrival order, without removing them.
    pub(crate) fn peek_all(&self) -> Vec<Arc<OperationCore>> {
        let mut operations = Vec::new();
        let mut item = self.head;
        while let Some(slot) = item {
            if let Some(operation) = &self.nodes[slot].item {
                operations.push(operation.clone());
            }
            item = self.nodes[slot].sequential_next;
        }
        operations
    }

    /// Empties the queue and returns the items that were in it so that the
    /// caller can drop them outside of the dispatcher lock.
    pub(crate) fn clear(&mut self) -> Vec<Arc<OperationCore>> {
        let operations = self.peek_all();
        for operation in &operations {
            operation.set_queue_slot(NOT_QUEUED);
        }
        self.nodes.clear();
        self.free.clear();
        self.priority_chains = [PriorityChain::default(); CHAIN_COUNT];
        self.non_empty_chains = 0;
        self.head = None;
        self.tail = None;
        operations
    }
}
