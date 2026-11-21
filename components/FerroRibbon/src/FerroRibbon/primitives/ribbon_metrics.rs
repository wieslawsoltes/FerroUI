use ferroribbon_core::RibbonDensity;
use std::sync::{Arc, OnceLock};

/// Construction/with-expression values for the immutable source record.
#[derive(Clone, Copy, Debug)]
pub struct RibbonMetricsValues {
    pub tab_height: f64,
    pub group_content_height: f64,
    pub group_caption_height: f64,
    pub large_item_height: f64,
    pub large_item_min_width: f64,
    pub row_height: f64,
    pub large_icon_size: f64,
    pub small_icon_size: f64,
    pub simplified_height: f64,
    pub simplified_item_height: f64,
    pub font_size: f64,
    pub caption_font_size: f64,
    pub item_padding: f64,
}
impl Default for RibbonMetricsValues {
    fn default() -> Self {
        Self {
            tab_height: 30.,
            group_content_height: 70.,
            group_caption_height: 17.,
            large_item_height: 68.,
            large_item_min_width: 44.,
            row_height: 22.,
            large_icon_size: 32.,
            small_icon_size: 16.,
            simplified_height: 40.,
            simplified_item_height: 30.,
            font_size: 12.,
            caption_font_size: 11.,
            item_padding: 5.,
        }
    }
}
impl PartialEq for RibbonMetricsValues {
    fn eq(&self, other: &Self) -> bool {
        (self.tab_height == other.tab_height
            || (self.tab_height.is_nan() && other.tab_height.is_nan()))
            && (self.group_content_height == other.group_content_height
                || (self.group_content_height.is_nan() && other.group_content_height.is_nan()))
            && (self.group_caption_height == other.group_caption_height
                || (self.group_caption_height.is_nan() && other.group_caption_height.is_nan()))
            && (self.large_item_height == other.large_item_height
                || (self.large_item_height.is_nan() && other.large_item_height.is_nan()))
            && (self.large_item_min_width == other.large_item_min_width
                || (self.large_item_min_width.is_nan() && other.large_item_min_width.is_nan()))
            && (self.row_height == other.row_height
                || (self.row_height.is_nan() && other.row_height.is_nan()))
            && (self.large_icon_size == other.large_icon_size
                || (self.large_icon_size.is_nan() && other.large_icon_size.is_nan()))
            && (self.small_icon_size == other.small_icon_size
                || (self.small_icon_size.is_nan() && other.small_icon_size.is_nan()))
            && (self.simplified_height == other.simplified_height
                || (self.simplified_height.is_nan() && other.simplified_height.is_nan()))
            && (self.simplified_item_height == other.simplified_item_height
                || (self.simplified_item_height.is_nan() && other.simplified_item_height.is_nan()))
            && (self.font_size == other.font_size
                || (self.font_size.is_nan() && other.font_size.is_nan()))
            && (self.caption_font_size == other.caption_font_size
                || (self.caption_font_size.is_nan() && other.caption_font_size.is_nan()))
            && (self.item_padding == other.item_padding
                || (self.item_padding.is_nan() && other.item_padding.is_nan()))
    }
}
impl Eq for RibbonMetricsValues {}
/// Immutable layout metrics. Presets retain process-wide reference identity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RibbonMetrics {
    values: RibbonMetricsValues,
}
impl RibbonMetrics {
    pub fn new() -> Arc<Self> {
        Self::new_with_values(RibbonMetricsValues::default())
    }
    pub fn new_with_values(values: RibbonMetricsValues) -> Arc<Self> {
        Arc::new(Self { values })
    }
    pub fn values(&self) -> RibbonMetricsValues {
        self.values
    }
    pub fn with_values(&self, update: impl FnOnce(&mut RibbonMetricsValues)) -> Arc<Self> {
        let mut values = self.values;
        update(&mut values);
        Self::new_with_values(values)
    }
    pub fn tab_height(&self) -> f64 {
        self.values.tab_height
    }
    pub fn group_content_height(&self) -> f64 {
        self.values.group_content_height
    }
    pub fn group_caption_height(&self) -> f64 {
        self.values.group_caption_height
    }
    pub fn large_item_height(&self) -> f64 {
        self.values.large_item_height
    }
    pub fn large_item_min_width(&self) -> f64 {
        self.values.large_item_min_width
    }
    pub fn row_height(&self) -> f64 {
        self.values.row_height
    }
    pub fn large_icon_size(&self) -> f64 {
        self.values.large_icon_size
    }
    pub fn small_icon_size(&self) -> f64 {
        self.values.small_icon_size
    }
    pub fn simplified_height(&self) -> f64 {
        self.values.simplified_height
    }
    pub fn simplified_item_height(&self) -> f64 {
        self.values.simplified_item_height
    }
    pub fn font_size(&self) -> f64 {
        self.values.font_size
    }
    pub fn caption_font_size(&self) -> f64 {
        self.values.caption_font_size
    }
    pub fn item_padding(&self) -> f64 {
        self.values.item_padding
    }
    fn presets() -> &'static [Arc<Self>; 3] {
        static VALUES: OnceLock<[Arc<RibbonMetrics>; 3]> = OnceLock::new();
        VALUES.get_or_init(|| {
            [
                Self::new(),
                Self::new_with_values(RibbonMetricsValues {
                    tab_height: 26.,
                    group_content_height: 64.,
                    group_caption_height: 15.,
                    large_item_height: 62.,
                    large_item_min_width: 40.,
                    row_height: 20.,
                    large_icon_size: 26.,
                    small_icon_size: 16.,
                    simplified_height: 34.,
                    simplified_item_height: 26.,
                    font_size: 11.,
                    caption_font_size: 10.,
                    item_padding: 4.,
                }),
                Self::new_with_values(RibbonMetricsValues {
                    tab_height: 38.,
                    group_content_height: 94.,
                    group_caption_height: 19.,
                    large_item_height: 92.,
                    large_item_min_width: 56.,
                    row_height: 30.,
                    large_icon_size: 32.,
                    small_icon_size: 20.,
                    simplified_height: 50.,
                    simplified_item_height: 40.,
                    font_size: 13.,
                    caption_font_size: 12.,
                    item_padding: 8.,
                }),
            ]
        })
    }
    pub fn comfortable() -> Arc<Self> {
        Self::presets()[0].clone()
    }
    pub fn compact() -> Arc<Self> {
        Self::presets()[1].clone()
    }
    pub fn touch() -> Arc<Self> {
        Self::presets()[2].clone()
    }
    pub fn for_density(density: RibbonDensity) -> Arc<Self> {
        match density {
            RibbonDensity::Compact => Self::compact(),
            RibbonDensity::Touch => Self::touch(),
            _ => Self::comfortable(),
        }
    }
}
ferroui_base::ferro_markup_type!(class RibbonMetrics {
    namespace:"FerroRibbon.Controls",this:Arc<RibbonMetrics>,handles:[Arc<RibbonMetrics>,Option<Arc<RibbonMetrics>>],
    constructors:[()=>RibbonMetrics::new],
    static_properties:[
        Comfortable:Arc<RibbonMetrics> {get:RibbonMetrics::comfortable},
        Compact:Arc<RibbonMetrics> {get:RibbonMetrics::compact},
        Touch:Arc<RibbonMetrics> {get:RibbonMetrics::touch},
    ],
    methods:[static fn For(RibbonDensity)->Arc<RibbonMetrics> => RibbonMetrics::for_density],
    properties:[
TabHeight:f64 {get:|this:&Arc<RibbonMetrics>|this.tab_height()},
GroupContentHeight:f64 {get:|this:&Arc<RibbonMetrics>|this.group_content_height()},
GroupCaptionHeight:f64 {get:|this:&Arc<RibbonMetrics>|this.group_caption_height()},
LargeItemHeight:f64 {get:|this:&Arc<RibbonMetrics>|this.large_item_height()},
LargeItemMinWidth:f64 {get:|this:&Arc<RibbonMetrics>|this.large_item_min_width()},
RowHeight:f64 {get:|this:&Arc<RibbonMetrics>|this.row_height()},
LargeIconSize:f64 {get:|this:&Arc<RibbonMetrics>|this.large_icon_size()},
SmallIconSize:f64 {get:|this:&Arc<RibbonMetrics>|this.small_icon_size()},
SimplifiedHeight:f64 {get:|this:&Arc<RibbonMetrics>|this.simplified_height()},
SimplifiedItemHeight:f64 {get:|this:&Arc<RibbonMetrics>|this.simplified_item_height()},
FontSize:f64 {get:|this:&Arc<RibbonMetrics>|this.font_size()},
CaptionFontSize:f64 {get:|this:&Arc<RibbonMetrics>|this.caption_font_size()},
ItemPadding:f64 {get:|this:&Arc<RibbonMetrics>|this.item_padding()},
],
});
