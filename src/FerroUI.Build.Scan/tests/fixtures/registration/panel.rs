use ferroui_base::collections::FerroList;
use ferroui_base::{
    ferro_class, ferro_markup_type, ferro_properties, ferro_property, AttachedProperty, DirectProperty, FerroObject, FerroProperty, Ref,
    StyledProperty,
};
use std::rc::Rc;

#[repr(C)]
pub struct Panel {
    base: FerroObject,
    spacing: f64,
    gap: f64,
}

ferro_class!(Panel: FerroObject);

/// The list of panels, by another name.
pub type PanelList = FerroList<Ref<Panel>>;

/// An alias of an alias.
pub type Panels = Option<PanelList>;

/// An alias whose type the scanner does not resolve: it stays a name.
pub type Lost = Vec<Missing>;

/// An alias under a condition of the build: it stays a name.
#[cfg(feature = "wide")]
pub type Width = f64;

/// An alias with a parameter: it stays a name.
pub type Pair<T> = (T, T);

/// Declares the accessor of a direct property among the members of `Panel`: a macro of the
/// crate that the scanner expands where it is invoked.
macro_rules! cell_property {
    ($(#[$meta:meta])* $accessor:ident, $name:literal, $field:ident) => {
        ferro_property!(for Panel;
            $(#[$meta])*
            pub fn $accessor() -> DirectProperty<Panel, f64> {
                FerroProperty::register_direct::<Panel, _>($name, |panel| panel.$field, None, 0.0)
            }
        );

        pub fn $field(&self) -> f64 {
            self.$field
        }
    };
}

ferro_properties! {
    impl Panel, also [Panel::spacing_property, Panel::gap_property, Marker::mark_property] {
        pub fn items_property() -> StyledProperty<Panels> {
            FerroProperty::register::<Panel, _>("Items", None)
        }
    }
}

impl Panel {
    cell_property!(
        /// Defines the `Spacing` property.
        spacing_property, "Spacing", spacing
    );

    cell_property!(gap_property, "Gap", gap);
}

/// A type without metadata that has the accessor of a property of `Panel`.
pub struct Marker;

impl Marker {
    ferro_property!(for Panel;
        pub fn mark_property() -> StyledProperty<bool> {
            FerroProperty::register::<Panel, _>("Mark", false)
        }
    );
}

#[repr(C)]
pub struct Slot {
    base: Panel,
}

ferro_class!(Slot: Panel);

ferro_properties! {
    impl Slot {
        /// An owner added through the function of `Marker`.
        pub fn mark_property() -> StyledProperty<bool> {
            Marker::mark_property().add_owner::<Slot>()
        }

        /// A property of `Panel` that the properties of `Slot` register.
        fn saved_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<Panel, Panel, _>("Saved", 0)
        }
    }
}

/// A class the list of registered classes leaves out.
#[repr(C)]
pub struct Hidden {
    base: Panel,
}

ferro_class!(Hidden: Panel);

/// A class in the list whose base is not.
#[repr(C)]
pub struct Deep {
    base: Hidden,
}

ferro_class!(Deep: Hidden);

pub trait IPanel {}

ferro_markup_type!(interface dyn IPanel as "IPanel" {
    handles: [Rc<dyn IPanel>],
});

/// The carrier of the metadata of the list of panels.
pub struct PanelListOf;

ferro_markup_type!(class PanelListOf as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<Panel>>],
    generic: "FerroList`1" [Ref<Panel>],
});

/// A collection that is the list it derives from: the crate registers the cast.
pub struct PanelCollection;

ferro_markup_type!(class PanelCollection {
    handles: [PanelCollection],
    base: FerroList<Ref<Panel>>,
    properties: [
        Owner: Option<Ref<Hidden>> { get: PanelCollection::owner },
        Pairs: std::collections::HashMap<
            String,
            Ref<Panel>,
        > { get: PanelCollection::pairs },
    ],
});

/// A collection that declares the list as its base and registers no cast to it.
pub struct PanelStack;

ferro_markup_type!(class PanelStack {
    handles: [PanelStack],
    base: PanelList,
});

/// A value another crate would hold a panel contract in.
pub struct Wrapper;
