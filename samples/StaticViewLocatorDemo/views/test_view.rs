//! Port of `Views/TestView` (markup).

use crate::class_prelude::*;
use crate::located_view;

use crate::view_models::TestViewModel;
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use std::rc::Rc;

located_view!(TestView, TestViewModel, test);

fn test(view_model: &Rc<TestViewModel>) -> Element {
    text_block().margin(16.0).text(view_model.message()).text_wrapping(TextWrapping::Wrap).into()
}
