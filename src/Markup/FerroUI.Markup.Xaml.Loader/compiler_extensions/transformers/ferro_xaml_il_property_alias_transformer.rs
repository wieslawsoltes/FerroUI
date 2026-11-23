//! Markup aliases of properties (docs/porting/xaml.md, "Property aliases").
//!
//! Not from upstream. A crate can give a property of another crate's class a WinUI name without
//! adding anything to that class: its `MarkupAssembly` states
//! `(MarkupAssembly::PROPERTY_ALIAS, "FerroUI.Visual.OpacityTransition=FerroUI.Controls.ImplicitTransitions.Opacity")`
//! and `<Rectangle.OpacityTransition>` then sets the attached property. The table lives in the
//! metadata of the declaring crate and is read only by the markup compiler, so an application
//! that does not depend on that crate has no entry, registration or code for it.
//!
//! - A real property, event or attached property of the name always wins.
//! - The class of an alias and its derived classes have it.
//! - Two declarations of one alias are an error where the name is used.

use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

use ferroui_base::metadata::MarkupAssembly;
use xamlx::ast::{IXamlAstNode, XamlAstClrTypeReference, XamlAstNamePropertyReference, XamlAstNodeExtensions};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer, TransformerConfiguration};
use xamlx::type_system::{IXamlType, XamlValue};

/// What an alias stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PropertyAliasTarget {
    pub(crate) owner: String,
    pub(crate) name: String,
    pub(crate) assembly: String,
}

/// The aliases of the assemblies of a compilation, by `Owner.Name`.
#[derive(Default)]
pub(crate) struct PropertyAliases {
    table: OnceCell<HashMap<String, Vec<PropertyAliasTarget>>>,
}

/// Adds the alias `value` declared by `assembly`. A value that is not `Owner.Name=Owner.Name` is
/// not an alias.
pub(crate) fn add_alias(table: &mut HashMap<String, Vec<PropertyAliasTarget>>, assembly: &str, value: &str) {
    let Some((alias, target)) = value.split_once('=') else { return };
    let Some((owner, name)) = target.trim().rsplit_once('.') else { return };
    let alias = alias.trim();
    if !alias.contains('.') || owner.is_empty() || name.is_empty() {
        return;
    }
    table.entry(alias.to_owned()).or_default().push(PropertyAliasTarget {
        owner: owner.to_owned(),
        name: name.to_owned(),
        assembly: assembly.to_owned(),
    });
}

impl PropertyAliases {
    fn table(&self, configuration: &TransformerConfiguration) -> &HashMap<String, Vec<PropertyAliasTarget>> {
        self.table.get_or_init(|| {
            let mut table = HashMap::new();
            for assembly in configuration.type_system.assemblies() {
                for attribute in assembly.custom_attributes() {
                    if let [XamlValue::String(key), XamlValue::String(value)] = &attribute.parameters()[..] {
                        if key == MarkupAssembly::PROPERTY_ALIAS {
                            add_alias(&mut table, &assembly.name(), value);
                        }
                    }
                }
            }
            table
        })
    }
}

/// The first alias of `name` on `type_` or a base class. `Err` for one declared more than once.
pub(crate) fn find_alias(
    table: &HashMap<String, Vec<PropertyAliasTarget>>,
    type_names: impl Iterator<Item = String>,
    name: &str,
) -> Result<Option<PropertyAliasTarget>, String> {
    for type_name in type_names {
        let key = format!("{type_name}.{name}");
        match table.get(&key).map(Vec::as_slice) {
            None | Some([]) => {}
            Some([target]) => return Ok(Some(target.clone())),
            Some(targets) => {
                let assemblies: Vec<&str> = targets.iter().map(|target| target.assembly.as_str()).collect();
                return Err(format!("The property alias {key} is declared more than once, by {}.", assemblies.join(", ")));
            }
        }
    }
    Ok(None)
}

