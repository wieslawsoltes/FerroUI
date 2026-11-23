//! The answers to conditional XAML namespaces (`xmlns:new="uri?IsTypePresent(A.B)"`).
//!
//! The built-in conditions ask the type system, so the run-time loader and the build-time
//! emitter read the same document the same way: a false one leaves its nodes out before any of
//! their types resolve. A custom condition (`uri?prefix:Name(argument)`) is registered by the
//! application and only has an answer in the running process: the loader evaluates it, and the
//! emitter writes a test of it around the nodes.

use std::rc::Rc;

use ferroui_base::metadata::FERRO_XML_NAMESPACE;
use xamlx::parsers::{XamlCondition, XamlConditionOutcome, XamlConditionResolver};
use xamlx::transform::TransformerConfiguration;
use xamlx::type_system::IXamlType;

/// The API contract of the framework, versioned by the release.
const FRAMEWORK_CONTRACT: &str = "FerroUI.ApiContract";

/// How a compiler answers custom conditions.
#[derive(Clone, Copy, Default)]
pub enum CustomConditions {
    /// Keep the nodes with a test around them, evaluated when the document loads.
    #[default]
    AtLoad,
    /// Evaluate the condition now, with the function of the running application.
    Evaluate(fn(&str, &str) -> bool),
}

/// Answers the conditions of a document against the types of `configuration`.
pub struct FerroXamlConditionResolver<'a> {
    pub configuration: &'a TransformerConfiguration,
    pub custom: CustomConditions,
}

impl FerroXamlConditionResolver<'_> {
    fn find_type(&self, name: &str) -> Option<Rc<dyn IXamlType>> {
        if let Some(found) = self.configuration.type_system.find_type(name) {
            return Some(found);
        }
        let short = name.rsplit('.').next()?;
        let mappings = self.configuration.xmlns_mappings.namespaces.get(FERRO_XML_NAMESPACE)?;
        mappings.iter().find_map(|(assembly, namespace)| assembly.find_type(&format!("{namespace}.{short}")))
    }

    fn has_property(&self, type_name: &str, property: &str) -> bool {
        self.find_type(type_name)
            .is_some_and(|found| found.get_all_properties().iter().any(|candidate| candidate.name() == property))
    }

    fn built_in(&self, condition: &XamlCondition) -> Option<Result<bool, String>> {
        let arguments = condition.arguments();
        let expect = |count: std::ops::RangeInclusive<usize>| {
            if count.contains(&arguments.len()) {
                Ok(())
            } else {
                Err(format!("{} does not take {} arguments", condition.name, arguments.len()))
            }
        };
        let answer = match condition.name.as_str() {
            "IsTypePresent" => expect(1..=1).map(|()| self.find_type(arguments[0]).is_some()),
            "IsTypeNotPresent" => expect(1..=1).map(|()| self.find_type(arguments[0]).is_none()),
            "IsPropertyPresent" => expect(2..=2).map(|()| self.has_property(arguments[0], arguments[1])),
            "IsPropertyNotPresent" => expect(2..=2).map(|()| !self.has_property(arguments[0], arguments[1])),
            "IsApiContractPresent" => expect(2..=3).and_then(|()| contract_present(&arguments)),
            "IsApiContractNotPresent" => {
                expect(2..=3).and_then(|()| contract_present(&arguments).map(|present| !present))
            }
            _ => return None,
        };
        Some(answer)
    }
}

/// Whether the framework provides the contract: only its own, up to the version of this release.
fn contract_present(arguments: &[&str]) -> Result<bool, String> {
    let number = |text: &str| text.parse::<u32>().map_err(|_| format!("`{text}` is not a contract version"));
    let major = number(arguments[1])?;
    let minor = arguments.get(2).map_or(Ok(0), |text| number(text))?;
    let current = (number(env!("CARGO_PKG_VERSION_MAJOR"))?, number(env!("CARGO_PKG_VERSION_MINOR"))?);
    Ok(arguments[0] == FRAMEWORK_CONTRACT && (major, minor) <= current)
}

impl XamlConditionResolver for FerroXamlConditionResolver<'_> {
    fn resolve(&self, condition: &XamlCondition) -> Result<XamlConditionOutcome, String> {
        let outcome = |value: bool| match value {
            true => XamlConditionOutcome::Included,
            false => XamlConditionOutcome::Excluded,
        };
        if condition.prefix.is_none() {
            return match self.built_in(condition) {
                Some(answer) => answer.map(outcome),
                None => Err(format!(
                    "`{}` is not a built-in condition; a custom condition is written `prefix:{}(..)`",
                    condition.name, condition.name
                )),
            };
        }
        Ok(match self.custom {
            CustomConditions::AtLoad => XamlConditionOutcome::AtLoad,
            CustomConditions::Evaluate(evaluate) => outcome(evaluate(&condition.name, &condition.argument)),
        })
    }
}
