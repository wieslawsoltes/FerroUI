//! Port of `Dialog/ShowDialogAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::{TopLevel, Window};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Shows a [`Window`] as a dialog: owned by [`owner`](Self::owner), or else by the window of the
/// sender, or as a plain window when there is no owner.
#[repr(C)]
pub struct ShowDialogAction {
    base: StyledElementAction,
}

ferro_class!(ShowDialogAction: StyledElementAction);
ferro_class_info!(ShowDialogAction {
    new: ShowDialogAction::new,
    markup: {
        property_attributes: [
            Dialog: [ResolveByName],
            Owner: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ShowDialogAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ShowDialogAction {
        /// Identifies the `Dialog` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the windows it names, so
        /// the values of `Dialog` and `Owner` are element references.
        pub fn dialog_property() -> StyledProperty<Option<ElementRef<Window>>> {
            ValueTypes::register_element_ref::<Window>();
            FerroProperty::register::<ShowDialogAction, _>("Dialog", None)
        }

        /// Identifies the `Owner` property.
        pub fn owner_property() -> StyledProperty<Option<ElementRef<Window>>> {
            ValueTypes::register_element_ref::<Window>();
            FerroProperty::register::<ShowDialogAction, _>("Owner", None)
        }
    }
}

impl StyledElementActionImpl for ShowDialogAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let Some(dialog) = this.dialog() else {
            return Some(Rc::new(false));
        };

        let owner = this.owner().or_else(|| {
            let visual = sender.and_then(object_of).and_then(|sender| sender.cast::<Visual>());
            visual
                .and_then(|visual| TopLevel::get_top_level(Some(&*visual)))
                .and_then(|top_level| top_level.cast::<Window>())
        });
        match owner {
            // The task of the result of the dialog is not observed, as the reference does not
            // await it.
            Some(owner) => drop(dialog.show_dialog(&owner)),
            None => dialog.show(),
        }

        Some(Rc::new(true))
    }
}

impl ShowDialogAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the dialog window instance. This is a property of the object model. Markup
    /// resolves a name given for it to the named element.
    pub fn dialog(&self) -> Option<Ref<Window>> {
        ElementRef::resolve(&self.get_value(Self::dialog_property()))
    }

    pub fn set_dialog(&self, value: impl Into<Nullable<Window>>) {
        self.set_value(Self::dialog_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the owner window for the dialog. This is a property of the object model.
    /// Markup resolves a name given for it to the named element.
    pub fn owner(&self) -> Option<Ref<Window>> {
        ElementRef::resolve(&self.get_value(Self::owner_property()))
    }

    pub fn set_owner(&self, value: impl Into<Nullable<Window>>) {
        self.set_value(Self::owner_property(), ElementRef::from_nullable(value.into().0))
    }
}
