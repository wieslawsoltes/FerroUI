//! Port of `InteractiveLineControl.cs`: a control that formats its text as one line with the
//! text formatter and draws the line with what the text formatting interface says about it:
//! the extent, the baseline, the bounds of the text and of its runs, the caret stops in both
//! directions, the backspace stops and the distance of every character.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, ITextSource, TextCharacters, TextFormatter, TextLayout,
    TextLine, TextParagraphProperties, TextRun, TextRunProperties,
};
use ferroui_base::media::{
    ArcSegment, BaselineAlignment, Brushes, CharacterHit, DashStyle, DrawingContext, EdgeMode, FlowDirection, FontFamily,
    FontFeatureCollection, FontStretch, FontStyle, FontWeight, FormattedText, IBrush, IDashStyle, IPen, PathFigure,
    PathGeometry, Pen, PenLineCap, PenLineJoin, PolylineGeometry, RenderOptions, SweepDirection, TextAlignment,
    TextOptions, TextRenderingMode, TextWrapping, Typeface,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{CultureInfo, HandlerList};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Matrix, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty, Vector, Visual,
    VisualImpl, WeakRef,
};
use ferroui_controls::documents::TextElement;
use ferroui_controls::{Border, Control, ControlImpl, TextBlock};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

#[repr(C)]
pub struct InteractiveLineControl {
    base: Control,

    extent_pen: RefCell<Option<Rc<dyn IPen>>>,
    baseline_pen: RefCell<Option<Rc<dyn IPen>>>,
    text_bounds_pen: RefCell<Option<Rc<dyn IPen>>>,
    run_bounds_pen: RefCell<Option<Rc<dyn IPen>>>,
    next_hit_pen: RefCell<Option<Rc<dyn IPen>>>,
    previous_hit_pen: RefCell<Option<Rc<dyn IPen>>>,
    backspace_hit_pen: RefCell<Option<Rc<dyn IPen>>>,
    distance_pen: RefCell<Option<Rc<dyn IPen>>>,

    text_run_properties: RefCell<Option<Rc<GenericTextRunProperties>>>,
    text_paragraph_properties: RefCell<Option<Rc<GenericTextParagraphProperties>>>,

    /// Assigned once, by the constructor.
    text_source: OnceCell<Rc<TextSource>>,

    text_line: RefCell<Option<Rc<dyn TextLine>>>,
    text_layout: RefCell<Option<Rc<TextLayout>>>,
    text_line_size: Cell<Option<Size>>,
    ink_size: Cell<Option<Size>>,

    text_line_changed: HandlerList<dyn Fn()>,

    /// The subscription to the changes of the font feature collection the control has
    /// (`CollectionChanged += OnFeatureCollectionChanged` of the managed original).
    feature_collection_subscription: RefCell<Option<Rc<dyn IDisposable>>>,

    labels_cache: RefCell<HashMap<String, Rc<FormattedText>>>,

    ink_render_bounds: Cell<Rect>,
    line_render_bounds: Cell<Rect>,
}

ferro_class!(InteractiveLineControl: Control);
ferro_impl_classes!(InteractiveLineControl: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(InteractiveLineControl { new: InteractiveLineControl::new });

ferro_properties! {
    impl InteractiveLineControl {
        /// Defines the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            TextBlock::text_property().add_owner::<InteractiveLineControl>()
        }

        /// Defines the `Background` property.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Border::background_property().add_owner::<InteractiveLineControl>()
        }

        pub fn extent_stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<InteractiveLineControl, _>("ExtentStroke", None)
        }

        pub fn baseline_stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<InteractiveLineControl, _>("BaselineStroke", None)
        }

        pub fn text_bounds_stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<InteractiveLineControl, _>("TextBoundsStroke", None)
        }

        pub fn run_bounds_stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<InteractiveLineControl, _>("RunBoundsStroke", None)
        }

        pub fn next_hit_stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<InteractiveLineControl, _>("NextHitStroke", None)
        }

        pub fn backspace_hit_stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<InteractiveLineControl, _>("BackspaceHitStroke", None)
        }

        pub fn previous_hit_stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<InteractiveLineControl, _>("PreviousHitStroke", None)
        }

        pub fn distance_stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<InteractiveLineControl, _>("DistanceStroke", None)
        }

        // TextRunProperties

        /// Defines the `FontFamily` property.
        pub fn font_family_property() -> StyledProperty<FontFamily> {
            TextElement::font_family_property().add_owner::<InteractiveLineControl>()
        }

        /// Defines the `FontFeatures` property.
        pub fn font_features_property() -> StyledProperty<Option<FontFeatureCollection>> {
            TextElement::font_features_property().add_owner::<InteractiveLineControl>()
        }

        /// Defines the `FontSize` property.
        pub fn font_size_property() -> StyledProperty<f64> {
            TextElement::font_size_property().add_owner::<InteractiveLineControl>()
        }

        /// Defines the `FontStyle` property.
        pub fn font_style_property() -> StyledProperty<FontStyle> {
            TextElement::font_style_property().add_owner::<InteractiveLineControl>()
        }

        /// Defines the `FontWeight` property.
        pub fn font_weight_property() -> StyledProperty<FontWeight> {
            TextElement::font_weight_property().add_owner::<InteractiveLineControl>()
        }

        /// Defines the `FontStretch` property.
        pub fn font_stretch_property() -> StyledProperty<FontStretch> {
            TextElement::font_stretch_property().add_owner::<InteractiveLineControl>()
        }
    }
}

