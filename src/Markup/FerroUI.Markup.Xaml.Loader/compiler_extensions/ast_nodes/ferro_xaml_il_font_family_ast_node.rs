//! Port of `CompilerExtensions/AstNodes/FerroXamlIlFontFamilyAstNode.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlLineInfo,
    XamlAstClrTypeReference, XamlAstNode,
};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypes;

/// A `FontFamily` created from its textual form, resolved against the base URI of the document.
///
/// # What a back end has to do (upstream IL)
///
/// Stack effect: pushes one value of type `types.font_family`.
///
/// 1. Load the runtime context (`ldloc context.ContextLocal`) and cast it to the configured
///    URI context provider type (`context.Configuration.TypeMappings.UriContextProvider`, which
///    upstream requires to be set).
/// 2. Call its `get_BaseUri` getter (looked up as `UriContextProvider.GetMethod("get_BaseUri",
///    types.uri, false)`), which yields the document's base `Uri`.
/// 3. Push the string `text`.
/// 4. Create the font family with `types.font_family_constructor_uri_name`
///    (`new FontFamily(Uri baseUri, string name)`).
pub struct FerroXamlIlFontFamilyAstNode {
    base: XamlAstNode,
    types: Rc<FerroXamlIlWellKnownTypes>,
    text: String,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl FerroXamlIlFontFamilyAstNode {
    pub fn new(
        types: &Rc<FerroXamlIlWellKnownTypes>,
        text: &str,
        line_info: &dyn IXamlLineInfo,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            types: types.clone(),
            text: text.to_string(),
            type_: XamlAstClrTypeReference::new(line_info, types.font_family.clone(), false),
        })
    }

    /// `_types` (private upstream; exposed for the back ends).
    pub fn types(&self) -> &Rc<FerroXamlIlWellKnownTypes> {
        &self.types
    }

    /// `_text`: the font family text as written in the document.
    pub fn text(&self) -> &str {
        &self.text
    }
}

xaml_line_info_impl!(FerroXamlIlFontFamilyAstNode, base);

impl IXamlAstNode for FerroXamlIlFontFamilyAstNode {
    xaml_ast_node_members!("FerroXamlIlFontFamilyAstNode", value);
}

impl IXamlAstValueNode for FerroXamlIlFontFamilyAstNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}
