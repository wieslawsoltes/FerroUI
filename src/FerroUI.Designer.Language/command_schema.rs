use crate::{PropertyInfo, Schema};

impl PropertyInfo {
    /// The registered ICommand contract, shared by XAML and mapped Rust.
    pub fn is_command(&self) -> bool {
        if self.event {
            return false;
        }
        if let Some(model) = &self.model_type {
            return model.name == "ICommand" && model.namespace == "System.Windows.Input";
        }
        let kind = crate::project_schema::unwrapped_type(&self.type_name);
        let kind = kind
            .strip_prefix("dyn ")
            .unwrap_or(kind)
            .trim_start_matches("::");
        matches!(kind, "ICommand" | "System.Windows.Input.ICommand")
            || kind.starts_with("ferroui_base::") && kind.ends_with("::ICommand")
    }

    pub fn is_writable_command(&self) -> bool {
        !self.read_only && !self.attached && self.is_command()
    }
}

impl Schema {
    pub fn command_parameter(
        &self,
        owner: &str,
        namespace: Option<&str>,
        command: &str,
    ) -> Option<PropertyInfo> {
        self.property(owner, namespace, &format!("{command}Parameter"))
            .filter(|property| !property.read_only && !property.attached && !property.event)
    }
}
