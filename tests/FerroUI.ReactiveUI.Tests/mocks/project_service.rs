//! `ProjectService` (`ReactiveUI.Tests/ReactiveObjects/Mocks/ProjectService.cs`).

use super::Project;
use ferroui_reactiveui::{reactive_object, ReactiveObject};
use ferroui_system::collections::generic::SlotOrderMap;
use std::rc::Rc;
use uuid::Uuid;

/// A service with five projects, and four projects plus a missing one (`ProjectService`).
pub struct ProjectService {
    reactive: ReactiveObject,
    projects: SlotOrderMap<Uuid, Rc<Project>>,
    projects_nullable: SlotOrderMap<Uuid, Option<Rc<Project>>>,
}
reactive_object!(ProjectService);

impl Default for ProjectService {
    fn default() -> Self {
        let mut projects = SlotOrderMap::new();
        for name in ["Dummy1", "Dummy2", "Dummy3", "Dummy4", "Dummy5"] {
            projects.add(Uuid::new_v4(), Project::with_name(name));
        }

        let mut projects_nullable = SlotOrderMap::new();
        for name in ["Dummy1", "Dummy2", "Dummy3", "Dummy4"] {
            projects_nullable.add(Uuid::new_v4(), Some(Project::with_name(name)));
        }
        projects_nullable.add(Uuid::new_v4(), None);

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
    pub fn projects(&self) -> &SlotOrderMap<Uuid, Rc<Project>> {
        &self.projects
    }

    /// `ProjectsNullable`.
    pub fn projects_nullable(&self) -> &SlotOrderMap<Uuid, Option<Rc<Project>>> {
        &self.projects_nullable
    }
}
