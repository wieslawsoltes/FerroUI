use super::AutomationProperty;

/// Contains values used as automation property identifiers by UI Automation providers.
pub struct AutomationElementIdentifiers;

impl AutomationElementIdentifiers {
    /// Identifies the bounding rectangle automation property. The bounding rectangle property
    /// value is returned by the `AutomationPeer::get_bounding_rectangle` method.
    pub fn bounding_rectangle_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the class name automation property. The class name property value is returned
    /// by the `AutomationPeer::get_class_name` method.
    pub fn class_name_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the name automation property. The class name property value is returned
    /// by the `AutomationPeer::get_name` method.
    pub fn name_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the help text automation property. The class name property value is returned
    /// by the `AutomationPeer::get_help_text` method.
    pub fn help_text_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the item status automation property. The item status property value is returned
    /// by the `AutomationPeer::get_item_status` method.
    pub fn item_status_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the landmark type automation property. The class name property value is returned
    /// by the `AutomationPeer::get_landmark_type` method.
    pub fn landmark_type_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the heading level automation property. The class name property value is returned
    /// by the `AutomationPeer::get_heading_level` method.
    pub fn heading_level_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the automation id automation property. The automation id property value is returned
    /// by the `AutomationPeer::get_automation_id` method.
    pub fn automation_id_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }
}
