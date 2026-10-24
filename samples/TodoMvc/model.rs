//! The model of TodoMVC without user interface: a todo, the list of todos
//! with its operations and counts, and the filters of the list. The view
//! (`view.rs`) keeps a [`Todos`] in a signal and calls these operations from
//! its handlers.

/// A todo of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Todo {
    /// The identity of the todo in the list: the key of its item.
    pub id: u32,
    /// What there is to do, trimmed and never empty.
    pub title: String,
    /// Whether it is done.
    pub completed: bool,
}

/// The filters of the list, each the route of a page of the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Filter {
    /// Every todo (`/`).
    All,
    /// The todos that are not done (`/active`).
    Active,
    /// The todos that are done (`/completed`).
    Completed,
}

impl Filter {
    /// The filters, in the order of the footer.
    pub const ALL: [Filter; 3] = [Filter::All, Filter::Active, Filter::Completed];

    /// The route of the filter.
    pub fn path(self) -> &'static str {
        match self {
            Filter::All => "/",
            Filter::Active => "/active",
            Filter::Completed => "/completed",
        }
    }

    /// The filter of a route, `None` for a route that is not a filter.
    pub fn from_path(path: &str) -> Option<Filter> {
        Filter::ALL.into_iter().find(|filter| filter.path() == path)
    }

    /// The text of the filter in the footer.
    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Active => "Active",
            Filter::Completed => "Completed",
        }
    }

    /// Whether the filter shows `todo`.
    pub fn matches(self, todo: &Todo) -> bool {
        match self {
            Filter::All => true,
            Filter::Active => !todo.completed,
            Filter::Completed => todo.completed,
        }
    }
}

/// The list of todos, in the order they were added.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Todos {
    items: Vec<Todo>,
    /// The identity the last todo added took.
    last_id: u32,
}

impl Todos {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// A list of the todos `(title, completed)`, added in order.
    pub fn from_titles(titles: &[(&str, bool)]) -> Self {
        let mut todos = Self::new();
        for &(title, completed) in titles {
            if let Some(id) = todos.add(title) {
                if completed {
                    todos.toggle(id);
                }
            }
        }
        todos
    }

    /// The todos, in order.
    pub fn items(&self) -> &[Todo] {
        &self.items
    }

    /// The number of todos.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are no todos.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The todo `id`.
    pub fn get(&self, id: u32) -> Option<&Todo> {
        self.items.iter().find(|todo| todo.id == id)
    }

    /// Adds a todo of `title`, trimmed, at the end of the list, and returns
    /// its identity; a title that is empty once trimmed adds nothing.
    pub fn add(&mut self, title: &str) -> Option<u32> {
        let title = title.trim();
        if title.is_empty() {
            return None;
        }
        self.last_id += 1;
        let id = self.last_id;
        self.items.push(Todo { id, title: title.to_string(), completed: false });
        Some(id)
    }

    /// Marks the todo `id` done if it is not, and not done if it is.
    pub fn toggle(&mut self, id: u32) {
        if let Some(todo) = self.items.iter_mut().find(|todo| todo.id == id) {
            todo.completed = !todo.completed;
        }
    }

    /// Marks every todo done (`true`) or not done (`false`).
    pub fn set_all_completed(&mut self, completed: bool) {
        for todo in &mut self.items {
            todo.completed = completed;
        }
    }

    /// Gives the todo `id` the title `title`, trimmed; an empty title
    /// removes the todo, as the specification of TodoMVC asks.
    pub fn rename(&mut self, id: u32, title: &str) {
        let title = title.trim();
        if title.is_empty() {
            self.remove(id);
        } else if let Some(todo) = self.items.iter_mut().find(|todo| todo.id == id) {
            todo.title = title.to_string();
        }
    }

    /// Removes the todo `id`.
    pub fn remove(&mut self, id: u32) {
        self.items.retain(|todo| todo.id != id);
    }

    /// Removes the todos that are done.
    pub fn clear_completed(&mut self) {
        self.items.retain(|todo| !todo.completed);
    }

