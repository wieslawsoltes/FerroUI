//! The port of `ApplicationAtSpiNode.cs`: the root object of the
//! application, whose children are its windows.

use super::at_spi_constants::ROOT_PATH;
use super::at_spi_node::AtSpiNode;
use super::at_spi_role::AtSpiRole;
use ferroui_controls::Application;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct ApplicationAtSpiNode {
    window_children: RefCell<Vec<Rc<AtSpiNode>>>,
    name: String,
}

impl ApplicationAtSpiNode {
    pub(crate) fn new(application_name: Option<String>) -> Self {
        let name = application_name
            .or_else(|| Application::current().and_then(|application| application.name()))
            .unwrap_or_else(process_name);
        Self { window_children: RefCell::new(Vec::new()), name }
    }

    pub(crate) fn path(&self) -> &'static str {
        ROOT_PATH
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn role(&self) -> AtSpiRole {
        AtSpiRole::Application
    }

    pub(crate) fn window_children(&self) -> Vec<Rc<AtSpiNode>> {
        self.window_children.borrow().clone()
    }

    pub(crate) fn add_window_child(&self, window_node: &Rc<AtSpiNode>) {
        self.window_children.borrow_mut().push(window_node.clone());
    }

    pub(crate) fn remove_window_child(&self, window_node: &Rc<AtSpiNode>) {
        self.window_children.borrow_mut().retain(|child| !Rc::ptr_eq(child, window_node));
    }
}

/// `Process.GetCurrentProcess().ProcessName`: the file stem of the executable.
fn process_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()))
        .unwrap_or_default()
}
