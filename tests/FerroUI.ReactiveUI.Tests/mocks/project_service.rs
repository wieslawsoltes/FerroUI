//! `ProjectService` (`ReactiveUI.Tests/ReactiveObjects/Mocks/ProjectService.cs`).

use super::Project;
use ferroui_reactiveui::{reactive_object, ReactiveObject};
use ferroui_splat::Guid;
use ferroui_system::collections::generic::Dictionary;
use std::rc::Rc;

/// A service with five projects, and four projects plus a missing one (`ProjectService`).
pub struct ProjectService {
    reactive: ReactiveObject,
    projects: Dictionary<Guid, Rc<Project>>,
    projects_nullable: Dictionary<Guid, Option<Rc<Project>>>,
}
reactive_object!(ProjectService);

impl Default for ProjectService {
    fn default() -> Self {
        let mut projects = Dictionary::new();
        for name in ["Dummy1", "Dummy2", "Dummy3", "Dummy4", "Dummy5"] {
            projects.add(Guid::new_guid(), Project::with_name(name));
        }

        let mut projects_nullable = Dictionary::new();
        for name in ["Dummy1", "Dummy2", "Dummy3", "Dummy4"] {
            projects_nullable.add(Guid::new_guid(), Some(Project::with_name(name)));
        }
        projects_nullable.add(Guid::new_guid(), None);

        Self { reactive: ReactiveObject::new(), projects, projects_nullable }
    }
}

/// Services compare by identity, as the reference types of upstream do.
impl PartialEq for ProjectService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ProjectService {
    /// A service with its projects, shared as view models are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// `Projects`.
    pub fn projects(&self) -> &Dictionary<Guid, Rc<Project>> {
        &self.projects
    }

    /// `ProjectsNullable`.
    pub fn projects_nullable(&self) -> &Dictionary<Guid, Option<Rc<Project>>> {
        &self.projects_nullable
    }
}
