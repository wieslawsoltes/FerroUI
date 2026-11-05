//! Explicit migration policy and application-provided, audited aliases.

/// The presentation namespace used by WinUI documents.
pub const WINUI_PRESENTATION_NAMESPACE: &str =
    "http://schemas.microsoft.com/winfx/2006/xaml/presentation";

/// How WinUI theme lookups are adapted. The default requires an explicit decision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WinUIThemeResourcePolicy {
    /// Report the use; do not silently turn a theme lookup into another lookup.
    #[default]
    Reject,
    /// Use the framework's live resource lookup. The caller supplies the ported dictionaries
    /// and theme scopes; this does not import WinUI resources or emulate Windows settings.
    DynamicResource,
}

/// What the migration does with WinUI's `{x:Bind}` (docs/porting/x-bind.md, X6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WinUIXBindPolicy {
    /// Keep x:Bind as written: the XAML compiler (the run-time loader and the Rust emitter)
    /// compiles it with its x:Bind core, `XBindExtension` of `ferroui-markup-xaml`. The paths,
    /// functions, casts, indexers, converters, substitute values, event bindings, `x:Phase`,
    /// `x:DefaultBindMode`, `x:Load` and `x:DeferLoadStrategy` are WinUI's and are checked with
    /// WinUI's messages. The bindings of a document root are initialized when it is attached to
    /// a visual tree (a `Window` when it is activated), those of a typed template instance when
    /// its data context changes to an item; `XBindings::get_for_root` gives the per-root
    /// `Bindings` (`initialize`, `update`, `stop_tracking`).
    #[default]
    Compile,
    /// Report every x:Bind of the document as an error (`WXAML1007`), for a host that rewrites
    /// x:Bind by hand, for example to `{Binding}`.
    Reject,
}

/// Maps one XML namespace URI, including an application `using:` namespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinUIXamlNamespaceMapping {
    pub source: String,
    pub target: String,
}

impl WinUIXamlNamespaceMapping {
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
        }
    }
}

/// A specific type alias. A type's registered framework identity remains unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinUIXamlTypeMapping {
    pub source_namespace: String,
    pub source_name: String,
    pub target_namespace: String,
    pub target_name: String,
}

impl WinUIXamlTypeMapping {
    pub fn new(
        source_namespace: impl Into<String>,
        source_name: impl Into<String>,
        target_namespace: impl Into<String>,
        target_name: impl Into<String>,
    ) -> Self {
        Self {
            source_namespace: source_namespace.into(),
            source_name: source_name.into(),
            target_namespace: target_namespace.into(),
            target_name: target_name.into(),
        }
    }
}

/// An exact member rename for an explicitly named source type. This does not convert values
/// or assert inherited-member/behavioral compatibility. A caller audits those separately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinUIXamlMemberMapping {
    pub source_namespace: String,
    pub source_type: String,
    pub source_member: String,
    pub target_member: String,
}

impl WinUIXamlMemberMapping {
    pub fn new(
        source_namespace: impl Into<String>,
        source_type: impl Into<String>,
        source_member: impl Into<String>,
        target_member: impl Into<String>,
    ) -> Self {
        Self {
            source_namespace: source_namespace.into(),
            source_type: source_type.into(),
            source_member: source_member.into(),
            target_member: target_member.into(),
        }
    }
}

/// Per-document compatibility policy; never stored globally. Custom mappings take precedence
/// over built-in aliases. Duplicate/conflicting mappings are rejected before any edits.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WinUIXamlOptions {
    pub namespaces: Vec<WinUIXamlNamespaceMapping>,
    pub types: Vec<WinUIXamlTypeMapping>,
    pub members: Vec<WinUIXamlMemberMapping>,
    pub theme_resources: WinUIThemeResourcePolicy,
    pub x_bind: WinUIXBindPolicy,
    /// WinUI `Style` resource keys explicitly known to be ported `ControlTheme` resources.
    pub style_resources: Vec<String>,
    /// Replaces `ms-appx:///` in resource-bearing URI attributes. For example,
    /// `Some("ferres://WinUIGallery/".into())`. Must be an absolute URI ending in `/`.
    pub asset_base_uri: Option<String>,
}

impl WinUIXamlOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Policy for an application that installs `ferroui-themes-winui`. These keys exist as
    /// `ControlTheme` resources in that crate. This call does not install the theme.
    pub fn for_winui_theme() -> Self {
        Self {
            theme_resources: WinUIThemeResourcePolicy::DynamicResource,
            style_resources: [
                "DefaultButtonStyle",
                "AccentButtonStyle",
                "SubtleButtonStyle",
                "BaseTextBlockStyle",
                "CaptionTextBlockStyle",
                "BodyTextBlockStyle",
                "BodyStrongTextBlockStyle",
                "BodyLargeTextBlockStyle",
                "SubtitleTextBlockStyle",
                "TitleTextBlockStyle",
                "TitleLargeTextBlockStyle",
                "DisplayTextBlockStyle",
                "DefaultInfoBadgeStyle",
                "AttentionDotInfoBadgeStyle",
                "AttentionValueInfoBadgeStyle",
                "AttentionIconInfoBadgeStyle",
                "InformationalDotInfoBadgeStyle",
                "InformationalValueInfoBadgeStyle",
                "InformationalIconInfoBadgeStyle",
                "SuccessDotInfoBadgeStyle",
                "SuccessValueInfoBadgeStyle",
                "SuccessIconInfoBadgeStyle",
                "CautionDotInfoBadgeStyle",
                "CautionValueInfoBadgeStyle",
                "CautionIconInfoBadgeStyle",
                "CriticalDotInfoBadgeStyle",
                "CriticalValueInfoBadgeStyle",
                "CriticalIconInfoBadgeStyle",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            ..Self::default()
        }
    }
}
