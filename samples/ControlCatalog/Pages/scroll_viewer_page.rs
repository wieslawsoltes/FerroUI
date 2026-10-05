//! Port of `Pages/ScrollViewerPage.xaml.cs`: the class of the document
//! `Pages/ScrollViewerPage.xaml`, and its view model.

use crate::markup::xaml_class;
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::{ScrollBarVisibility, SnapPointsAlignment, SnapPointsType, TemplatedControlImpl};
use ferroui_controls::{ControlImpl, ItemsSource, MultiPageImpl, PageImpl, SelectingMultiPageImpl, TabbedPage};
use mini_mvvm::ViewModelBase;
use std::cell::Cell;
use std::rc::Rc;

/// The view model of the scroll viewer page.
pub struct ScrollViewerPageViewModel {
    base: ViewModelBase,
    allow_auto_hide: Cell<bool>,
    enable_inertia: Cell<bool>,
    horizontal_scroll_visibility: Cell<ScrollBarVisibility>,
    vertical_scroll_visibility: Cell<ScrollBarVisibility>,
    snap_points_type: Cell<SnapPointsType>,
    snap_points_alignment: Cell<SnapPointsAlignment>,
    are_snap_points_regular: Cell<bool>,
    available_visibility: Rc<BindableList<ScrollBarVisibility>>,
    available_snap_points_type: Rc<BindableList<SnapPointsType>>,
    available_snap_points_alignment: Rc<BindableList<SnapPointsAlignment>>,
}

impl PartialEq for ScrollViewerPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ScrollViewerPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ScrollViewerPageViewModel {
    pub fn new() -> Rc<ScrollViewerPageViewModel> {
        let this = Rc::new(Self {
            base: ViewModelBase::new(),
            allow_auto_hide: Cell::new(false),
            enable_inertia: Cell::new(false),
            // The default values of the enumerations (their members with the value zero).
            horizontal_scroll_visibility: Cell::new(ScrollBarVisibility::Disabled),
            vertical_scroll_visibility: Cell::new(ScrollBarVisibility::Disabled),
            snap_points_type: Cell::new(SnapPointsType::None),
            snap_points_alignment: Cell::new(SnapPointsAlignment::Near),
            are_snap_points_regular: Cell::new(false),
            available_visibility: BindableList::new([
                ScrollBarVisibility::Auto,
                ScrollBarVisibility::Visible,
                ScrollBarVisibility::Hidden,
                ScrollBarVisibility::Disabled,
            ]),
            available_snap_points_type: BindableList::new([
                SnapPointsType::None,
                SnapPointsType::Mandatory,
                SnapPointsType::MandatorySingle,
            ]),
            available_snap_points_alignment: BindableList::new([
                SnapPointsAlignment::Near,
                SnapPointsAlignment::Center,
                SnapPointsAlignment::Far,
            ]),
        });

        this.set_horizontal_scroll_visibility(ScrollBarVisibility::Auto);
        this.set_vertical_scroll_visibility(ScrollBarVisibility::Auto);
        this.set_allow_auto_hide(true);
        this.set_enable_inertia(true);
        this
    }

    pub fn allow_auto_hide(&self) -> bool {
        self.allow_auto_hide.get()
    }

    pub fn set_allow_auto_hide(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.allow_auto_hide, value, "AllowAutoHide");
    }

    pub fn enable_inertia(&self) -> bool {
        self.enable_inertia.get()
    }

    pub fn set_enable_inertia(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.enable_inertia, value, "EnableInertia");
    }

    pub fn horizontal_scroll_visibility(&self) -> ScrollBarVisibility {
        self.horizontal_scroll_visibility.get()
    }

    pub fn set_horizontal_scroll_visibility(&self, value: ScrollBarVisibility) {
        self.base.raise_and_set_if_changed_cell(&self.horizontal_scroll_visibility, value, "HorizontalScrollVisibility");
    }

    pub fn vertical_scroll_visibility(&self) -> ScrollBarVisibility {
        self.vertical_scroll_visibility.get()
    }

    pub fn set_vertical_scroll_visibility(&self, value: ScrollBarVisibility) {
        self.base.raise_and_set_if_changed_cell(&self.vertical_scroll_visibility, value, "VerticalScrollVisibility");
    }

    pub fn available_visibility(&self) -> Rc<BindableList<ScrollBarVisibility>> {
        self.available_visibility.clone()
    }

    pub fn are_snap_points_regular(&self) -> bool {
        self.are_snap_points_regular.get()
    }

    pub fn set_are_snap_points_regular(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.are_snap_points_regular, value, "AreSnapPointsRegular");
    }

    pub fn snap_points_type(&self) -> SnapPointsType {
        self.snap_points_type.get()
    }

    pub fn set_snap_points_type(&self, value: SnapPointsType) {
        self.base.raise_and_set_if_changed_cell(&self.snap_points_type, value, "SnapPointsType");
    }

    pub fn snap_points_alignment(&self) -> SnapPointsAlignment {
        self.snap_points_alignment.get()
    }

    pub fn set_snap_points_alignment(&self, value: SnapPointsAlignment) {
        self.base.raise_and_set_if_changed_cell(&self.snap_points_alignment, value, "SnapPointsAlignment");
    }

    pub fn available_snap_points_type(&self) -> Rc<BindableList<SnapPointsType>> {
        self.available_snap_points_type.clone()
    }

    pub fn available_snap_points_alignment(&self) -> Rc<BindableList<SnapPointsAlignment>> {
        self.available_snap_points_alignment.clone()
    }
}