/// The text source of the control: the whole text of the control as one run of characters.
struct TextSource {
    owner: WeakRef<InteractiveLineControl>,
}

impl TextSource {
    fn new(owner: WeakRef<InteractiveLineControl>) -> Self {
        Self { owner }
    }
}

impl ITextSource for TextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        // The control owns its text source: the source holds the control weakly.
        let owner = self.owner.upgrade()?;
        let text = owner.text().unwrap_or_default();

        // `text.Length`: the length of the text in UTF-16 code units.
        if text_source_index < 0 || text_source_index >= text.encode_utf16().count() as i32 {
            return None;
        }

        Some(Rc::new(TextCharacters::from_str(&text, owner.text_run_properties())))
    }
}

const VERTICAL_SPACING: f64 = 5.0;
const HORIZONTAL_SPACING: f64 = 5.0;
const ARROW_SIZE: f64 = 5.0;
const LABEL_FONT_SIZE: f64 = 9.0;

impl FerroObjectImpl for InteractiveLineControl {
    /// The body of the constructor of the managed original.
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let _ = this.text_source.set(Rc::new(TextSource::new(this.to_ref().downgrade())));

        RenderOptions::set_edge_mode(this, EdgeMode::Aliased);
        TextOptions::set_text_rendering_mode(this, TextRenderingMode::SubpixelAntialias);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        match change.property().name() {
            "FontFamily" | "FontSize" => this.invalidate_text_run_properties(),

            "FontFeatures" => {
                // `oc.CollectionChanged -= OnFeatureCollectionChanged`
                let subscription = this.feature_collection_subscription.borrow_mut().take();
                if let Some(subscription) = subscription {
                    subscription.dispose();
                }
                // `nc.CollectionChanged += OnFeatureCollectionChanged`
                if let Some(nc) = change.get_new_value::<Option<FontFeatureCollection>>() {
                    let weak = this.to_ref().downgrade();
                    let subscription = nc.collection_changed(move |_| {
                        if let Some(this) = weak.upgrade() {
                            this.on_feature_collection_changed();
                        }
                    });
                    *this.feature_collection_subscription.borrow_mut() = Some(subscription);
                }
                this.on_feature_collection_changed();
            }

            "FontStyle" | "FontWeight" | "FontStretch" => this.invalidate_text_run_properties(),

            "FlowDirection" => this.invalidate_text_paragraph_properties(),

            "Text" => this.invalidate_text_line(),

            "BaselineStroke" => {
                *this.baseline_pen.borrow_mut() = None;
                this.invalidate_visual();
            }

            "TextBoundsStroke" => {
                *this.text_bounds_pen.borrow_mut() = None;
                this.invalidate_visual();
            }

            "RunBoundsStroke" => {
                *this.run_bounds_pen.borrow_mut() = None;
                this.invalidate_visual();
            }

            "NextHitStroke" => {
                *this.next_hit_pen.borrow_mut() = None;
                this.invalidate_visual();
            }

            "PreviousHitStroke" => {
                *this.previous_hit_pen.borrow_mut() = None;
                this.invalidate_visual();
            }

            "BackspaceHitStroke" => {
                *this.backspace_hit_pen.borrow_mut() = None;
                this.invalidate_visual();
            }

            _ => {}
        }

        // The managed original calls the implementation of the base class a second time.
        Self::parent_on_property_changed(this, change);
    }
}

impl LayoutableImpl for InteractiveLineControl {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        if this.text_line().is_none() {
            return Size::default();
        }

        let text_line_size = this.text_line_size();
        let ink_size = this.ink_size();
        Size::new(f64::max(text_line_size.width, ink_size.width), f64::max(text_line_size.height, ink_size.height))
    }
}

