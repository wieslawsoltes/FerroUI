//! Typed values of registered StyleBuilder setters.
use crate::animation::{
    rust_literal, syntax, TargetType, ValueShape, ValueSource, BUILTIN_PROPERTIES,
};
use crate::literal::Literal;
use crate::{catalog, RsxCatalog};
use ferroui_designer_animation::{AnimationValue, ValueKind};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_language::{PropertyInfo, Schema, FRAMEWORK_NAMESPACE};
use ferroui_designer_project::{rsx_mapping::assembly_path, MarkupCatalog};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use syn::Expr;

#[derive(Clone, Debug)]
pub struct RsxStyleField {
    pub name: String,
    pub getter: String,
    pub type_name: String,
    pub(crate) owner: TargetType,
    pub(crate) insertable: bool,
    codec: Codec,
}

#[derive(Clone, Debug)]
enum Codec {
    Native(ValueShape),
    Literal(Literal),
}

#[derive(Debug)]
pub(super) enum SetterValue {
    Native(ValueSource),
    Literal { text: String, span: SourceSpan },
    Resource(String),
}

impl RsxStyleField {
    pub fn accepts_color_resource(&self) -> bool {
        matches!(&self.codec, Codec::Native(shape) if shape.kind() == ValueKind::Color)
    }

    pub fn owner_name(&self) -> &str {
        &self.owner.name
    }

    pub fn kind(&self) -> &'static str {
        match &self.codec {
            Codec::Native(shape) if shape.kind() == ValueKind::Number => "number",
            Codec::Native(shape) if shape.kind() == ValueKind::Color => "color",
            Codec::Literal(Literal::Bool) => "boolean",
            Codec::Literal(Literal::Integer) => "number",
            Codec::Literal(Literal::Enum { .. }) => "choice",
            _ => "text",
        }
    }

    pub fn options(&self) -> &[String] {
        match &self.codec {
            Codec::Literal(Literal::Enum { variants, .. }) => variants,
            _ => &[],
        }
    }

    pub(super) fn from_expression(expression: &Expr, catalog: &RsxCatalog, bindings: &crate::local_rust::LocalRust) -> Option<Self> {
        let Expr::Call(call) = expression else {
            return None;
        };
        if !call.args.is_empty() {
            return None;
        }
        let path = bindings.path(&call.func)?;
        catalog.style_field(&path).cloned()
    }

    pub(super) fn read(&self, source: &str, expression: &Expr, bindings: &crate::local_rust::LocalRust) -> Option<SetterValue> {
        match &self.codec {
            Codec::Native(shape) => shape.read_with(expression, |path| bindings.path(path)).map(SetterValue::Native),
            Codec::Literal(codec) => {
                let span = syntax::span(expression);
                let text = codec.read_rust(expression)?;
                // A multipart expression may contain comments. The native codec
                // edits those per token. Other codecs must retain it in source.
                let mutable = crate::blocks::keep_comments(source.get(span.range())?)
                    .ok()?
                    .is_empty();
                mutable.then_some(SetterValue::Literal { text, span })
            }
        }
    }

    pub(super) fn rust(&self, value: &str) -> Result<String, String> {
        match &self.codec {
            Codec::Native(shape) => rust_literal(
                &AnimationValue::parse(value, shape.kind()).map_err(|error| error.to_string())?,
            ),
            Codec::Literal(codec) => codec
                .rust(value)
                .ok_or_else(|| format!("Enter a {} value for {}", self.type_name, self.name)),
        }
    }
}

impl SetterValue {
    pub(super) fn text(&self) -> String {
        match self {
            Self::Native(value) => value.value.to_xaml(),
            Self::Literal { text, .. } => text.clone(),
            Self::Resource(key) => key.clone(),
        }
    }

    pub(super) fn plan(
        &self,
        field: &RsxStyleField,
        source: &str,
        value: &str,
    ) -> Result<Vec<TextPatch>, String> {
        let mut patches = Vec::new();
        match self {
            Self::Resource(_) => return Err("Choose a resource for this setter".into()),
            Self::Native(authored) => authored.plan(value, &mut patches)?,
            Self::Literal { span, .. } => {
                let mut replacement = field.rust(value)?;
                if let Ok(literal) = syn::parse_str::<syn::LitStr>(&replacement) {
                    replacement = super::string_literal(&source[span.range()], &literal.value());
                }
                patches.push(TextPatch {
                    span: *span,
                    replacement,
                });
            }
        }
        Ok(patches)
    }
}

