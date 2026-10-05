//! Port of `Controls/SampleGalleryPage.cs`.

use super::{SampleGroups, SampleInfo, SampleInfoGroup};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{ICommand, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, DirectProperty,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType,
    StyledElementImpl, StyledProperty, Thickness, TypeInfo, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentPage, Control, ControlImpl, PageImpl, ScrollViewer, SelectableTextBlock, StackPanel, TextBlock,
};
use mini_mvvm::{start_async, MiniCommand};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// A catalog page for controls with many substantial samples. The page
/// shows its `Description` and a grouped grid of cards built from
/// `Samples`, and opens a sample on the hosting navigation page. Keeping
/// the samples on the host stack leaves a single navigation bar on screen:
/// the page title and the drawer toggle here, the sample title and the back
/// button once a sample is open. Samples are constructed only when opened.
#[repr(C)]
pub struct SampleGalleryPage {
    base: ContentPage,
    samples: RefCell<FerroList<Rc<SampleInfo>>>,
    groups: RefCell<FerroList<Rc<SampleInfoGroup>>>,
    opening: Cell<bool>,
    open_sample_command: RefCell<Option<Rc<dyn ICommand>>>,
}

ferro_class!(SampleGalleryPage: ContentPage);
ferro_class_info!(SampleGalleryPage {
    new: SampleGalleryPage::new,
    markup: {
        properties: [
            OpenSampleCommand: Rc<dyn ICommand> { get: |this: &Ref<SampleGalleryPage>| this.open_sample_command() },
        ],
    },
});
ferro_impl_classes!(
    SampleGalleryPage: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);

impl FerroObjectImpl for SampleGalleryPage {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        // The command is a field of the page: it holds the page weakly.
        let weak = this.to_ref().downgrade();
        let command = MiniCommand::create_with::<Rc<SampleInfo>>(move |sample| {
            if let Some(this) = weak.upgrade() {
                this.open_async(&sample);
            }
        });
        *this.open_sample_command.borrow_mut() = Some(command.as_command());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::samples_property().as_property() {
            // The grouping keeps first appearance order, so unknown groups stay in registry order at the end.
            let mut groups: Vec<(String, Vec<Rc<SampleInfo>>)> = Vec::new();
            for sample in this.samples().snapshot().iter() {
                let key = sample.group();
                match groups.iter_mut().find(|(group, _)| *group == key) {
                    Some((_, samples)) => samples.push(sample.clone()),
                    None => groups.push((key, vec![sample.clone()])),
                }
            }
            // A stable sort, as the ordering of the original.
            groups.sort_by_key(|(group, _)| SampleGroups::index_of(group));
            this.set_groups(FerroList::from_items(
                groups.into_iter().map(|(group, samples)| SampleInfoGroup::new(&group, FerroList::from_items(samples))),
            ));
        }
    }
}

impl StyledElementImpl for SampleGalleryPage {
    // Pages derive from this class, and a theme is looked up by the exact type.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <SampleGalleryPage as StaticType>::TYPE
    }
}

ferro_properties! {
    impl SampleGalleryPage {
        pub fn description_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SampleGalleryPage, _>("Description", None)
        }

        pub fn samples_property() -> DirectProperty<SampleGalleryPage, FerroList<Rc<SampleInfo>>> {
            FerroProperty::register_direct::<SampleGalleryPage, _>(
                "Samples",
                SampleGalleryPage::samples,
                Some(SampleGalleryPage::set_samples),
                FerroList::new(),
            )
        }

        pub fn groups_property() -> DirectProperty<SampleGalleryPage, FerroList<Rc<SampleInfoGroup>>> {
            FerroProperty::register_direct::<SampleGalleryPage, _>(
                "Groups",
                SampleGalleryPage::groups,
                None,
                FerroList::new(),
            )
        }
    }
}

/// Clears the flag of a page that is opening a sample (the `finally` of the
/// original).
struct OpeningScope(Ref<SampleGalleryPage>);

impl Drop for OpeningScope {
    fn drop(&mut self) {
        self.0.opening.set(false);
    }
}

/// The text of the payload of a panic.
fn panic_text(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else {
        String::from("The sample panicked.")
    }
}

