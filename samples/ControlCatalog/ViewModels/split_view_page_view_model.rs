//! Port of `ViewModels/SplitViewPageViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_controls::{SplitViewDisplayMode, SplitViewPanePlacement};
use mini_mvvm::ViewModelBase;
use std::cell::Cell;
use std::rc::Rc;

/// The view model of the split view page.
pub struct SplitViewPageViewModel {
    base: ViewModelBase,
    /// The index of the display mode; 3 is `CompactOverlay`.
    display_mode: Cell<i32>,
    /// The index of the placement; 0 is `Left`.
    placement: Cell<i32>,
}

impl PartialEq for SplitViewPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for SplitViewPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl SplitViewPageViewModel {
    pub fn new() -> Rc<SplitViewPageViewModel> {
        Rc::new(Self { base: ViewModelBase::new(), display_mode: Cell::new(3), placement: Cell::new(0) })
    }

    pub fn placement(&self) -> i32 {
        self.placement.get()
    }

    pub fn set_placement(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.placement, value, "Placement");
        self.base.raise_property_changed("PanePlacement");
    }

    pub fn display_mode(&self) -> i32 {
        self.display_mode.get()
    }

    pub fn set_display_mode(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.display_mode, value, "DisplayMode");
        self.base.raise_property_changed("CurrentDisplayMode");
    }

    pub fn pane_placement(&self) -> SplitViewPanePlacement {
        match self.placement.get() {
            0 => SplitViewPanePlacement::Left,
            1 => SplitViewPanePlacement::Right,
            2 => SplitViewPanePlacement::Top,
            3 => SplitViewPanePlacement::Bottom,
            _ => SplitViewPanePlacement::Left,
        }
    }

    pub fn current_display_mode(&self) -> SplitViewDisplayMode {
        match self.display_mode.get() {
            0 => SplitViewDisplayMode::Inline,
            1 => SplitViewDisplayMode::CompactInline,
            2 => SplitViewDisplayMode::Overlay,
            3 => SplitViewDisplayMode::CompactOverlay,
            // Not a defined value of the enumeration.
            _ => SplitViewDisplayMode::CompactOverlay,
        }
    }
}

ferro_markup_type!(class SplitViewPageViewModel {
    this: Rc<SplitViewPageViewModel>,
    handles: [SplitViewPageViewModel, Rc<SplitViewPageViewModel>, Option<Rc<SplitViewPageViewModel>>],
    constructors: [() => SplitViewPageViewModel::new],
    properties: [
        Placement: i32 {
            get: |this: &Rc<SplitViewPageViewModel>| this.placement(),
            set: |this: &Rc<SplitViewPageViewModel>, value: i32| this.set_placement(value)
        },
        DisplayMode: i32 {
            get: |this: &Rc<SplitViewPageViewModel>| this.display_mode(),
            set: |this: &Rc<SplitViewPageViewModel>, value: i32| this.set_display_mode(value)
        },
        PanePlacement: SplitViewPanePlacement { get: |this: &Rc<SplitViewPageViewModel>| this.pane_placement() },
        CurrentDisplayMode: SplitViewDisplayMode {
            get: |this: &Rc<SplitViewPageViewModel>| this.current_display_mode()
        },
    ],
    notify_property_changed: SplitViewPageViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn the_indexes_map_to_the_placement_and_the_display_mode() {
        let view_model = SplitViewPageViewModel::new();
        assert_eq!(SplitViewPanePlacement::Left, view_model.pane_placement());
        assert_eq!(SplitViewDisplayMode::CompactOverlay, view_model.current_display_mode());

        view_model.set_placement(3);
        assert_eq!(SplitViewPanePlacement::Bottom, view_model.pane_placement());
        view_model.set_placement(-1);
        assert_eq!(SplitViewPanePlacement::Left, view_model.pane_placement());

        view_model.set_display_mode(1);
        assert_eq!(SplitViewDisplayMode::CompactInline, view_model.current_display_mode());
        view_model.set_display_mode(7);
        assert_eq!(SplitViewDisplayMode::CompactOverlay, view_model.current_display_mode());
    }

    #[test]
    fn a_setter_always_notifies_for_the_derived_property() {
        let view_model = SplitViewPageViewModel::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        view_model.set_placement(0);
        view_model.set_display_mode(2);
        assert_eq!(vec!["PanePlacement", "DisplayMode", "CurrentDisplayMode"], *seen.borrow());
    }
}