/// Whether `type_` has a member of the name, which an alias never replaces.
fn has_member(type_: &Rc<dyn IXamlType>, name: &str) -> bool {
    let (setter, getter) = (format!("Set{name}"), format!("Get{name}"));
    type_.get_all_properties().iter().any(|property| property.name() == name)
        || type_.get_all_events().iter().any(|event| event.name() == name)
        || type_.methods().iter().any(|method| method.is_static() && (method.name() == setter || method.name() == getter))
}

fn with_bases(type_: &Rc<dyn IXamlType>) -> impl Iterator<Item = String> {
    let mut current = Some(type_.clone());
    std::iter::from_fn(move || {
        let found = current.take()?;
        current = found.base_type();
        Some(found.full_name())
    })
}

/// Rewrites a reference to an alias into a reference to its attached property.
pub struct FerroXamlIlPropertyAliasTransformer;

impl IXamlAstTransformer for FerroXamlIlPropertyAliasTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(prop) = node.cast::<XamlAstNamePropertyReference>() else {
            return Ok(node);
        };
        let Some(declaring) = prop.declaring_type.borrow().clone().cast::<XamlAstClrTypeReference>() else {
            return Ok(node);
        };
        let name = prop.name();
        if has_member(&declaring.type_, &name) {
            return Ok(node);
        }
        let configuration = context.configuration();
        let aliases = configuration.get_or_create_extra::<PropertyAliases>();
        let found = match find_alias(aliases.table(configuration), with_bases(&declaring.type_), &name) {
            Ok(found) => found,
            Err(message) => return context.report_transform_error(&message, Some(&*node), node.clone()),
        };
        let Some(target) = found else { return Ok(node) };
        let Some(owner) = configuration.type_system.find_type(&target.owner) else {
            let message = format!("The property alias {name} names the type {}, which is not found.", target.owner);
            return context.report_transform_error(&message, Some(&*node), node.clone());
        };
        *prop.declaring_type.borrow_mut() = XamlAstClrTypeReference::new(&*prop, owner, false);
        *prop.name.borrow_mut() = target.name;
        Ok(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(entries: &[(&str, &str)]) -> HashMap<String, Vec<PropertyAliasTarget>> {
        let mut table = HashMap::new();
        for (assembly, value) in entries {
            add_alias(&mut table, assembly, value);
        }
        table
    }

    fn names(names: &[&str]) -> std::vec::IntoIter<String> {
        names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>().into_iter()
    }

    #[test]
    fn an_alias_is_found_on_the_class_and_through_its_bases() {
        let table = table(&[("A", "N.Visual.Fade=M.Motion.Opacity")]);
        for chain in [&["N.Visual"][..], &["N.Rectangle", "N.Shape", "N.Visual"][..]] {
            let target = find_alias(&table, names(chain), "Fade").unwrap().unwrap();
            assert_eq!((target.owner.as_str(), target.name.as_str()), ("M.Motion", "Opacity"));
        }
        assert_eq!(find_alias(&table, names(&["N.Other"]), "Fade"), Ok(None));
        assert_eq!(find_alias(&table, names(&["N.Visual"]), "Other"), Ok(None));
    }

    #[test]
    fn the_nearest_class_decides() {
        let table = table(&[("A", "N.Shape.Fade=M.Near.Value"), ("B", "N.Visual.Fade=M.Far.Value")]);
        let target = find_alias(&table, names(&["N.Rectangle", "N.Shape", "N.Visual"]), "Fade").unwrap().unwrap();
        assert_eq!(target.owner, "M.Near");
    }

    #[test]
    fn a_duplicate_alias_is_an_error_that_names_both_crates() {
        let table = table(&[("A", "N.Visual.Fade=M.One.Value"), ("B", "N.Visual.Fade=M.Two.Value")]);
        let error = find_alias(&table, names(&["N.Visual"]), "Fade").unwrap_err();
        assert!(error.contains("N.Visual.Fade") && error.contains("A, B"), "{error}");
    }

    #[test]
    fn a_malformed_entry_is_not_an_alias() {
        for value in ["", "N.Visual.Fade", "Fade=M.Motion.Value", "N.Visual.Fade=Value", "N.Visual.Fade=.Value"] {
            assert!(table(&[("A", value)]).is_empty(), "{value}");
        }
    }
}
