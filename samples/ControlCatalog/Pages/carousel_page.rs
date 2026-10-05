//! Port of `Pages/CarouselPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage.xaml`.

use super::transitions::{CardStackPageTransition, WaveRevealPageTransition};
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::animation::{
    CompositePageTransition, CrossFade, IPageTransition, PageSlide, Rotate3DTransition, SlideAxis, TimeSpan,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::media::RotateTransform;
use ferroui_base::{ferro_class_info, instantiate, FerroPropertyChangedEventArgs, Ref, Thickness};
use ferroui_controls::primitives::{RangeBaseValueChangedEventArgs, SelectingItemsControl};
use ferroui_controls::shapes::Path;
use ferroui_controls::{
    Button, Carousel, CheckBox, ComboBox, ContentPage, Grid, SelectionChangedEventArgs, Slider, TextBlock,
};
use std::rc::Rc;

/// `value.ToString("0.##")`: at most two decimals, without trailing zeros.
fn format_up_to_two_decimals(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// `Math.Round(value, 2)`: to two decimals, midpoints to even.
fn round_to_two_decimals(value: f64) -> f64 {
    (value * 100.0).round_ties_even() / 100.0
}

#[repr(C)]
pub struct CarouselPage {
    base: ContentPage,
}

content_page_class!(CarouselPage);
ferro_class_info!(CarouselPage { new: CarouselPage::new });
xaml_class!(CarouselPage, "/Pages/CarouselPage.xaml");

impl CarouselPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        this.left().click(move |_s, _e| {
            if let Some(this) = weak.upgrade() {
                this.carousel().previous();
            }
        });
        let weak = this.downgrade();
        this.right().click(move |_s, _e| {
            if let Some(this) = weak.upgrade() {
                this.carousel().next();
            }
        });
        let weak = this.downgrade();
        this.transition().selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.transition_changed(sender, e);
            }
        });
        let weak = this.downgrade();
        this.orientation().selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.transition_changed(sender, e);
            }
        });
        let weak = this.downgrade();
        this.viewport_fraction().value_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.viewport_fraction_changed(sender, e);
            }
        });

        this.wrap_selection().set_is_checked(Some(this.carousel().wrap_selection()));
        let weak = this.downgrade();
        this.wrap_selection().is_checked_changed(move |_s, _e| {
            if let Some(this) = weak.upgrade() {
                this.carousel().set_wrap_selection(this.wrap_selection().is_checked().unwrap_or(false));
                this.update_button_state();
            }
        });

        this.swipe_enabled().set_is_checked(Some(this.carousel().is_swipe_enabled()));
        let weak = this.downgrade();
        this.swipe_enabled().is_checked_changed(move |_s, _e| {
            if let Some(this) = weak.upgrade() {
                this.carousel().set_is_swipe_enabled(this.swipe_enabled().is_checked().unwrap_or(false));
            }
        });

        let weak = this.downgrade();
        this.carousel().property_changed(move |e: &FerroPropertyChangedEventArgs<'_>| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            if e.property() == SelectingItemsControl::selected_index_property().as_property() {
                this.update_button_state();
            } else if e.property() == Carousel::viewport_fraction_property().as_property() {
                this.update_viewport_fraction_display();
            }
        });

        this.carousel().set_viewport_fraction(this.viewport_fraction().value());
        this.update_button_state();
        this.update_viewport_fraction_display();
        this
    }

    fn left(&self) -> Ref<Button> {
        self.get_control::<Button>("left")
    }

    fn right(&self) -> Ref<Button> {
        self.get_control::<Button>("right")
    }

    fn carousel(&self) -> Ref<Carousel> {
        self.get_control::<Carousel>("carousel")
    }

    fn transition(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("transition")
    }

    fn orientation(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("orientation")
    }

    fn viewport_fraction(&self) -> Ref<Slider> {
        self.get_control::<Slider>("viewportFraction")
    }

    fn wrap_selection(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("wrapSelection")
    }

    fn swipe_enabled(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("swipeEnabled")
    }

    fn text(&self, name: &str) -> Ref<TextBlock> {
        self.get_control::<TextBlock>(name)
    }

    fn update_button_state(&self) {
        let carousel = self.carousel();
        self.text("itemsCountIndicator").set_text(Some(&carousel.item_count().to_string()));
        self.text("selectedIndexIndicator").set_text(Some(&carousel.selected_index().to_string()));

        let wrap = carousel.wrap_selection();
        self.left().set_is_enabled(wrap || carousel.selected_index() > 0);
        self.right().set_is_enabled(wrap || carousel.selected_index() < carousel.item_count() - 1);
    }

    fn viewport_fraction_changed(&self, _sender: &Interactive, e: &RangeBaseValueChangedEventArgs) {
        self.carousel().set_viewport_fraction(round_to_two_decimals(e.new_value()));
        self.update_viewport_fraction_display();
    }

    fn update_viewport_fraction_display(&self) {
        let value = self.carousel().viewport_fraction();
        self.text("viewportFractionIndicator").set_text(Some(&format!("{value:.2}")));

        let pages_in_view = 1.0 / value;
        self.text("viewportFractionHint").set_text(Some(&if value >= 1.0 {
            String::from("1.00 shows a single full page.")
        } else {
            format!(
                "{} pages fit in view. Try 0.80 for peeking or 0.33 for three full items.",
                format_up_to_two_decimals(pages_in_view)
            )
        }));
    }

    fn transition_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        let is_vertical = self.orientation().selected_index() == 1;
        let axis = if is_vertical { SlideAxis::Vertical } else { SlideAxis::Horizontal };
        let carousel = self.carousel();

        let transition: Option<Option<Rc<dyn IPageTransition>>> = match self.transition().selected_index() {
            0 => Some(None),
            1 => Some(Some(Rc::new(PageSlide::with_duration(TimeSpan::from_seconds(0.25), axis)))),
            2 => Some(Some(Rc::new(CrossFade::with_duration(TimeSpan::from_seconds(0.25))))),
            3 => Some(Some(Rc::new(Rotate3DTransition::with_duration(TimeSpan::from_seconds(0.5), axis, None)))),
            4 => Some(Some(Rc::new(CardStackPageTransition::with_duration(TimeSpan::from_seconds(0.5), axis)))),
            5 => Some(Some(Rc::new(WaveRevealPageTransition::with_duration(TimeSpan::from_seconds(0.8), axis)))),
            6 => {
                let composite = CompositePageTransition::new();
                composite.add(Rc::new(PageSlide::with_duration(TimeSpan::from_seconds(0.25), axis)));
                composite.add(Rc::new(CrossFade::with_duration(TimeSpan::from_seconds(0.25))));
                Some(Some(Rc::new(composite)))
            }
            _ => None,
        };
        if let Some(transition) = transition {
            carousel.set_page_transition(transition);
        }

        self.update_layout_for_orientation(is_vertical);
    }

    fn update_layout_for_orientation(&self, is_vertical: bool) {
        let (left, right) = (self.left(), self.right());
        let (left_arrow, right_arrow) = (self.get_control::<Path>("leftArrow"), self.get_control::<Path>("rightArrow"));
        if is_vertical {
            Grid::set_column(&left, 1);
            Grid::set_row(&left, 0);
            Grid::set_column(&right, 1);
            Grid::set_row(&right, 2);

            left.set_padding(Thickness::symmetric(20.0, 10.0));
            right.set_padding(Thickness::symmetric(20.0, 10.0));

            left_arrow.set_render_transform(Some(RotateTransform::with_angle(90.0).into()));
            right_arrow.set_render_transform(Some(RotateTransform::with_angle(90.0).into()));
        } else {
            Grid::set_column(&left, 0);
            Grid::set_row(&left, 1);
            Grid::set_column(&right, 2);
            Grid::set_row(&right, 1);

            left.set_padding(Thickness::symmetric(10.0, 20.0));
            right.set_padding(Thickness::symmetric(10.0, 20.0));

            left_arrow.set_render_transform(None);
            right_arrow.set_render_transform(None);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn numbers_are_formatted_as_the_page_shows_them() {
        assert_eq!("1.25", format_up_to_two_decimals(1.25));
        assert_eq!("3", format_up_to_two_decimals(3.0));
        assert_eq!("3.3", format_up_to_two_decimals(3.3));
        assert_eq!(0.33, round_to_two_decimals(0.333));
        assert_eq!(0.8, round_to_two_decimals(0.8049));
    }
}