pub(crate) fn builtins() -> &'static BTreeMap<String, RsxStyleField> {
    static FIELDS: OnceLock<BTreeMap<String, RsxStyleField>> = OnceLock::new();
    FIELDS.get_or_init(|| {
        let native = BUILTIN_PROPERTIES.iter().map(|(getter, name, shape)| {
            let (owner, _) = getter.rsplit_once("::").unwrap();
            let owner = owner.rsplit("::").next().unwrap();
            let type_name = catalog::schema()
                .property(owner, None, name)
                .map_or_else(|| format!("{:?}", shape.kind()), |info| info.type_name);
            field(getter, name, owner, None, type_name, Codec::Native(*shape))
        });
        let additional = [
            (
                "ferroui_controls::TextBlock::text_property",
                "Text",
                "TextBlock",
            ),
            (
                "ferroui_controls::TextBlock::font_size_property",
                "FontSize",
                "TextBlock",
            ),
            (
                "ferroui_base::layout::Layoutable::margin_property",
                "Margin",
                "Layoutable",
            ),
            (
                "ferroui_controls::Border::border_thickness_property",
                "BorderThickness",
                "Border",
            ),
            ("ferroui_controls::Grid::row_property", "Row", "Grid"),
            ("ferroui_controls::Grid::column_property", "Column", "Grid"),
            (
                "ferroui_base::Visual::is_visible_property",
                "IsVisible",
                "Visual",
            ),
        ]
        .into_iter()
        .filter_map(|(getter, name, owner)| {
            let info = catalog::schema().property(owner, None, name)?;
            let codec = codec(&info, &info.type_name)?;
            Some(field(getter, name, owner, None, info.type_name, codec))
        });
        native
            .chain(additional)
            .map(|field| (field.getter.clone(), field))
            .collect()
    })
}

pub(crate) fn project_fields(
    catalog: &MarkupCatalog,
    schema: &Schema,
    package: Option<&str>,
) -> BTreeMap<String, RsxStyleField> {
    let models = catalog.models();
    let mut fields = BTreeMap::new();
    for assembly in models.models() {
        for listed in &assembly.types {
            for member in &listed.registered {
                let owner = models.accessor_type_path(listed, member);
                let canonical = format!("{owner}::{}", member.accessor);
                let Some(property) = catalog.style_property(&canonical) else {
                    continue;
                };
                let class = &property.owner.class.name;
                let namespace = property.owner.namespace();
                let info = schema
                    .property(class, Some(&namespace), property.name)
                    .or_else(|| schema.property(class, Some(FRAMEWORK_NAMESPACE), property.name));
                let Some(info) = info else { continue };
                let Some(codec) = codec(&info, &property.value_type) else {
                    continue;
                };
                let namespace = if matches!(
                    property.owner.assembly.crate_name.as_str(),
                    "ferroui_base" | "ferroui_controls"
                ) {
                    None
                } else {
                    Some(namespace)
                };
                let Some(path) = assembly_path(&canonical, &assembly.crate_name, package) else {
                    continue;
                };
                let getter = path.trim_start_matches("::");
                fields.entry(getter.to_owned()).or_insert_with(|| {
                    let mut field = field(
                        getter,
                        property.name,
                        class,
                        namespace,
                        info.type_name,
                        codec,
                    );
                    field.insertable = member.visibility == "pub"
                        || getter.starts_with("crate::") && member.visibility == "pub(crate)";
                    field
                });
            }
        }
    }
    fields
}

fn field(
    getter: &str,
    name: &str,
    owner: &str,
    namespace: Option<String>,
    type_name: String,
    codec: Codec,
) -> RsxStyleField {
    RsxStyleField {
        getter: getter.into(),
        name: name.into(),
        type_name,
        owner: TargetType {
            name: owner.into(),
            namespace,
        },
        insertable: true,
        codec,
    }
}

fn codec(info: &PropertyInfo, expanded_type: &str) -> Option<Codec> {
    if info.read_only || info.event || info.style_settable == Some(false) {
        return None;
    }
    if info.type_name == "Option<Rc<dyn IBrush>>" {
        return Some(Codec::Native(ValueShape::SOLID_BRUSH));
    }
    if info.type_name == "Option<Rc<dyn ITransform>>" {
        return Some(Codec::Native(ValueShape::TRANSFORM_LIST));
    }
    if let Some(shape) = ValueShape::from_type(expanded_type) {
        return Some(Codec::Native(shape));
    }
    let literal = catalog::property_codec(info)?;
    match literal {
        Literal::EmbeddedImage | Literal::CornerRadius | Literal::BoxShadows | Literal::TransformList => None,
        _ if info.type_name == "Option<bool>" => None,
        _ => Some(Codec::Literal(literal)),
    }
}