impl VisualImpl for InteractiveLineControl {
    fn render(this: &Self, context: &mut DrawingContext) {
        let Some(text_line) = this.text_line() else {
            return;
        };

        // overhang leading should be negative when extending (e.g. for j)   WPF: "When the leading alignment point comes before the leading drawn pixel, the value is negative." - docs wrong but values correct
        // overhang trailing should be negative when extending (e.g. for f)  WPF: "The OverhangTrailing value will be positive when the trailing drawn pixel comes before the trailing alignment point."
        // overhang after should be negative when inside (e.g. for x) WPF: "The value is positive if the bottommost drawn pixel goes below the line bottom, and is negative if it is within (on or above) the line."
        // => we want overhang before to be negative when inside (e.g. for x)

        let overhang_before = text_line.extent() - text_line.overhang_after() - text_line.height();
        let ink_bounds =
            Rect::from_position_size(Point::new(text_line.overhang_leading(), -overhang_before), this.ink_size());
        let mut line_bounds = Rect::from_position_size(Point::new(0.0, 0.0), this.text_line_size());

        if ink_bounds.left() < 0.0 {
            line_bounds = line_bounds.translate(Vector::new(-ink_bounds.left(), 0.0));
        }

        if ink_bounds.top() < 0.0 {
            line_bounds = line_bounds.translate(Vector::new(0.0, -ink_bounds.top()));
        }

        this.ink_render_bounds.set(ink_bounds);
        this.line_render_bounds.set(line_bounds);

        let bounds = Rect::new(
            0.0,
            0.0,
            f64::max(ink_bounds.right(), line_bounds.right()),
            f64::max(ink_bounds.bottom(), line_bounds.bottom()),
        );
        let mut label_x = bounds.right() + HORIZONTAL_SPACING;

        if let Some(background) = this.background() {
            context.fill_rectangle(&background, line_bounds, 0.0);
        }

        if let Some(extent_stroke) = this.extent_stroke() {
            context.draw_rectangle_outline(&this.extent_pen(), ink_bounds, 0.0);
            this.render_label(context, "Extent", Some(extent_stroke), label_x, ink_bounds.top(), TextAlignment::Left, false);
        }

        let transform = context.push_transform(Matrix::create_translation(line_bounds.left(), line_bounds.top()));
        {
            label_x -= line_bounds.left(); // labels to ignore horizontal transform

            if let Some(baseline_stroke) = this.baseline_stroke() {
                // no other lines currently available in the framework
                this.render_font_line(context, text_line.baseline(), line_bounds.width, &this.baseline_pen());
                this.render_label(
                    context,
                    "Baseline",
                    Some(baseline_stroke),
                    label_x,
                    text_line.baseline(),
                    TextAlignment::Left,
                    false,
                );
            }

            text_line.draw(context, Point::default());

            let run_bounds_stroke = this.run_bounds_stroke();
            if this.text_bounds_stroke().is_some() || run_bounds_stroke.is_some() {
                let text_bounds = text_line.get_text_bounds(text_line.first_text_source_index(), text_line.length());
                for text_bound in &text_bounds {
                    if run_bounds_stroke.is_some() {
                        let run_bounds = text_bound.text_run_bounds();
                        for run_bound in run_bounds {
                            context.draw_rectangle_outline(&this.run_bounds_pen(), run_bound.rectangle(), 0.0);
                        }
                    }

                    context.draw_rectangle_outline(&this.text_bounds_pen(), text_bound.rectangle(), 0.0);
                }
            }

            let mut y = f64::max(ink_bounds.bottom(), line_bounds.bottom()) + VERTICAL_SPACING * 2.0;

            if let Some(next_hit_stroke) = this.next_hit_stroke() {
                this.render_hits(
                    context,
                    &this.next_hit_pen(),
                    &*text_line,
                    &|hit| text_line.get_next_caret_character_hit(hit),
                    CharacterHit::new(0),
                    &mut y,
                );
                this.render_label(
                    context,
                    "GetNextCaretCharacterHit",
                    Some(next_hit_stroke),
                    label_x,
                    y,
                    TextAlignment::Left,
                    false,
                );
                y += VERTICAL_SPACING * 2.0;
            }

            if let Some(previous_hit_stroke) = this.previous_hit_stroke() {
                this.render_label(
                    context,
                    "GetPreviousCaretCharacterHit",
                    Some(previous_hit_stroke),
                    label_x,
                    y,
                    TextAlignment::Left,
                    false,
                );
                this.render_hits(
                    context,
                    &this.previous_hit_pen(),
                    &*text_line,
                    &|hit| text_line.get_previous_caret_character_hit(hit),
                    CharacterHit::new(text_line.length()),
                    &mut y,
                );
                y += VERTICAL_SPACING * 2.0;
            }

            if let Some(backspace_hit_stroke) = this.backspace_hit_stroke() {
                this.render_label(
                    context,
                    "GetBackspaceCaretCharacterHit",
                    Some(backspace_hit_stroke),
                    label_x,
                    y,
                    TextAlignment::Left,
                    false,
                );
                this.render_hits(
                    context,
                    &this.backspace_hit_pen(),
                    &*text_line,
                    &|hit| text_line.get_backspace_caret_character_hit(hit),
                    CharacterHit::new(text_line.length()),
                    &mut y,
                );
                y += VERTICAL_SPACING * 2.0;
            }

            if let Some(distance_stroke) = this.distance_stroke() {
                y += VERTICAL_SPACING;

                let mut label = this
                    .render_label(
                        context,
                        "GetDistanceFromCharacterHit",
                        Some(distance_stroke.clone()),
                        0.0,
                        y,
                        TextAlignment::Left,
                        false,
                    )
                    .expect("a label drawn with a brush");
                y += label.height();

                for i in 0..text_line.length() {
                    let hit = CharacterHit::new(i);
                    let mut prev_hit = CharacterHit::default();
                    let mut next_hit = CharacterHit::default();

                    let mut left_label_x = -HORIZONTAL_SPACING;

                    // we want z-order to be previous, next, distance
                    // but labels need to be ordered next, distance, previous
                    if let Some(next_hit_stroke) = this.next_hit_stroke() {
                        next_hit = text_line.get_next_caret_character_hit(hit);
                        let next_label = this
                            .render_label(
                                context,
                                &format!(" > {}+{}", next_hit.first_character_index(), next_hit.trailing_length()),
                                Some(next_hit_stroke),
                                left_label_x,
                                y,
                                TextAlignment::Right,
                                true,
                            )
                            .expect("a label drawn with a brush");
                        left_label_x -= next_label.width_including_trailing_whitespace();
                    }

                    if this.backspace_hit_stroke().is_some() {
                        let back_hit = text_line.get_backspace_caret_character_hit(hit);
                        let x1 = text_line.get_distance_from_character_hit(CharacterHit::with_trailing_length(
                            back_hit.first_character_index(),
                            0,
                        ));
                        let x2 = text_line.get_distance_from_character_hit(CharacterHit::with_trailing_length(
                            back_hit.first_character_index() + back_hit.trailing_length(),
                            0,
                        ));
                        this.render_horizontal_point(context, x1, x2, y, &this.backspace_hit_pen(), ARROW_SIZE);
                    }

                    if this.previous_hit_stroke().is_some() {
                        prev_hit = text_line.get_previous_caret_character_hit(hit);
                        let x1 = text_line.get_distance_from_character_hit(CharacterHit::with_trailing_length(
                            prev_hit.first_character_index(),
                            0,
                        ));
                        let x2 = text_line.get_distance_from_character_hit(CharacterHit::with_trailing_length(
                            prev_hit.first_character_index() + prev_hit.trailing_length(),
                            0,
                        ));
                        this.render_horizontal_point(context, x1, x2, y, &this.previous_hit_pen(), ARROW_SIZE);
                    }

                    if this.next_hit_stroke().is_some() {
                        let x1 = text_line.get_distance_from_character_hit(CharacterHit::with_trailing_length(
                            next_hit.first_character_index(),
                            0,
                        ));
                        let x2 = text_line.get_distance_from_character_hit(CharacterHit::with_trailing_length(
                            next_hit.first_character_index() + next_hit.trailing_length(),
                            0,
                        ));
                        this.render_horizontal_point(context, x1, x2, y, &this.next_hit_pen(), ARROW_SIZE);
                    }

                    label = this
                        .render_label(
                            context,
                            &format!("[{i}]"),
                            Some(distance_stroke.clone()),
                            left_label_x,
                            y,
                            TextAlignment::Right,
                            false,
                        )
                        .expect("a label drawn with a brush");
                    left_label_x -= label.width_including_trailing_whitespace();

                    if let Some(previous_hit_stroke) = this.previous_hit_stroke() {
                        this.render_label(
                            context,
                            &format!("{}+{} < ", prev_hit.first_character_index(), prev_hit.trailing_length()),
                            Some(previous_hit_stroke),
                            left_label_x,
                            y,
                            TextAlignment::Right,
                            true,
                        );
                    }

                    let distance = text_line.get_distance_from_character_hit(CharacterHit::new(i));
                    this.render_horizontal_bar(context, 0.0, distance, y, &this.distance_pen(), ARROW_SIZE);
                    //RenderLabel(context, distance.ToString("F2"), DistanceStroke, distance + HorizontalSpacing, y, disableCache: true);

                    y += label.height();
                }
            }
        }
        context.pop(transform);
    }
}

