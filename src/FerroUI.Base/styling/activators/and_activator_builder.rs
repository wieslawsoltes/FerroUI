use super::{AndActivator, IStyleActivator};
use std::rc::Rc;

/// Builds an [`AndActivator`], avoiding the aggregate when there is only a
/// single input.
#[derive(Default)]
pub(crate) struct AndActivatorBuilder {
    single: Option<Rc<dyn IStyleActivator>>,
    multiple: Option<Rc<AndActivator>>,
}

impl AndActivatorBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, activator: Option<Rc<dyn IStyleActivator>>) {
        let Some(activator) = activator else { return };

        if self.single.is_none() && self.multiple.is_none() {
            self.single = Some(activator);
        } else {
            if self.multiple.is_none() {
                let multiple = AndActivator::new();
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
