//! `Project` (`ReactiveUI.Tests/ReactiveObjects/Mocks/Project.cs`).

use ferroui_reactiveui::{reactive_object, reactive_properties, ReactiveObject};
use std::rc::Rc;

reactive_properties! {
    /// A project (`Project`).
    #[derive(Default)]
    pub struct Project {
        reactive: ReactiveObject,
        #[reactive]
        name: Option<String>,
    }
}
reactive_object!(Project);

impl Project {
    /// A project named `name` (`new() { Name = .. }`).
    pub fn with_name(name: &str) -> Rc<Self> {
        let project = Rc::new(Self::default());
        project.set_name(Some(name.to_owned()));
        project
    }
}