impl SampleGalleryPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            samples: RefCell::new(FerroList::new()),
            groups: RefCell::new(FerroList::new()),
            opening: Cell::new(false),
            open_sample_command: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// One or two sentences saying what the control is for. Shown above the
    /// sample cards.
    pub fn description(&self) -> Option<String> {
        self.get_value(Self::description_property())
    }

    pub fn set_description(&self, value: Option<&str>) {
        self.set_value(Self::description_property(), value.map(str::to_string))
    }

    /// The samples to offer. Cards are grouped by the group of the sample
    /// and the groups are ordered as in [`SampleGroups`].
    pub fn samples(&self) -> FerroList<Rc<SampleInfo>> {
        self.samples.borrow().clone()
    }

    pub fn set_samples(&self, value: FerroList<Rc<SampleInfo>>) {
        self.set_and_raise(Self::samples_property(), &self.samples, value);
    }

    pub fn groups(&self) -> FerroList<Rc<SampleInfoGroup>> {
        self.groups.borrow().clone()
    }

    fn set_groups(&self, value: FerroList<Rc<SampleInfoGroup>>) {
        self.set_and_raise(Self::groups_property(), &self.groups, value);
    }

    pub fn open_sample_command(&self) -> Rc<dyn ICommand> {
        self.open_sample_command.borrow().clone().expect("the command is created by the constructor")
    }

    /// Opens a sample on the hosting navigation page (`async Task`).
    fn open_async(&self, sample: &Rc<SampleInfo>) {
        let navigation = self.navigation();
        let Some(navigation) = navigation.filter(|_| !self.opening.get()) else {
            return;
        };

        self.opening.set(true);
        let scope = OpeningScope(self.to_ref());

        let factory = sample.factory();
        let content = match catch_unwind(AssertUnwindSafe(|| factory())) {
            Ok(content) => content,
            Err(payload) => Self::create_error_content(sample, &panic_text(&*payload)),
        };

        let page = ContentPage::new();
        page.set_header(Some(Rc::new(sample.title()) as BoxedValue));
        // Transparent like the catalog pages, so window transparency shows through.
        let transparent: Rc<dyn IBrush> = Brushes::transparent();
        page.set_background(Some(transparent));
        page.set_content(Some(Control::boxed(&content)));
        page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
        page.set_vertical_content_alignment(VerticalAlignment::Stretch);

        let pushed = navigation.push_async_with_transition(page.upcast(), None);
        let _ = start_async(async move {
            let _scope = scope;
            let _ = pushed.await;
        });
    }

    fn create_error_content(sample: &SampleInfo, error: &str) -> Ref<Control> {
        let title = TextBlock::new();
        title.classes().add("sample-card-title");
        title.set_text(Some(&format!("The sample \"{}\" failed to load.", sample.title())));

        let caption = SelectableTextBlock::new();
        caption.classes().add("sample-caption");
        caption.set_text(Some(error));

        let panel = StackPanel::new();
        panel.set_spacing(12.0);
        panel.children().add(title);
        panel.children().add(caption);

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_padding(Thickness::symmetric(24.0, 20.0));
        scroll_viewer.set_content(Some(Control::boxed(&panel)));
        scroll_viewer.upcast()
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_controls::Border;

    fn sample(group: &str, title: &str) -> Rc<SampleInfo> {
        SampleInfo::new(group, title, "", || Border::new().upcast())
    }

    #[test]
    fn samples_are_grouped_in_the_order_of_the_known_groups() {
        let page = SampleGalleryPage::new();
        page.set_samples(FerroList::from_items([
            sample("Custom", "a"),
            sample(SampleGroups::EVENTS, "b"),
            sample(SampleGroups::OVERVIEW, "c"),
            sample("Other", "d"),
            sample(SampleGroups::EVENTS, "e"),
            sample("Custom", "f"),
        ]));

        let groups = page.groups().snapshot();
        let headers: Vec<String> = groups.iter().map(|group| group.header()).collect();
        assert_eq!(vec!["Overview", "Events", "Custom", "Other"], headers);
        let titles: Vec<String> = groups[1].samples().snapshot().iter().map(|sample| sample.title()).collect();
        assert_eq!(vec!["b", "e"], titles);
    }
}
