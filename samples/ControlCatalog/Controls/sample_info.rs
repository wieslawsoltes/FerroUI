//! Port of `Controls/SampleInfo.cs`: `SampleInfo`, `SampleInfoGroup` and
//! `SampleGroups`.

use ferroui_base::collections::FerroList;
use ferroui_base::{ferro_markup_type, Ref};
use ferroui_controls::Control;
use std::rc::Rc;

/// One entry in a `SampleGalleryPage` registry: a card on the gallery home
/// page that opens the control created by `Factory`. Construction is
/// deferred until the card is clicked.
pub struct SampleInfo {
    group: String,
    title: String,
    description: String,
    factory: Rc<dyn Fn() -> Ref<Control>>,
}

impl PartialEq for SampleInfo {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl SampleInfo {
    pub fn new(group: &str, title: &str, description: &str, factory: impl Fn() -> Ref<Control> + 'static) -> Rc<Self> {
        Rc::new(Self {
            group: group.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            factory: Rc::new(factory),
        })
    }

    pub fn group(&self) -> String {
        self.group.clone()
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn description(&self) -> String {
        self.description.clone()
    }

    pub fn factory(&self) -> Rc<dyn Fn() -> Ref<Control>> {
        self.factory.clone()
    }
}

ferro_markup_type!(class SampleInfo {
    this: Rc<SampleInfo>,
    handles: [SampleInfo, Rc<SampleInfo>, Option<Rc<SampleInfo>>],
    properties: [
        Group: String { get: |this: &Rc<SampleInfo>| this.group() },
        Title: String { get: |this: &Rc<SampleInfo>| this.title() },
        Description: String { get: |this: &Rc<SampleInfo>| this.description() },
    ],
});

/// The samples of one group, as shown under a single header on a
/// `SampleGalleryPage`.
pub struct SampleInfoGroup {
    header: String,
    samples: FerroList<Rc<SampleInfo>>,
}

impl PartialEq for SampleInfoGroup {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl SampleInfoGroup {
    pub fn new(header: &str, samples: FerroList<Rc<SampleInfo>>) -> Rc<Self> {
        Rc::new(Self { header: header.to_string(), samples })
    }

    pub fn header(&self) -> String {
        self.header.clone()
    }

    pub fn samples(&self) -> FerroList<Rc<SampleInfo>> {
        self.samples.clone()
    }
}

ferro_markup_type!(class SampleInfoGroup {
    this: Rc<SampleInfoGroup>,
    handles: [SampleInfoGroup, Rc<SampleInfoGroup>, Option<Rc<SampleInfoGroup>>],
    properties: [
        Header: String { get: |this: &Rc<SampleInfoGroup>| this.header() },
        Samples: FerroList<Rc<SampleInfo>> { get: |this: &Rc<SampleInfoGroup>| this.samples() },
    ],
});

/// The group names shared by every sample gallery, in the order they are
/// shown. A registry may use other group names; those are appended after
/// the known ones in the order they first appear.
pub struct SampleGroups;

impl SampleGroups {
    pub const OVERVIEW: &'static str = "Overview";
    pub const POPULATE: &'static str = "Populate";
    pub const APPEARANCE: &'static str = "Appearance";
    pub const FEATURES: &'static str = "Features";
    pub const EVENTS: &'static str = "Events";
    pub const PERFORMANCE: &'static str = "Performance";
    pub const SHOWCASES: &'static str = "Showcases";

    const ORDERED_NAMES: [&'static str; 7] = [
        Self::OVERVIEW,
        Self::POPULATE,
        Self::APPEARANCE,
        Self::FEATURES,
        Self::EVENTS,
        Self::PERFORMANCE,
        Self::SHOWCASES,
    ];

    pub(crate) fn index_of(group: &str) -> i32 {
        Self::ORDERED_NAMES.iter().position(|name| *name == group).map_or(i32::MAX, |index| index as i32)
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn known_groups_are_ordered_and_unknown_ones_come_last() {
        assert_eq!(0, SampleGroups::index_of("Overview"));
        assert_eq!(6, SampleGroups::index_of(SampleGroups::SHOWCASES));
        assert_eq!(i32::MAX, SampleGroups::index_of("Gallery"));
        assert_eq!(i32::MAX, SampleGroups::index_of("overview"));
    }
}
