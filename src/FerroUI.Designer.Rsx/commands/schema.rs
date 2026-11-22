use crate::catalog;
use ferroui_designer_language::{PropertyInfo, Schema, FRAMEWORK_NAMESPACE};
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommandMember {
    pub(crate) property: String,
    pub(crate) setter: String,
    pub(crate) type_name: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommandSlot {
    pub(crate) binding: CommandMember,
    pub(crate) parameter: Option<CommandMember>,
}

impl CommandSlot {
    pub(crate) fn members(&self) -> impl Iterator<Item = &CommandMember> {
        std::iter::once(&self.binding).chain(self.parameter.as_ref())
    }
}

pub(crate) fn slots(
    schema: &Schema,
    class: &str,
    namespace: Option<&str>,
    setter: impl Fn(&str) -> Option<String>,
) -> Vec<CommandSlot> {
    let member = |info: PropertyInfo| {
        Some(CommandMember {
            setter: setter(&info.name)?,
            property: info.name,
            type_name: info.type_name,
        })
    };
    schema
        .properties(class, namespace)
        .into_values()
        .filter(PropertyInfo::is_writable_command)
        .filter_map(|info| {
            let binding = member(info)?;
            let parameter = schema
                .command_parameter(class, namespace, &binding.property)
                .and_then(member);
            Some(CommandSlot { binding, parameter })
        })
        .collect()
}

pub(crate) fn builtin(class: &str) -> &'static [CommandSlot] {
    static COMMANDS: OnceLock<BTreeMap<String, Vec<CommandSlot>>> = OnceLock::new();
    let catalog = COMMANDS.get_or_init(|| {
        let schema = catalog::schema();
        schema
            .types
            .values()
            .filter(|ty| {
                ty.namespace == FRAMEWORK_NAMESPACE && catalog::children(&ty.name).is_some()
            })
            .filter_map(|ty| {
                let commands = slots(schema, &ty.name, None, |name| Some(catalog::setter(name)));
                (!commands.is_empty()).then(|| (ty.name.clone(), commands))
            })
            .collect()
    });
    catalog.get(class).map_or(&[], Vec::as_slice)
}
