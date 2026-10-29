//! Port of `ViewModels/IconViewModel.cs` of the reference sample.

use super::ViewModelBase;
use ferroui_base::ferro_markup_type;
use ferroui_base::media::Geometry;
use ferroui_base::Ref;
use ferroui_reactiveui::reactive_object;
use std::rc::Rc;

/// The view model of the icon page: the two geometries of its icon.
pub struct IconViewModel {
    base: ViewModelBase,
    plus_icon: Ref<Geometry>,
    minus_icon: Ref<Geometry>,
}
reactive_object!(IconViewModel, base);

impl PartialEq for IconViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl IconViewModel {
    /// `IconViewModel()`.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            plus_icon: Geometry::parse("M8 3v5H3v2h5v5h2V10h5V8h-5V3z")
                .expect("the plus icon is a valid geometry"),
            minus_icon: Geometry::parse("M3 7v2h10V7z").expect("the minus icon is a valid geometry"),
        })
    }

    /// `PlusIcon`.
    pub fn plus_icon(&self) -> Ref<Geometry> {
        self.plus_icon.clone()
    }

    /// `MinusIcon`.
    pub fn minus_icon(&self) -> Ref<Geometry> {
        self.minus_icon.clone()
    }
}

type Vm = Rc<IconViewModel>;

ferro_markup_type!(class IconViewModel {
    this: Rc<IconViewModel>,
    handles: [IconViewModel, Rc<IconViewModel>, Option<Rc<IconViewModel>>],
    constructors: [() => IconViewModel::new],
    properties: [
        PlusIcon: Ref<Geometry> { get: |this: &Vm| this.plus_icon() },
        MinusIcon: Ref<Geometry> { get: |this: &Vm| this.minus_icon() },
    ],
    notify_property_changed: IconViewModel,
});
