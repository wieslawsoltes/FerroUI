use std::collections::VecDeque;

/// FIFO Queue optimized for holding zero or one items.
pub struct SingleOrQueue<T> {
    head: Option<T>,
    tail: Option<VecDeque<T>>,
    empty: bool,
}

impl<T> Default for SingleOrQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SingleOrQueue<T> {
    pub fn new() -> Self {
        Self { head: None, tail: None, empty: true }
    }

    fn tail(&mut self) -> &mut VecDeque<T> {
        self.tail.get_or_insert_with(VecDeque::new)
    }

    fn has_tail(&self) -> bool {
        self.tail.is_some()
    }

    pub fn empty(&self) -> bool {
        self.empty
    }

    pub fn enqueue(&mut self, value: T) {
        if self.empty {
            self.head = Some(value);
        } else {
            self.tail().push_back(value);
        }

        self.empty = false;
    }

    /// Panics when the queue is empty.
    pub fn dequeue(&mut self) -> T {
        if self.empty {
            panic!("Cannot dequeue from an empty queue!");
        }

        let result = self.head.take();

        if self.has_tail() && !self.tail().is_empty() {
            self.head = self.tail().pop_front();
        } else {
            self.head = None;
            self.empty = true;
        }

        match result {
            Some(result) => result,
            None => unreachable!("a queue that is not empty has a head"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_single_or_queue_is_empty() {
        assert!(SingleOrQueue::<String>::new().empty());
    }

    #[test]
    #[should_panic(expected = "Cannot dequeue from an empty queue!")]
    fn dequeue_throws_when_empty() {
        let mut queue = SingleOrQueue::<String>::new();

        queue.dequeue();
    }

    #[test]
    fn enqueue_adds_element() {
        let mut queue = SingleOrQueue::<i32>::new();

        queue.enqueue(1);

        assert!(!queue.empty());

        assert_eq!(1, queue.dequeue());
    }

    #[test]
    fn multiple_elements_dequeued_in_correct_order() {
        let mut queue = SingleOrQueue::<i32>::new();

        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);
        assert_eq!(1, queue.dequeue());
        assert_eq!(2, queue.dequeue());
        assert_eq!(3, queue.dequeue());
    }
}
