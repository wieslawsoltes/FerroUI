use super::{ContainerQueryActivator, ContainerQueryActivatorBase, StyleActivator, StyleActivatorBase};
use crate::styling::screen_queries::SizeQueryArgument;
use crate::styling::{ContainerSizing, HeightQuery, WidthQuery};
use crate::Visual;
use std::rc::{Rc, Weak};

/// An activator which is active while the width of the container of a visual
/// satisfies a comparison.
pub(crate) struct WidthActivator {
    inner: ContainerQueryActivatorBase,
    this: Weak<WidthActivator>,
    argument: SizeQueryArgument,
}

impl WidthActivator {
    pub fn new(visual: &Visual, argument: SizeQueryArgument, container_name: Option<&str>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            inner: ContainerQueryActivatorBase::new(visual, container_name),
            this: this.clone(),
            argument,
        })
    }
}

impl ContainerQueryActivator for WidthActivator {
    fn container_base(&self) -> &ContainerQueryActivatorBase {
        &self.inner
    }

    fn weak(&self) -> Weak<Self> {
        self.this.clone()
    }
}

impl StyleActivator for WidthActivator {
    fn base(&self) -> &StyleActivatorBase {
        &self.inner.base
    }

    fn evaluate_is_active(&self) -> bool {
        let provider = self
            .inner
            .current_query_provider(|sizing| matches!(sizing, ContainerSizing::Width | ContainerSizing::WidthAndHeight));
        match provider {
            Some(query_provider) => WidthQuery::evaluate_provider(&query_provider, self.argument).is_match(),
            None => false,
        }
    }

    fn initialize(&self) {
        self.initialize_container_query()
    }

    fn deinitialize(&self) {
        self.deinitialize_container_query()
    }
}

/// An activator which is active while the height of the container of a
/// visual satisfies a comparison.
pub(crate) struct HeightActivator {
    inner: ContainerQueryActivatorBase,
    this: Weak<HeightActivator>,
    argument: SizeQueryArgument,
}

impl HeightActivator {
    pub fn new(visual: &Visual, argument: SizeQueryArgument, container_name: Option<&str>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            inner: ContainerQueryActivatorBase::new(visual, container_name),
            this: this.clone(),
            argument,
        })
    }
}

impl ContainerQueryActivator for HeightActivator {
    fn container_base(&self) -> &ContainerQueryActivatorBase {
        &self.inner
    }

    fn weak(&self) -> Weak<Self> {
        self.this.clone()
    }
}

impl StyleActivator for HeightActivator {
    fn base(&self) -> &StyleActivatorBase {
        &self.inner.base
    }

    fn evaluate_is_active(&self) -> bool {
        let provider = self.inner.current_query_provider(|sizing| {
            matches!(sizing, ContainerSizing::Height | ContainerSizing::WidthAndHeight)
        });
        match provider {
            Some(query_provider) => HeightQuery::evaluate_provider(&query_provider, self.argument).is_match(),
            None => false,
        }
    }

    fn initialize(&self) {
        self.initialize_container_query()
    }

    fn deinitialize(&self) {
        self.deinitialize_container_query()
    }
}