impl InteractiveLineControl {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            extent_pen: RefCell::new(None),
            baseline_pen: RefCell::new(None),
            text_bounds_pen: RefCell::new(None),
            run_bounds_pen: RefCell::new(None),
            next_hit_pen: RefCell::new(None),
            previous_hit_pen: RefCell::new(None),
            backspace_hit_pen: RefCell::new(None),
            distance_pen: RefCell::new(None),
            text_run_properties: RefCell::new(None),
            text_paragraph_properties: RefCell::new(None),
            text_source: OnceCell::new(),
            text_line: RefCell::new(None),
            text_layout: RefCell::new(None),
            text_line_size: Cell::new(None),
            ink_size: Cell::new(None),
            text_line_changed: HandlerList::new(),
            feature_collection_subscription: RefCell::new(None),
            labels_cache: RefCell::new(HashMap::new()),
            ink_render_bounds: Cell::new(Rect::default()),
            line_render_bounds: Cell::new(Rect::default()),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn extent_stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::extent_stroke_property())
    }

    pub fn set_extent_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::extent_stroke_property(), value)
    }

    pub fn baseline_stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::baseline_stroke_property())
    }

    pub fn set_baseline_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::baseline_stroke_property(), value)
    }

    pub fn text_bounds_stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::text_bounds_stroke_property())
    }

    pub fn set_text_bounds_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::text_bounds_stroke_property(), value)
    }

    pub fn run_bounds_stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::run_bounds_stroke_property())
    }

    pub fn set_run_bounds_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::run_bounds_stroke_property(), value)
    }

    pub fn next_hit_stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::next_hit_stroke_property())
    }

    pub fn set_next_hit_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::next_hit_stroke_property(), value)
    }

    pub fn backspace_hit_stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::backspace_hit_stroke_property())
    }

    pub fn set_backspace_hit_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::backspace_hit_stroke_property(), value)
    }

    pub fn previous_hit_stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::previous_hit_stroke_property())
    }

    pub fn set_previous_hit_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::previous_hit_stroke_property(), value)
    }

    pub fn distance_stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::distance_stroke_property())
    }

    pub fn set_distance_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::distance_stroke_property(), value)
    }

    /// `pen ??= create()`: the pen of `cache`, created when the cache is empty.
    fn cached_pen(cache: &RefCell<Option<Rc<dyn IPen>>>, create: impl FnOnce() -> Rc<dyn IPen>) -> Rc<dyn IPen> {
        let cached = cache.borrow().clone();
        match cached {
            Some(pen) => pen,
            None => {
                let pen = create();
                *cache.borrow_mut() = Some(pen.clone());
                pen
            }
        }
    }

    /// `new Pen(brush)`: a solid pen one unit thick.
    fn solid_pen(brush: Option<Rc<dyn IBrush>>) -> Rc<dyn IPen> {
        Pen::with_brush(brush, 1.0).into()
    }

    /// `new Pen(brush, dashStyle: DashStyle.Dash)`: a dashed pen one unit thick.
    fn dashed_pen(brush: Option<Rc<dyn IBrush>>) -> Rc<dyn IPen> {
        Pen::with_all(
            brush,
            1.0,
            Some(DashStyle::dash() as Rc<dyn IDashStyle>),
            PenLineCap::Flat,
            PenLineJoin::Miter,
            10.0,
        )
        .into()
    }

    fn extent_pen(&self) -> Rc<dyn IPen> {
        Self::cached_pen(&self.extent_pen, || Self::dashed_pen(self.extent_stroke()))
    }

    fn baseline_pen(&self) -> Rc<dyn IPen> {
        Self::cached_pen(&self.baseline_pen, || Self::solid_pen(self.baseline_stroke()))
    }

    fn text_bounds_pen(&self) -> Rc<dyn IPen> {
        Self::cached_pen(&self.text_bounds_pen, || Self::solid_pen(self.text_bounds_stroke()))
    }

    fn run_bounds_pen(&self) -> Rc<dyn IPen> {
        Self::cached_pen(&self.run_bounds_pen, || Self::dashed_pen(self.run_bounds_stroke()))
    }

    fn next_hit_pen(&self) -> Rc<dyn IPen> {
        Self::cached_pen(&self.next_hit_pen, || Self::solid_pen(self.next_hit_stroke()))
    }

    fn previous_hit_pen(&self) -> Rc<dyn IPen> {
        Self::cached_pen(&self.previous_hit_pen, || Self::solid_pen(self.previous_hit_stroke()))
    }

    fn backspace_hit_pen(&self) -> Rc<dyn IPen> {
        Self::cached_pen(&self.backspace_hit_pen, || Self::solid_pen(self.backspace_hit_stroke()))
    }

    fn distance_pen(&self) -> Rc<dyn IPen> {
        Self::cached_pen(&self.distance_pen, || Self::solid_pen(self.distance_stroke()))
    }

    /// Gets or sets the text to draw.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_owned))
    }

    /// Gets or sets a brush used to paint the control's background.
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value)
    }

    /// Gets or sets the font family used to draw the control's text.
    pub fn font_family(&self) -> FontFamily {
        self.get_value(Self::font_family_property())
    }

    pub fn set_font_family(&self, value: FontFamily) {
        self.set_value(Self::font_family_property(), value)
    }

    /// Gets or sets the font features turned on/off.
    pub fn font_features(&self) -> Option<FontFeatureCollection> {
        self.get_value(Self::font_features_property())
    }

    pub fn set_font_features(&self, value: Option<FontFeatureCollection>) {
        self.set_value(Self::font_features_property(), value)
    }

    /// Gets or sets the size of the control's text in points.
    pub fn font_size(&self) -> f64 {
        self.get_value(Self::font_size_property())
    }

    pub fn set_font_size(&self, value: f64) {
        self.set_value(Self::font_size_property(), value)
    }

    /// Gets or sets the font style used to draw the control's text.
    pub fn font_style(&self) -> FontStyle {
        self.get_value(Self::font_style_property())
    }

    pub fn set_font_style(&self, value: FontStyle) {
        self.set_value(Self::font_style_property(), value)
    }

    /// Gets or sets the font weight used to draw the control's text.
    pub fn font_weight(&self) -> FontWeight {
        self.get_value(Self::font_weight_property())
    }

    pub fn set_font_weight(&self, value: FontWeight) {
        self.set_value(Self::font_weight_property(), value)
    }

    /// Gets or sets the font stretch used to draw the control's text.
    pub fn font_stretch(&self) -> FontStretch {
        self.get_value(Self::font_stretch_property())
    }

    pub fn set_font_stretch(&self, value: FontStretch) {
        self.set_value(Self::font_stretch_property(), value)
    }

    pub fn text_run_properties(&self) -> Rc<GenericTextRunProperties> {
        let cached = self.text_run_properties.borrow().clone();
        match cached {
            Some(text_run_properties) => text_run_properties,
            None => {
                let text_run_properties = Rc::new(self.create_text_run_properties());
                *self.text_run_properties.borrow_mut() = Some(text_run_properties.clone());
                text_run_properties
            }
        }
    }

    pub fn set_text_run_properties(&self, value: Rc<GenericTextRunProperties>) {
        *self.text_run_properties.borrow_mut() = Some(value.clone());
        self.set_current_value(Self::font_family_property(), value.typeface().font_family().clone());
        self.set_current_value(Self::font_features_property(), value.font_features().cloned());
        self.set_current_value(Self::font_size_property(), value.font_rendering_em_size());
        self.set_current_value(Self::font_style_property(), value.typeface().style());
        self.set_current_value(Self::font_weight_property(), value.typeface().weight());
        self.set_current_value(Self::font_stretch_property(), value.typeface().stretch());
    }

    fn create_text_run_properties(&self) -> GenericTextRunProperties {
        let typeface =
            Typeface::with_style(self.font_family(), self.font_style(), self.font_weight(), self.font_stretch());
        GenericTextRunProperties::with_all(
            typeface,
            self.font_size(),
            None,
            Some(Brushes::black() as Rc<dyn IBrush>),
            None,
            BaselineAlignment::Baseline,
            None,
            self.font_features(),
        )
    }

    // TextParagraphProperties

    pub fn text_paragraph_properties(&self) -> Rc<GenericTextParagraphProperties> {
        let cached = self.text_paragraph_properties.borrow().clone();
        match cached {
            Some(text_paragraph_properties) => text_paragraph_properties,
            None => {
                let text_paragraph_properties = Rc::new(self.create_text_paragraph_properties());
                *self.text_paragraph_properties.borrow_mut() = Some(text_paragraph_properties.clone());
                text_paragraph_properties
            }
        }
    }

    pub fn set_text_paragraph_properties(&self, value: Rc<GenericTextParagraphProperties>) {
        *self.text_paragraph_properties.borrow_mut() = None;
        self.set_current_value(Visual::flow_direction_property(), value.flow_direction());
    }

    fn create_text_paragraph_properties(&self) -> GenericTextParagraphProperties {
        GenericTextParagraphProperties::with_all(
            self.flow_direction(),
            TextAlignment::Start,
            false,
            false,
            self.text_run_properties(),
            TextWrapping::NoWrap,
            0.0,
            0.0,
            0.0,
        )
    }

    fn text_source(&self) -> &Rc<TextSource> {
        self.text_source.get().expect("the text source is created by the constructor")
    }

    pub fn text_line(&self) -> Option<Rc<dyn TextLine>> {
        let cached = self.text_line.borrow().clone();
        if cached.is_some() {
            return cached;
        }

        let paragraph_properties: Rc<dyn TextParagraphProperties> = self.text_paragraph_properties();
        let text_line = <dyn TextFormatter>::current().format_line(
            &**self.text_source(),
            0,
            self.bounds().size().width,
            &paragraph_properties,
            None,
        );
        *self.text_line.borrow_mut() = text_line.clone();
        text_line
    }

    pub fn text_layout(&self) -> Rc<TextLayout> {
        let cached = self.text_layout.borrow().clone();
        match cached {
            Some(text_layout) => text_layout,
            None => {
                let text_source: Rc<dyn ITextSource> = self.text_source().clone();
                let paragraph_properties: Rc<dyn TextParagraphProperties> = self.text_paragraph_properties();
                let text_layout = Rc::new(TextLayout::from_text_source(
                    text_source,
                    paragraph_properties,
                    None,
                    f64::INFINITY,
                    f64::INFINITY,
                    0,
                    None,
                ));
                *self.text_layout.borrow_mut() = Some(text_layout.clone());
                text_layout
            }
        }
    }

    fn text_line_size(&self) -> Size {
        match self.text_line_size.get() {
            Some(text_line_size) => text_line_size,
            None => {
                let text_line_size = match self.text_line() {
                    Some(text_line) => Size::new(text_line.width_including_trailing_whitespace(), text_line.height()),
                    None => Size::default(),
                };
                self.text_line_size.set(Some(text_line_size));
                text_line_size
            }
        }
    }

    fn ink_size(&self) -> Size {
        match self.ink_size.get() {
            Some(ink_size) => ink_size,
            None => {
                let ink_size = match self.text_line() {
                    Some(text_line) => Size::new(
                        -text_line.overhang_leading() + text_line.width_including_trailing_whitespace()
                            - text_line.overhang_trailing(),
                        text_line.extent(),
                    ),
                    None => Size::default(),
                };
                self.ink_size.set(Some(ink_size));
                ink_size
            }
        }
    }

    /// The event `TextLineChanged`: raised when the line the control formats is no longer
    /// the line of its text and properties. Disposing the returned handle unsubscribes.
    pub fn text_line_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.text_line_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.text_line_changed.remove(token);
            }
        })
    }

    fn invalidate_text_run_properties(&self) {
        *self.text_run_properties.borrow_mut() = None;
        self.invalidate_text_paragraph_properties();
    }

    fn invalidate_text_paragraph_properties(&self) {
        *self.text_paragraph_properties.borrow_mut() = None;
        self.invalidate_text_line();
    }

    fn invalidate_text_line(&self) {
        *self.text_layout.borrow_mut() = None;
        *self.text_line.borrow_mut() = None;
        self.text_line_size.set(None);
        self.ink_size.set(None);
        self.invalidate_measure();
        self.invalidate_visual();

        let handlers = self.text_line_changed.snapshot();
        for (_, handler) in handlers.iter() {
            handler();
        }
    }

    fn on_feature_collection_changed(&self) {
        self.invalidate_text_run_properties();
    }

    fn get_or_create_label(&self, label: &str, brush: Rc<dyn IBrush>, disable_cache: bool) -> Rc<FormattedText> {
        let cached = self.labels_cache.borrow().get(label).cloned();
        if let Some(text) = cached {
            return text;
        }

        let text = Rc::new(FormattedText::new(
            label,
            CultureInfo::invariant_culture(),
            FlowDirection::LeftToRight,
            Typeface::default(),
            LABEL_FONT_SIZE,
            Some(brush),
        ));

        if !disable_cache {
            self.labels_cache.borrow_mut().insert(label.to_string(), text.clone());
        }

        text
    }

    pub fn ink_render_bounds(&self) -> Rect {
        self.ink_render_bounds.get()
    }

    pub fn line_render_bounds(&self) -> Rect {
        self.line_render_bounds.get()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_label(
        &self,
        context: &mut DrawingContext,
        label: &str,
        brush: Option<Rc<dyn IBrush>>,
        x: f64,
        y: f64,
        alignment: TextAlignment,
        disable_cache: bool,
    ) -> Option<Rc<FormattedText>> {
        let brush = brush?;

        let text = self.get_or_create_label(label, brush, disable_cache);

        if alignment == TextAlignment::Right {
            context.draw_text(&text, Point::new(x - text.width_including_trailing_whitespace(), y - text.height() / 2.0));
        } else {
            context.draw_text(&text, Point::new(x, y - text.height() / 2.0));
        }

        Some(text)
    }

    fn render_hits(
        &self,
        context: &mut DrawingContext,
        hit_pen: &Rc<dyn IPen>,
        text_line: &dyn TextLine,
        next_hit: &dyn Fn(CharacterHit) -> CharacterHit,
        starting_hit: CharacterHit,
        y: &mut f64,
    ) {
        let mut last_hit = starting_hit;
        let mut last_x = text_line.get_distance_from_character_hit(last_hit);
        let mut last_direction = 0.0;
        *y -= VERTICAL_SPACING; // we always start with adding one below

        loop {
            let hit = next_hit(last_hit);
            if hit == last_hit {
                break;
            }

            let x = text_line.get_distance_from_character_hit(hit);
            let direction = sign(x - last_x);

            if direction == 0.0 || last_direction != direction {
                *y += VERTICAL_SPACING;
            }

            if direction == 0.0 {
                self.render_point(context, x, *y, hit_pen, ARROW_SIZE);
            } else {
                self.render_horizontal_arrow(context, last_x, x, *y, hit_pen, ARROW_SIZE);
            }

            last_x = x;
            last_hit = hit;
            last_direction = direction;
        }
    }

    fn render_point(&self, context: &mut DrawingContext, x: f64, y: f64, pen: &Rc<dyn IPen>, _arrow_height: f64) {
        context.draw_ellipse_at(pen.brush().as_ref(), Some(pen), Point::new(x, y), ARROW_SIZE / 2.0, ARROW_SIZE / 2.0);
    }

    fn render_horizontal_point(
        &self,
        context: &mut DrawingContext,
        x_start: f64,
        x_end: f64,
        y: f64,
        pen: &Rc<dyn IPen>,
        size: f64,
    ) {
        let start_cap = PathGeometry::new();
        let start_figure = PathFigure::new();
        start_figure.set_start_point(Point::new(x_start, y - size / 2.0));
        start_figure.set_is_closed(true);
        start_figure.set_is_filled(true);
        let start_segment = ArcSegment::new();
        start_segment.set_size(Size::new(size / 2.0, size / 2.0));
        start_segment.set_point(Point::new(x_start, y + size / 2.0));
        start_segment.set_sweep_direction(SweepDirection::CounterClockwise);
        start_figure.segments().expect("the segments of a new figure").add(start_segment.upcast());
        start_cap.figures().expect("the figures of a new geometry").add(start_figure);

        context.draw_geometry(pen.brush().as_ref(), Some(pen), &start_cap.upcast());

        let end_cap = PathGeometry::new();
        let end_figure = PathFigure::new();
        end_figure.set_start_point(Point::new(x_end, y - size / 2.0));
        end_figure.set_is_closed(true);
        end_figure.set_is_filled(false);
        let end_segment = ArcSegment::new();
        end_segment.set_size(Size::new(size / 2.0, size / 2.0));
        end_segment.set_point(Point::new(x_end, y + size / 2.0));
        end_segment.set_sweep_direction(SweepDirection::Clockwise);
        end_figure.segments().expect("the segments of a new figure").add(end_segment.upcast());
        end_cap.figures().expect("the figures of a new geometry").add(end_figure);

        context.draw_geometry(pen.brush().as_ref(), Some(pen), &end_cap.upcast());
    }

    fn render_horizontal_arrow(
        &self,
        context: &mut DrawingContext,
        x_start: f64,
        x_end: f64,
        y: f64,
        pen: &Rc<dyn IPen>,
        size: f64,
    ) {
        context.draw_line(pen, Point::new(x_start, y), Point::new(x_end, y));
        context.draw_line(pen, Point::new(x_start, y - size / 2.0), Point::new(x_start, y + size / 2.0)); // start cap

        if x_end >= x_start {
            context.draw_geometry(
                pen.brush().as_ref(),
                Some(pen),
                &PolylineGeometry::with_points(
                    [
                        Point::new(x_end - size, y - size / 2.0),
                        Point::new(x_end - size, y + size / 2.0),
                        Point::new(x_end, y),
                    ],
                    true,
                )
                .upcast(),
            );
        } else {
            context.draw_geometry(
                pen.brush().as_ref(),
                Some(pen),
                &PolylineGeometry::with_points(
                    [
                        Point::new(x_end + size, y - size / 2.0),
                        Point::new(x_end + size, y + size / 2.0),
                        Point::new(x_end, y),
                    ],
                    true,
                )
                .upcast(),
            );
        }
    }

    fn render_horizontal_bar(
        &self,
        context: &mut DrawingContext,
        x_start: f64,
        x_end: f64,
        y: f64,
        pen: &Rc<dyn IPen>,
        size: f64,
    ) {
        context.draw_line(pen, Point::new(x_start, y), Point::new(x_end, y));
        context.draw_line(pen, Point::new(x_start, y - size / 2.0), Point::new(x_start, y + size / 2.0)); // start cap
        context.draw_line(pen, Point::new(x_end, y - size / 2.0), Point::new(x_end, y + size / 2.0)); // end cap
    }

    fn render_font_line(&self, context: &mut DrawingContext, y: f64, width: f64, pen: &Rc<dyn IPen>) {
        context.draw_line(pen, Point::new(0.0, y), Point::new(width, y));
    }
}

/// `Math.Sign` of a number as a number: -1, 0 or 1 (`f64::signum` gives 1 for zero).
fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}
