use super::{AndQueryActivator, IStyleActivator};
use std::rc::Rc;

/// Builds an [`AndQueryActivator`], avoiding the aggregate when there is only a
/// single input.
#[derive(Default)]
pub(crate) struct AndQueryActivatorBuilder {
    single: Option<Rc<dyn IStyleActivator>>,
    multiple: Option<Rc<AndQueryActivator>>,
}

impl AndQueryActivatorBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(&self) -> usize {
        match &self.multiple {
            Some(multiple) => multiple.count(),
            None => usize::from(self.single.is_some()),
        }
    }

    pub fn add(&mut self, activator: Option<Rc<dyn IStyleActivator>>) {
        let Some(activator) = activator else { return };

        if self.single.is_none() && self.multiple.is_none() {
            self.single = Some(activator);
        } else {
            if self.multiple.is_none() {
                let multiple = AndQueryActivator::new();
                multiple.add(self.single.take().expect("single activator is set"));
                self.multiple = Some(multiple);
            }
            self.multiple.as_ref().expect("aggregate is set").add(activator);
        }
    }

    pub fn get(self) -> Rc<dyn IStyleActivator> {
        match (self.single, self.multiple) {
            (Some(single), _) => single,
            (None, Some(multiple)) => multiple,
            (None, None) => panic!("no activators were added"),
        }
    }
}