ferro_markup_type!(class ScrollViewerPageViewModel {
    this: Rc<ScrollViewerPageViewModel>,
    handles: [ScrollViewerPageViewModel, Rc<ScrollViewerPageViewModel>, Option<Rc<ScrollViewerPageViewModel>>],
    constructors: [() => ScrollViewerPageViewModel::new],
    properties: [
        AllowAutoHide: bool {
            get: |this: &Rc<ScrollViewerPageViewModel>| this.allow_auto_hide(),
            set: |this: &Rc<ScrollViewerPageViewModel>, value: bool| this.set_allow_auto_hide(value)
        },
        EnableInertia: bool {
            get: |this: &Rc<ScrollViewerPageViewModel>| this.enable_inertia(),
            set: |this: &Rc<ScrollViewerPageViewModel>, value: bool| this.set_enable_inertia(value)
        },
        HorizontalScrollVisibility: ScrollBarVisibility {
            get: |this: &Rc<ScrollViewerPageViewModel>| this.horizontal_scroll_visibility(),
            set: |this: &Rc<ScrollViewerPageViewModel>, value: ScrollBarVisibility| {
                this.set_horizontal_scroll_visibility(value)
            }
        },
        VerticalScrollVisibility: ScrollBarVisibility {
            get: |this: &Rc<ScrollViewerPageViewModel>| this.vertical_scroll_visibility(),
            set: |this: &Rc<ScrollViewerPageViewModel>, value: ScrollBarVisibility| {
                this.set_vertical_scroll_visibility(value)
            }
        },
        // Lists a binding delivers to an items source property.
        AvailableVisibility: ItemsSource {
            get: |this: &Rc<ScrollViewerPageViewModel>| ItemsSource::from(this.available_visibility())
        },
        AreSnapPointsRegular: bool {
            get: |this: &Rc<ScrollViewerPageViewModel>| this.are_snap_points_regular(),
            set: |this: &Rc<ScrollViewerPageViewModel>, value: bool| this.set_are_snap_points_regular(value)
        },
        SnapPointsType: SnapPointsType {
            get: |this: &Rc<ScrollViewerPageViewModel>| this.snap_points_type(),
            set: |this: &Rc<ScrollViewerPageViewModel>, value: SnapPointsType| this.set_snap_points_type(value)
        },
        SnapPointsAlignment: SnapPointsAlignment {
            get: |this: &Rc<ScrollViewerPageViewModel>| this.snap_points_alignment(),
            set: |this: &Rc<ScrollViewerPageViewModel>, value: SnapPointsAlignment| {
                this.set_snap_points_alignment(value)
            }
        },
        AvailableSnapPointsType: ItemsSource {
            get: |this: &Rc<ScrollViewerPageViewModel>| ItemsSource::from(this.available_snap_points_type())
        },
        AvailableSnapPointsAlignment: ItemsSource {
            get: |this: &Rc<ScrollViewerPageViewModel>| ItemsSource::from(this.available_snap_points_alignment())
        },
    ],
    notify_property_changed: ScrollViewerPageViewModel,
});

#[repr(C)]
pub struct ScrollViewerPage {
    base: TabbedPage,
}

ferro_class!(ScrollViewerPage: TabbedPage);
ferro_impl_classes!(
    ScrollViewerPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl,
    MultiPageImpl,
    SelectingMultiPageImpl
);
ferro_class_info!(ScrollViewerPage { new: ScrollViewerPage::new });
xaml_class!(ScrollViewerPage, "/Pages/ScrollViewerPage.xaml");

impl ScrollViewerPage {
    pub fn construct() -> Self {
        Self { base: TabbedPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(ScrollViewerPageViewModel::new() as BoxedValue));
        this
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn the_view_model_starts_with_automatic_scroll_bars() {
        let view_model = ScrollViewerPageViewModel::new();
        assert_eq!(ScrollBarVisibility::Auto, view_model.horizontal_scroll_visibility());
        assert_eq!(ScrollBarVisibility::Auto, view_model.vertical_scroll_visibility());
        assert!(view_model.allow_auto_hide());
        assert!(view_model.enable_inertia());
        assert!(!view_model.are_snap_points_regular());
        assert_eq!(SnapPointsType::None, view_model.snap_points_type());
        assert_eq!(SnapPointsAlignment::Near, view_model.snap_points_alignment());
        assert_eq!(4, view_model.available_visibility().items().count());
        assert_eq!(3, view_model.available_snap_points_type().items().count());
        assert_eq!(3, view_model.available_snap_points_alignment().items().count());
    }

    #[test]
    fn a_change_is_notified_once() {
        let view_model = ScrollViewerPageViewModel::new();
        let changes = Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = changes.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        view_model.set_snap_points_type(SnapPointsType::Mandatory);
        view_model.set_snap_points_type(SnapPointsType::Mandatory);
        assert_eq!(vec!["SnapPointsType"], *changes.borrow());
    }
}
