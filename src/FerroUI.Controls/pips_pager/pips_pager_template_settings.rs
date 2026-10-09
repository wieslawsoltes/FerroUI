use ferroui_base::collections::FerroList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, DirectProperty, DirectPropertyMetadata, FerroObject,
    FerroObjectImpl, FerroProperty, Ref,
};

/// Provides calculated values for use with the control theme or template of
/// the [`PipsPager`](crate::PipsPager).
#[repr(C)]
pub struct PipsPagerTemplateSettings {
    base: FerroObject,
    pips: FerroList<i32>,
}

ferro_class!(PipsPagerTemplateSettings: FerroObject);
// The constructor is internal upstream: no default constructor is declared.
ferro_class_info!(PipsPagerTemplateSettings {});

ferroui_base::ferro_impl_classes!(PipsPagerTemplateSettings: FerroObjectImpl);

ferro_properties! {
    impl PipsPagerTemplateSettings {
        /// Defines the `Pips` property.
        pub fn pips_property() -> DirectProperty<PipsPagerTemplateSettings, FerroList<i32>> {
            FerroProperty::register_direct_with::<PipsPagerTemplateSettings, _>(
                "Pips",
                |o| o.pips(),
                None,
                DirectPropertyMetadata::new(None),
            )
        }
    }
}

impl PipsPagerTemplateSettings {
    /// Creates the settings. Only the pips pager creates them.
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct(), pips: FerroList::new() })
    }

    /// Gets the collection of pips indices.
    pub fn pips(&self) -> FerroList<i32> {
        self.pips.clone()
    }
}