    /// The number of todos that are not done.
    pub fn active_count(&self) -> usize {
        self.items.iter().filter(|todo| !todo.completed).count()
    }

    /// The number of todos that are done.
    pub fn completed_count(&self) -> usize {
        self.items.iter().filter(|todo| todo.completed).count()
    }

    /// Whether there are todos and every one is done: the state of the
    /// check box that marks them all.
    pub fn all_completed(&self) -> bool {
        !self.items.is_empty() && self.items.iter().all(|todo| todo.completed)
    }

    /// The todos `filter` shows, in order.
    pub fn visible(&self, filter: Filter) -> Vec<Todo> {
        self.items.iter().filter(|todo| filter.matches(todo)).cloned().collect()
    }
}

/// The count of the footer: `1 item left`, `3 items left`.
pub fn items_left(count: usize) -> String {
    if count == 1 {
        String::from("1 item left")
    } else {
        format!("{count} items left")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_todo_is_added_trimmed_and_an_empty_title_adds_nothing() {
        let mut todos = Todos::new();
        assert_eq!(todos.add("  Buy milk  "), Some(1));
        assert_eq!(todos.add("   "), None);
        assert_eq!(todos.add(""), None);
        assert_eq!(todos.add("Walk the dog"), Some(2));
        let titles: Vec<&str> = todos.items().iter().map(|todo| todo.title.as_str()).collect();
        assert_eq!(titles, ["Buy milk", "Walk the dog"]);
        assert!(todos.items().iter().all(|todo| !todo.completed));
    }

    #[test]
    fn identities_are_not_reused_after_a_removal() {
        let mut todos = Todos::new();
        let first = todos.add("a");
        todos.add("b");
        todos.remove(2);
        assert_eq!(todos.add("c"), Some(3));
        assert_eq!(first, Some(1));
    }

    #[test]
    fn toggling_counts_and_clearing() {
        let mut todos = Todos::from_titles(&[("a", false), ("b", true), ("c", false)]);
        assert_eq!((todos.active_count(), todos.completed_count()), (2, 1));
        todos.toggle(1);
        assert_eq!((todos.active_count(), todos.completed_count()), (1, 2));
        todos.clear_completed();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos.get(3).map(|todo| todo.title.as_str()), Some("c"));
    }

    #[test]
    fn every_todo_is_marked_at_once() {
        let mut todos = Todos::from_titles(&[("a", false), ("b", true)]);
        assert!(!todos.all_completed());
        todos.set_all_completed(true);
        assert!(todos.all_completed());
        todos.set_all_completed(false);
        assert_eq!(todos.active_count(), 2);
        assert!(!Todos::new().all_completed(), "an empty list is not all done");
    }

    #[test]
    fn a_rename_trims_and_an_empty_title_removes_the_todo() {
        let mut todos = Todos::from_titles(&[("a", false), ("b", false)]);
        todos.rename(1, "  A  ");
        assert_eq!(todos.get(1).map(|todo| todo.title.as_str()), Some("A"));
        todos.rename(2, "  ");
        assert_eq!(todos.get(2), None);
        assert_eq!(todos.len(), 1);
    }

    #[test]
    fn filters_show_their_todos_and_have_routes() {
        let todos = Todos::from_titles(&[("a", false), ("b", true), ("c", false)]);
        let ids = |filter| todos.visible(filter).iter().map(|todo| todo.id).collect::<Vec<_>>();
        assert_eq!(ids(Filter::All), [1, 2, 3]);
        assert_eq!(ids(Filter::Active), [1, 3]);
        assert_eq!(ids(Filter::Completed), [2]);
        for filter in Filter::ALL {
            assert_eq!(Filter::from_path(filter.path()), Some(filter));
        }
        assert_eq!(Filter::from_path("/other"), None);
    }

    #[test]
    fn the_count_of_the_footer_is_singular_for_one() {
        assert_eq!(items_left(0), "0 items left");
        assert_eq!(items_left(1), "1 item left");
        assert_eq!(items_left(2), "2 items left");
    }
}
