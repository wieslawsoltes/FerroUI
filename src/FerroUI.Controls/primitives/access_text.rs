use crate::TopLevel;
use crate::{ControlImpl, TextBlock, TextBlockImpl, TextBlockImplExt};
use ferroui_base::input::{AccessKeyHandler, IAccessKeyHandler, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::TextLayout;
use ferroui_base::media::{DrawingContext, IPen, Pen};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, AttachedProperty, FerroObjectExtensions, FerroObjectImpl,
    FerroObject, FerroObjectImplExt, Point, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A text block that displays a character prefixed with an underscore as an
/// access key.
#[repr(C)]
pub struct AccessText {
    base: TextBlock,
    access_key: RefCell<Option<String>>,
    access_keys: RefCell<Option<Rc<dyn IAccessKeyHandler>>>,
}

ferro_class!(AccessText: TextBlock);
ferroui_base::ferro_class_info!(AccessText { new: AccessText::new });

ferroui_base::ferro_impl_classes!(AccessText: StyledElementImpl);
ferroui_base::ferro_impl_classes!(AccessText: LayoutableImpl);
ferroui_base::ferro_impl_classes!(AccessText: InteractiveImpl);
ferroui_base::ferro_impl_classes!(AccessText: InputElementImpl);
impl ControlImpl for AccessText {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::NoneAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for AccessText {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        FerroObjectExtensions::get_observable::<StyledProperty<Option<String>>>(as_object(this), TextBlock::text_property()).subscribe_fn(move |text| {
            if let Some(this) = weak.upgrade() {
                this.text_changed(text.as_deref());
            }
        });
    }
}

impl VisualImpl for AccessText {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let access_keys = TopLevel::get_top_level(Some(this)).and_then(|top_level| top_level.access_key_handler());
        *this.access_keys.borrow_mut() = access_keys.clone();

        if let (Some(access_keys), Some(access_key)) = (access_keys, this.access_key()) {
            if !access_key.is_empty() {
                access_keys.register(&access_key, &this.to_ref().upcast());
            }
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        let access_keys = this.access_keys.borrow().clone();
        if let Some(access_keys) = access_keys {
            if this.access_key().is_some_and(|access_key| !access_key.is_empty()) {
                access_keys.unregister(this);
                *this.access_keys.borrow_mut() = None;
            }
        }
    }
}

impl TextBlockImpl for AccessText {
    /// Renders the `AccessText` to a drawing context.
    fn render_core(this: &Self, context: &mut DrawingContext) {
        Self::parent_render_core(this, context);

        let underscore = this.text().and_then(|text| utf16_index_of_underscore(&text));

        if let Some(underscore) = underscore {
            if this.show_access_key() {
                let rect = this.text_layout().hit_test_text_position(underscore);

                let x1 = rect.left().round();
                let x2 = rect.right().round();
                let y = rect.bottom().round() - 1.5;

                let pen: Rc<dyn IPen> = Pen::with_brush(this.foreground(), 1.0).into();

                context.draw_line(&pen, Point::new(x1, y), Point::new(x2, y));
            }
        }
    }

    fn create_text_layout(this: &Self, text: Option<&str>) -> Rc<TextLayout> {
        let text = Self::remove_access_key_marker(text);
        Self::parent_create_text_layout(this, text.as_deref())
    }
}

fn as_object(object: &FerroObject) -> &FerroObject {
    object
}

/// The index, in UTF-16 code units, of the first underscore of `text`.
fn utf16_index_of_underscore(text: &str) -> Option<i32> {
    text.find('_').map(|index| text[..index].encode_utf16().count() as i32)
}

ferroui_base::ferro_properties! { impl AccessText {
    ferro_property!(
        /// Defines the `ShowAccessKey` attached property.
        pub fn show_access_key_property() -> AttachedProperty<bool> {
            AccessKeyHandler::show_access_key_property().add_owner::<AccessText>()
        }
    );
} }

impl AccessText {
    fn static_constructor() {
        Visual::affects_render::<AccessText>(&[Self::show_access_key_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TextBlock::construct(), access_key: RefCell::new(None), access_keys: RefCell::new(None) }
    }

    /// Initializes a new instance of the `AccessText` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The access key.
    pub fn access_key(&self) -> Option<String> {
        self.access_key.borrow().clone()
    }

    /// A value indicating whether the access key should be underlined.
    pub fn show_access_key(&self) -> bool {
        self.get_value(Self::show_access_key_property())
    }

    pub fn set_show_access_key(&self, value: bool) {
        self.set_value(Self::show_access_key_property(), value)
    }

    /// Removes the first access key marker of a text and unescapes doubled
    /// markers.
    pub fn remove_access_key_marker(text: Option<&str>) -> Option<String> {
        let text = text?;
        let mut text = text.to_owned();

        if !text.is_empty() {
            if let Some(index) = Self::find_access_key_marker(&text) {
                if index < text.len() - 1 {
                    text.remove(index);
                }
            }
            text = text.replace("__", "_");
        }

        Some(text)
    }

    /// The byte index of the first underscore that is not followed by
    /// another one.
    fn find_access_key_marker(text: &str) -> Option<usize> {
        let bytes = text.as_bytes();
        let length = bytes.len();
        let mut start_index = 0;

        while start_index < length {
            let index = start_index + bytes[start_index..].iter().position(|b| *b == b'_')?;
            if index + 1 < length && bytes[index + 1] != b'_' {
                return Some(index);
            }
            start_index = index + 2;
        }

        None
    }

    /// Called when the `Text` property changes.
    fn text_changed(&self, text: Option<&str>) {
        let mut key: Option<String> = None;

        if let Some(text) = text {
            if let Some(underscore) = text.find('_') {
                // The key is the code point after the marker.
                if let Some(rune) = text[underscore + 1..].chars().next() {
                    key = Some(rune.to_string());
                }
            }
        }

        *self.access_key.borrow_mut() = key;

        let access_keys = self.access_keys.borrow().clone();
        if let (Some(access_keys), Some(access_key)) = (access_keys, self.access_key()) {
            if !access_key.is_empty() {
                access_keys.register(&access_key, &self.to_ref().upcast());
            }
        }
    }
}
