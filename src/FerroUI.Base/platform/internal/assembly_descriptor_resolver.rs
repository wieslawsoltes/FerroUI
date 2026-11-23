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

    /// Whether `scope_name`, a registered assembly name, is the assembly `name`: the same name,
    /// or that name with a `.dll` or `.exe` extension, ignoring case. A longer name that merely
    /// starts with `name` (`Lib.Tests` for `Lib`) is another assembly.
    fn is_scope_name_of(scope_name: &str, name: &str) -> bool {
        let mut scope = scope_name.chars().flat_map(char::to_uppercase);
        if !name.chars().flat_map(char::to_uppercase).all(|c| scope.next() == Some(c)) {
            return false;
        }
        let rest: String = scope.collect();
        rest.is_empty() || rest == ".DLL" || rest == ".EXE"
    }
}

impl IAssemblyDescriptorResolver for AssemblyDescriptorResolver {
    fn get_assembly(&self, name: &str) -> Result<Rc<dyn IAssemblyDescriptor>, String> {
        if let Some(rv) = self.assembly_name_cache.borrow().get(name) {
            return Ok(rv.clone());
        }

        let loaded_assemblies = super::registered_assembly_names();
        // The shortest registered name that is the requested name, ignoring case; the first
        // registered one among equally long names.
        let mut matched: Option<&String> = None;
        for candidate in &loaded_assemblies {
            if Self::is_scope_name_of(candidate, name) && matched.is_none_or(|m| candidate.len() < m.len()) {
                matched = Some(candidate);
            }
        }

        // The descriptor is cached under the requested name, also when the
        // assembly is found by its unescaped name.
        let rv: Rc<dyn IAssemblyDescriptor> = match matched {
            Some(matched) => Rc::new(AssemblyDescriptor::new(&AssetAssembly::new(matched))),
            None => {
                let name = UriExtensions::unescape_data_string(name);
                if !loaded_assemblies.iter().any(|candidate| *candidate == name) {
                    return Err(format!(
                        "Assembly {name} needs to be referenced and explicitly loaded before loading resources"
                    ));
                }
                Rc::new(AssemblyDescriptor::new(&AssetAssembly::new(&name)))
            }
        };

        self.assembly_name_cache.borrow_mut().insert(name.to_owned(), rv.clone());
        Ok(rv)
    }

    fn invalidate_assembly_cache(&self, name: &str) {
        self.assembly_name_cache.borrow_mut().remove(name);
    }

    fn invalidate_assembly_cache_all(&self) {
        self.assembly_name_cache.borrow_mut().clear();
    }
}
