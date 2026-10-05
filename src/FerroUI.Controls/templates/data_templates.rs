use super::IDataTemplate;
use ferroui_base::collections::{FerroList, ResetBehavior};
use std::ops::Deref;
use std::rc::Rc;

/// A collection of [`IDataTemplate`]s.
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity.
#[derive(Clone, PartialEq, Debug)]
pub struct DataTemplates {
    list: FerroList<Rc<dyn IDataTemplate>>,
}

impl Default for DataTemplates {
    fn default() -> Self {
        Self::new()
    }
}

impl DataTemplates {
    /// Creates an empty collection.
    pub fn new() -> Self {
        let list = FerroList::new();
        list.set_reset_behavior(ResetBehavior::Remove);
        list.set_validator(Some(Rc::new(|item: &Rc<dyn IDataTemplate>| {
            let valid = match item.as_typed_data_template() {
                Some(typed) => typed.data_type().is_some(),
                None => true,
            };

            if !valid {
                panic!(
                    "DataTemplate inside of DataTemplates must have a DataType set. Set DataType property or use \
                     ItemTemplate with single template instead."
                );
            }
        })));
        Self { list }
    }
}

impl Deref for DataTemplates {
    type Target = FerroList<Rc<dyn IDataTemplate>>;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}
