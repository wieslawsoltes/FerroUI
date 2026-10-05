use super::{AssemblyDescriptor, IAssemblyDescriptor};
use crate::platform::AssetAssembly;
use crate::utilities::UriExtensions;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Finds the descriptor of an assembly by name.
pub(crate) trait IAssemblyDescriptorResolver {
    /// The descriptor of the assembly with the given name. Fails when no
    /// such assembly is known.
    fn get_assembly(&self, name: &str) -> Result<Rc<dyn IAssemblyDescriptor>, String>;

    fn invalidate_assembly_cache(&self, name: &str);

    fn invalidate_assembly_cache_all(&self);
}

#[derive(Default)]
pub(crate) struct AssemblyDescriptorResolver {
    assembly_name_cache: RefCell<HashMap<String, Rc<dyn IAssemblyDescriptor>>>,
}

impl AssemblyDescriptorResolver {
    pub fn new() -> Self {
        Self::default()
    }

    fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
        let mut text = text.chars().flat_map(char::to_uppercase);
        prefix.chars().flat_map(char::to_uppercase).all(|c| text.next() == Some(c))
    }
}

impl IAssemblyDescriptorResolver for AssemblyDescriptorResolver {
    fn get_assembly(&self, name: &str) -> Result<Rc<dyn IAssemblyDescriptor>, String> {
        if let Some(rv) = self.assembly_name_cache.borrow().get(name) {
            return Ok(rv.clone());
        }

        let loaded_assemblies = super::registered_assembly_names();
        // The shortest registered name that starts with the requested name,
        // ignoring case; the first registered one among equally long names.
        let mut matched: Option<&String> = None;
        for candidate in &loaded_assemblies {
            if Self::starts_with_ignore_case(candidate, name) && matched.is_none_or(|m| candidate.len() < m.len()) {
                matched = Some(candidate);
            }
        }

        let (key, rv): (String, Rc<dyn IAssemblyDescriptor>) = match matched {
            Some(matched) => (name.to_owned(), Rc::new(AssemblyDescriptor::new(&AssetAssembly::new(matched)))),
            None => {
                let name = UriExtensions::unescape_data_string(name);
                if !loaded_assemblies.iter().any(|candidate| *candidate == name) {
                    return Err(format!(
                        "Assembly {name} needs to be referenced and explicitly loaded before loading resources"
                    ));
                }
                let descriptor = Rc::new(AssemblyDescriptor::new(&AssetAssembly::new(&name)));
                (name, descriptor)
            }
        };

        self.assembly_name_cache.borrow_mut().insert(key, rv.clone());
        Ok(rv)
    }

    fn invalidate_assembly_cache(&self, name: &str) {
        self.assembly_name_cache.borrow_mut().remove(name);
    }

    fn invalidate_assembly_cache_all(&self) {
        self.assembly_name_cache.borrow_mut().clear();
    }
}
