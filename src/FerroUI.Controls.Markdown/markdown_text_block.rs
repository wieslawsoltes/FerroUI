//! The control: a `RichTextBlock` whose document is built from its Markdown.

use crate::builder::{self, BuildOptions};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::IImage;
use ferroui_base::reactive::{Disposable, IDisposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, Size, StaticType, StyledElementImpl, StyledProperty, TypeInfo,
    VisualImpl,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_controls::documents::HyperlinkClickEventArgs;
use ferroui_controls::{Border, ControlImpl, RichTextBlock, RichTextBlockImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// Gives the image of an address in the Markdown, or `None` to show its alternative text. The
/// control never loads an address by itself: the text it shows may come from anyone.
pub type MarkdownImageResolver = Rc<dyn Fn(&str) -> Option<Rc<dyn IImage>>>;

/// Shows Markdown (CommonMark with tables, task lists and strikethrough) as rich text.
#[repr(C)]
pub struct MarkdownTextBlock {
    base: RichTextBlock,
    /// The thematic breaks, which take the width the control is measured with.
    rules: RefCell<Vec<Ref<Border>>>,
    image_resolver: RefCell<Option<MarkdownImageResolver>>,
    link_handlers: Rc<HandlerList<dyn Fn(&HyperlinkClickEventArgs)>>,
}

ferro_class!(MarkdownTextBlock: RichTextBlock);
ferroui_base::ferro_class_info!(MarkdownTextBlock {
    new: MarkdownTextBlock::new,
});
ferro_impl_classes!(MarkdownTextBlock: InteractiveImpl, VisualImpl);

impl StyledElementImpl for MarkdownTextBlock {
    // The control is a rich text block to the themes.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <RichTextBlock as StaticType>::TYPE
    }
}

impl FerroObjectImpl for MarkdownTextBlock {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::markdown_property().as_property() {
            this.rebuild();
        }
    }
}

impl ControlImpl for MarkdownTextBlock {}
impl RichTextBlockImpl for MarkdownTextBlock {}
impl ferroui_base::input::InputElementImpl for MarkdownTextBlock {}

impl LayoutableImpl for MarkdownTextBlock {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let width = available_size.width;
        if width.is_finite() {
            for rule in this.rules.borrow().iter() {
                rule.set_width(width.max(0.0));
            }
        }
        Self::parent_measure_override(this, available_size)
    }
}

ferro_properties! {
    impl MarkdownTextBlock {
        /// Defines the `Markdown` property.
        pub fn markdown_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<MarkdownTextBlock, _>("Markdown", None)
        }
    }
}

impl MarkdownTextBlock {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: RichTextBlock::construct(),
            rules: RefCell::new(Vec::new()),
            image_resolver: RefCell::new(None),
            link_handlers: Rc::new(HandlerList::new()),
        }
    }

    /// Creates an empty control.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The Markdown the control shows.
    pub fn markdown(&self) -> Option<String> {
        self.get_value(Self::markdown_property())
    }

    pub fn set_markdown(&self, value: Option<&str>) {
        self.set_value(Self::markdown_property(), value.map(str::to_owned));
    }

    /// Sets what loads the images of the Markdown, and builds the document again. Without a
    /// resolver an image is its alternative text.
    pub fn set_image_resolver(&self, resolver: Option<MarkdownImageResolver>) {
        *self.image_resolver.borrow_mut() = resolver;
        self.rebuild();
    }

    /// Occurs when a link is clicked, before its address is opened. A handler that sets
    /// `Handled` keeps the address from being opened.
    pub fn link_clicked(&self, handler: impl Fn(&HyperlinkClickEventArgs) + 'static) -> Subscription {
        let handlers = self.link_handlers.clone();
        let token = handlers.add(Rc::new(handler));
        let disposable: Rc<dyn IDisposable> = Disposable::create(move || {
            handlers.remove(token);
        });
        Subscription::new(disposable)
    }

    fn rebuild(&self) {
        let blocks = self.blocks();
        blocks.clear();
        let markdown = self.markdown();
        let resolver = self.image_resolver.borrow().clone();
        let options = BuildOptions { image_resolver: resolver, link_handlers: self.link_handlers.clone() };
        let built = builder::build(markdown.as_deref().unwrap_or_default(), &options);
        for block in built.blocks {
            blocks.add(block);
        }
        *self.rules.borrow_mut() = built.rules;
        self.invalidate_measure();
    }
}
