//! Port of `Automation/Behaviors/AutomationNameBehavior.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};

/// Sets the `AutomationProperties.Name` property on the associated control when attached, and
/// clears it when detached.
#[repr(C)]
pub struct AutomationNameBehavior {
    base: StyledElementBehaviorOfT,
}

ferro_class!(AutomationNameBehavior: StyledElementBehaviorOfT);
ferro_class_info!(AutomationNameBehavior { new: AutomationNameBehavior::new });
ferroui_base::ferro_impl_classes!(AutomationNameBehavior: StyledElementImpl);

ferro_properties! {
    impl AutomationNameBehavior {
        /// Identifies the `AutomationName` property.
        pub fn automation_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<AutomationNameBehavior, _>("AutomationName", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for AutomationNameBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);
            this.update_name();
        }

        fn on_detaching(this: &Self) {
            this.set_automation_properties_name(None);
            Self::parent_on_detaching(this);
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for AutomationNameBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::automation_name_property().as_property() {
                this.update_name();
            }
        }
    }
}

impl AutomationNameBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the automation name. This is a property of the object model.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the accessors of the reference read and write the
    /// `Name` property of the behavior instead of `AutomationName` (markup, which sets the
    /// registered property, was not affected); here they use `AutomationName` (defect corrected).
    pub fn automation_name(&self) -> Option<String> {
        self.get_value(Self::automation_name_property())
    }

    pub fn set_automation_name(&self, value: Option<String>) {
        self.set_value(Self::automation_name_property(), value)
    }

    fn update_name(&self) {
        self.set_automation_properties_name(self.automation_name().as_deref());
    }

    fn set_automation_properties_name(&self, value: Option<&str>) {
        let Some(associated_object) = self.associated_object_as::<Control>() else {
            return;
        };

        AutomationProperties::set_name(&associated_object, value);
    }
}
