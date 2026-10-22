//! Namespace-aware source edits; no global textual substitutions or XML reserialization.

use std::collections::{BTreeMap, HashSet};
use std::ops::Range;

use ferroui_base::metadata::FERRO_XML_NAMESPACE;
use roxmltree::{Document, Node};

use super::{
    WinUIThemeResourcePolicy, WinUIXBindPolicy, WinUIXamlDiagnostic, WinUIXamlDiagnosticSeverity,
    WinUIXamlEdit, WinUIXamlMigration, WinUIXamlMigrationError, WinUIXamlOptions,
    WinUIXamlSourceSpan, WINUI_PRESENTATION_NAMESPACE,
};

const XAML_NAMESPACE: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
const MC_NAMESPACE: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const CONTROLS_NAMESPACE: &str = "using:FerroUI.Controls";
const PRIMITIVES_NAMESPACE: &str = "using:FerroUI.Controls.Primitives";
const NAVIGATION_NAMESPACE: &str = "using:FerroUI.Navigation";

/// Migrates an explicit WinUI document without changing the normal XAML loader.
///
/// Invalid XML or invalid policy returns `Err`. Unimplemented semantic features produce a
/// reviewable report with error diagnostics; `WinUIXamlMigration::into_xaml` rejects those.
/// Known namespace/name mappings are syntax mappings, not a behavioral-parity assertion.
pub fn migrate_winui_xaml(
    source: &str,
    options: &WinUIXamlOptions,
) -> Result<WinUIXamlMigration, WinUIXamlMigrationError> {
    validate_options(options)?;
    // DTDs/entities are disabled by roxmltree's default options. Migration never fetches assets.
    let document = Document::parse(source).map_err(|error| WinUIXamlMigrationError {
        diagnostics: vec![WinUIXamlDiagnostic {
            code: "WXAML0001",
            severity: WinUIXamlDiagnosticSeverity::Error,
            message: error.to_string(),
            span: WinUIXamlSourceSpan {
                bytes: error_offset(source, error.pos().row, error.pos().col)
                    ..error_offset(source, error.pos().row, error.pos().col),
                line: error.pos().row,
                column: error.pos().col,
            },
        }],
    })?;
    let reserved = document
        .descendants()
        .filter(Node::is_element)
        .flat_map(|node| {
            node.namespaces()
                .filter_map(|ns| ns.name().map(str::to_string))
        })
        .collect();
    let mut migration = Migration {
        source,
        document: &document,
        options,
        edits: Vec::new(),
        diagnostics: Vec::new(),
        prefixes: BTreeMap::new(),
        reserved,
    };
    for node in document.descendants().filter(Node::is_element) {
        migration.element(node);
    }
    if !migration.prefixes.is_empty() {
        let root_name = opening_name(source, document.root_element().range().start);
        let mut declarations = String::new();
        for (namespace, prefix) in &migration.prefixes {
            declarations.push_str(&format!(
                " xmlns:{prefix}=\"{}\"",
                escape_attribute(namespace)
            ));
        }
        migration.edit(root_name.end..root_name.end, declarations);
    }
    migration
        .edits
        .sort_by_key(|edit| (edit.span.bytes.start, edit.span.bytes.end));
    let mut cursor = 0;
    let mut output = String::with_capacity(source.len());
    for edit in &migration.edits {
        if edit.span.bytes.start < cursor {
            return Err(policy_error(
                "Overlapping migration edits; the document was not emitted",
            ));
        }
        output.push_str(&source[cursor..edit.span.bytes.start]);
        output.push_str(&edit.replacement);
        cursor = edit.span.bytes.end;
    }
    output.push_str(&source[cursor..]);
    migration
        .diagnostics
        .sort_by_key(|diagnostic| (diagnostic.span.bytes.start, diagnostic.code));
    Ok(WinUIXamlMigration {
        xaml: output,
        edits: migration.edits,
        diagnostics: migration.diagnostics,
    })
}

struct Migration<'a, 'input> {
    source: &'input str,
    document: &'a Document<'input>,
    options: &'a WinUIXamlOptions,
    edits: Vec<WinUIXamlEdit>,
    diagnostics: Vec<WinUIXamlDiagnostic>,
    prefixes: BTreeMap<String, String>,
    reserved: HashSet<String>,
}

impl Migration<'_, '_> {
    fn span(&self, bytes: Range<usize>) -> WinUIXamlSourceSpan {
        let position = self.document.text_pos_at(bytes.start);
        WinUIXamlSourceSpan {
            bytes,
            line: position.row,
            column: position.col,
        }
    }

    fn diagnostic(
        &mut self,
        code: &'static str,
        error: bool,
        bytes: Range<usize>,
        message: impl Into<String>,
    ) {
        self.diagnostics.push(WinUIXamlDiagnostic {
            code,
            severity: if error {
                WinUIXamlDiagnosticSeverity::Error
            } else {
                WinUIXamlDiagnosticSeverity::Adaptation
            },
            message: message.into(),
            span: self.span(bytes),
        });
    }

    fn edit(&mut self, bytes: Range<usize>, replacement: String) {
        if self.source[bytes.clone()] != replacement {
            self.edits.push(WinUIXamlEdit {
                span: self.span(bytes),
                replacement,
            });
        }
    }

    fn prefix(&mut self, namespace: &str) -> String {
        if let Some(prefix) = self.prefixes.get(namespace) {
            return prefix.clone();
        }
        let mut index = self.prefixes.len();
        let prefix = loop {
            let candidate = format!("fui{index}");
            if self.reserved.insert(candidate.clone()) {
                break candidate;
            }
            index += 1;
        };
        self.prefixes.insert(namespace.to_string(), prefix.clone());
        prefix
    }

    fn namespace(&self, source: &str) -> Option<String> {
        if let Some(mapping) = self
            .options
            .namespaces
            .iter()
            .find(|mapping| mapping.source == source)
        {
            return Some(mapping.target.clone());
        }
        let target = match source {
            WINUI_PRESENTATION_NAMESPACE => FERRO_XML_NAMESPACE,
            "using:Microsoft.UI.Xaml" => "using:FerroUI",
            "using:Microsoft.UI.Xaml.Controls" => "using:FerroUI.Controls",
            "using:Microsoft.UI.Xaml.Controls.Primitives" => "using:FerroUI.Controls.Primitives",
            "using:Microsoft.UI.Xaml.Controls.Presenters" => "using:FerroUI.Controls.Presenters",
            "using:Microsoft.UI.Xaml.Navigation" => NAVIGATION_NAMESPACE,
            "using:Microsoft.UI.Xaml.Shapes" => "using:FerroUI.Controls.Shapes",
            "using:Microsoft.UI.Xaml.Documents" => "using:FerroUI.Controls.Documents",
            "using:Microsoft.UI.Xaml.Media" => "using:FerroUI.Media",
            "using:Microsoft.UI.Xaml.Input" => "using:FerroUI.Input",
            "using:Microsoft.UI.Xaml.Automation" => "using:FerroUI.Automation",
            "using:Microsoft.UI.Xaml.Automation.Peers" => "using:FerroUI.Automation.Peers",
            _ => return None,
        };
        Some(target.to_string())
    }

    fn explicit_type(&self, namespace: &str, name: &str) -> bool {
        self.options
            .types
            .iter()
            .any(|mapping| mapping.source_namespace == namespace && mapping.source_name == name)
    }

    fn type_name(&mut self, namespace: &str, name: &str, original: &str) -> String {
        let mapping = self
            .options
            .types
            .iter()
            .find(|mapping| mapping.source_namespace == namespace && mapping.source_name == name)
            .map(|mapping| {
                (
                    mapping.target_namespace.clone(),
                    mapping.target_name.clone(),
                )
            })
            .or_else(|| {
                (is_control_namespace(namespace) && name == "Page")
                    .then(|| (CONTROLS_NAMESPACE.to_string(), "FramePage".to_string()))
            })
            .or_else(|| {
                ((is_control_namespace(namespace)
                    || namespace == "using:Microsoft.UI.Xaml.Navigation")
                    && matches!(
                        name,
                        "NavigationCacheMode"
                            | "NavigationMode"
                            | "NavigatingCancelEventArgs"
                            | "NavigationEventArgs"
                            | "NavigationFailedEventArgs"
                            | "FrameNavigationOptions"
                            | "PageStackEntry"
                    ))
                .then(|| (NAVIGATION_NAMESPACE.to_string(), name.to_string()))
            })
            .or_else(|| {
                ((is_control_namespace(namespace)
                    || namespace == "using:Microsoft.UI.Xaml.Controls.Primitives")
                    && matches!(
                        name,
                        "InfoBarPanel"
                            | "InfoBarTemplateSettings"
                            | "InfoBadgeTemplateSettings"
                            | "ProgressRingTemplateSettings"
                    ))
                .then(|| (PRIMITIVES_NAMESPACE.to_string(), name.to_string()))
            })
            .or_else(|| {
                (is_control_namespace(namespace)
                    && matches!(
                        name,
                        "Frame"
                            | "NavigationView"
                            | "NavigationViewItemBase"
                            | "NavigationViewItem"
                            | "NavigationViewItemHeader"
                            | "NavigationViewItemSeparator"
                            | "NavigationViewDisplayMode"
                            | "NavigationViewBackButtonVisible"
                            | "NavigationViewPaneDisplayMode"
                            | "NavigationViewSelectionFollowsFocus"
                            | "NavigationViewDisplayModeChangedEventArgs"
                            | "NavigationViewSelectionChangedEventArgs"
                            | "NavigationViewItemInvokedEventArgs"
                            | "NavigationViewPaneClosingEventArgs"
                            | "NavigationViewBackRequestedEventArgs"
                            | "NavigationViewItemExpandingEventArgs"
                            | "NavigationViewItemCollapsedEventArgs"
                            | "NumberBox"
                            | "NumberBoxSpinButtonPlacementMode"
                            | "NumberBoxValidationMode"
                            | "NumberBoxValueChangedEventArgs"
                            | "RadioButtons"
                            | "FontIcon"
                            | "SymbolIcon"
                            | "Symbol"
                            | "InfoBar"
                            | "InfoBarSeverity"
                            | "InfoBarCloseReason"
                            | "InfoBarClosingEventArgs"
                            | "InfoBarClosedEventArgs"
                            | "InfoBarOpenedEventArgs"
                            | "InfoBadge"
                            | "RatingControl"
                            | "RatingItemInfo"
                            | "RatingItemFontInfo"
                            | "RatingItemImageInfo"
                            | "ProgressRing"
                            | "IconSource"
                            | "FontIconSource"
                            | "SymbolIconSource"
                            | "PathIconSource"
                            | "AutoSuggestBox"
                            | "AutoSuggestionBoxTextChangeReason"
                            | "AutoSuggestBoxTextChangedEventArgs"
                            | "AutoSuggestBoxSuggestionChosenEventArgs"
                            | "AutoSuggestBoxQuerySubmittedEventArgs"
                    ))
                .then(|| (CONTROLS_NAMESPACE.to_string(), name.to_string()))
            })
            .or_else(|| {
                (namespace == "using:Microsoft.UI.Xaml.Automation.Peers"
                    && matches!(
                        name,
                        "AutoSuggestBoxAutomationPeer"
                            | "InfoBarAutomationPeer"
                            | "NavigationViewAutomationPeer"
                            | "NavigationViewItemAutomationPeer"
                            | "NumberBoxAutomationPeer"
                            | "ProgressRingAutomationPeer"
                            | "RadioButtonsAutomationPeer"
                            | "RatingControlAutomationPeer"
                    ))
                    .then(|| ("using:FerroUI.Automation.Peers".to_string(), name.to_string()))
            });
        match mapping {
            Some((namespace, name)) => format!("{}:{name}", self.prefix(&namespace)),
            None => original.to_string(),
        }
    }

    fn member_name(&self, namespace: &str, owner: &str, name: &str) -> String {
        if let Some(mapping) = self.options.members.iter().find(|mapping| {
            mapping.source_namespace == namespace
                && mapping.source_type == owner
                && mapping.source_member == name
        }) {
            return mapping.target_member.clone();
        }
        if is_control_namespace(namespace) && owner == "ToggleSwitch" && name == "IsOn" {
            return "IsChecked".to_string();
        }
        name.to_string()
    }

    fn element(&mut self, node: Node<'_, '_>) {
        // Keep design-time markup verbatim. The compiler applies mc:Ignorable; scanning an
        // ignored subtree must not manufacture runtime x:Bind/ThemeResource diagnostics.
        if node.ancestors().filter(Node::is_element).any(|ancestor| {
            ancestor
                .tag_name()
                .namespace()
                .is_some_and(|namespace| ignored_namespace(ancestor, namespace))
        }) {
            return;
        }
        let namespace = node.tag_name().namespace().unwrap_or("");
        let local = node.tag_name().name();
        let opening = opening_name(self.source, node.range().start);
        let original = &self.source[opening.clone()];
        let (owner, member) = local
            .split_once('.')
            .map_or((local, None), |(owner, member)| (owner, Some(member)));
        let original_owner = original
            .rsplit_once('.')
            .map_or(original, |(owner, _)| owner);
        let mut name = self.type_name(namespace, owner, original_owner);
        if let Some(member) = member {
            name.push('.');
            name.push_str(&self.member_name(namespace, owner, member));
        }
        self.edit(opening.clone(), name.clone());
        if let Some(closing) = closing_name(self.source, node.range()) {
            self.edit(closing, name);
        }
        if is_winui_namespace(namespace) && !self.explicit_type(namespace, owner) {
            match owner {
                "Style" | "VisualState" | "VisualStateGroup" | "VisualStateManager" |
                "VisualTransition" | "StateTrigger" | "AdaptiveTrigger" => self.diagnostic(
                    "WXAML1001", true, opening.clone(),
                    format!("{owner} requires a semantic implementation; an equally named framework type is not an alias")),
                _ => {},
            }
        }
        if is_winui_namespace(namespace)
            && owner == "Binding"
            && member.is_none()
            && node.attribute("Mode").is_none()
            && !node
                .children()
                .any(|child| child.is_element() && child.tag_name().name() == "Binding.Mode")
        {
            self.edit(opening.end..opening.end, " Mode=\"OneWay\"".into());
        }
        if is_winui_namespace(namespace)
            && matches!(member, Some("Visibility" | "RequestedTheme" | "Style"))
        {
            self.diagnostic("WXAML1010", true, opening.clone(),
                "This member needs an explicit property-element value adapter; the attribute form supports audited literal/resource cases");
        }

        let start_tag = start_tag(self.source, opening.end);
        for attribute in lexical_attributes(self.source, start_tag) {
            let qname = &self.source[attribute.name.clone()];
            if qname == "xmlns" || qname.starts_with("xmlns:") {
                let prefix = qname.strip_prefix("xmlns:");
                if let Some(uri) = node.lookup_namespace_uri(prefix) {
                    if let Some(target) = self.namespace(uri) {
                        self.edit(attribute.value, escape_attribute(&target));
                    }
                }
                continue;
            }
            let (prefix, attribute_local) = qname
                .split_once(':')
                .map_or((None, qname), |(prefix, name)| (Some(prefix), name));
            let attribute_namespace =
                prefix.and_then(|prefix| node.lookup_namespace_uri(Some(prefix)));
            if attribute_namespace.is_some_and(|namespace| ignored_namespace(node, namespace)) {
                continue;
            }
            let decoded = match attribute_namespace {
                Some(namespace) => node.attribute((namespace, attribute_local)),
                None => node.attribute(attribute_local),
            };
            if let Some(value) = decoded {
                self.attribute(
                    node,
                    namespace,
                    owner,
                    attribute_namespace,
                    attribute_local,
                    &attribute,
                    value,
                );
            }
        }
    }

    fn attribute(
        &mut self,
        node: Node<'_, '_>,
        namespace: &str,
        owner: &str,
        attribute_namespace: Option<&str>,
        local: &str,
        attribute: &LexicalAttribute,
        value: &str,
    ) {
        // WinUI's icon shorthand creates a fresh SymbolIcon. Expand only these audited
        // imported members; adding a parser to the shared IconElement would also change
        // existing native-dialect documents. Property elements work with both back ends.
        if self.symbol_icon_attribute(node, namespace, owner, attribute_namespace, local, attribute, value) {
            return;
        }
        let original_name = &self.source[attribute.name.clone()];
        let mut name = original_name.to_string();
        let mut result = value.to_string();
        if attribute_namespace == Some(MC_NAMESPACE) && local == "ProcessContent" {
            self.diagnostic(
                "WXAML1011",
                true,
                attribute.name.clone(),
                "mc:ProcessContent needs an explicit markup-compatibility adapter",
            );
        }
        let context = is_winui_namespace(namespace)
            || node
                .lookup_namespace_uri(None)
                .is_some_and(is_winui_namespace)
            || self.explicit_type(namespace, owner)
            || self
                .options
                .namespaces
                .iter()
                .any(|mapping| mapping.source == namespace)
            || self.options.members.iter().any(|mapping| {
                mapping.source_namespace == namespace && mapping.source_type == owner
            });
        if attribute_namespace == Some(XAML_NAMESPACE) {
            if context
                && matches!(
                    local,
                    "Bind" | "Load" | "DeferLoadStrategy" | "Phase" | "DefaultBindMode" | "Uid"
                )
            {
                self.diagnostic("WXAML1002", true, attribute.name.clone(),
                    format!("x:{local} requires a WinUI lifecycle, binding, or localization implementation"));
            }
            if local == "DataType" {
                result = self.qualified_type(node, value);
            } else if local == "Class" {
                for mapping in &self.options.namespaces {
                    if let (Some(source), Some(target)) = (
                        mapping.source.strip_prefix("using:"),
                        mapping.target.strip_prefix("using:"),
                    ) {
                        if let Some(class) = value.strip_prefix(&format!("{source}.")) {
                            if !class.contains('.') {
                                result = format!("{target}.{class}");
                                break;
                            }
                        }
                    }
                }
            }
        } else if context
            && (attribute_namespace.is_none()
                || attribute_namespace.is_some_and(is_winui_namespace))
        {
            if let Some((attached_owner, member)) = local.split_once('.') {
                let attached_namespace = attribute_namespace.unwrap_or(namespace);
                let original_owner = original_name
                    .rsplit_once('.')
                    .map_or(attached_owner, |(owner, _)| owner);
                let mapped_owner =
                    self.type_name(attached_namespace, attached_owner, original_owner);
                name = format!(
                    "{mapped_owner}.{}",
                    self.member_name(attached_namespace, attached_owner, member)
                );
                if attached_owner == "VisualStateManager" {
                    self.diagnostic(
                        "WXAML1001",
                        true,
                        attribute.name.clone(),
                        "VisualStateManager requires a semantic implementation",
                    );
                }
            } else if attribute_namespace.is_none() {
                name = self.member_name(namespace, owner, local);
                match local {
                    "TargetType" if !value.starts_with('{') => {
                        result = self.qualified_type(node, value);
                    },
                    "Mode" if owner == "Binding" && value == "Default" => self.diagnostic(
                        "WXAML1009", true, attribute.value.clone(), "Binding Mode=Default must explicitly be OneWay for WinUI"),
                    "Visibility" => match value {
                        "Visible" => { name = "IsVisible".into(); result = "True".into(); },
                        "Collapsed" => { name = "IsVisible".into(); result = "False".into(); },
                        _ => self.diagnostic("WXAML1003", true, attribute.value.clone(),
                            "A dynamic Visibility value needs an explicit Visibility-to-bool converter"),
                    },
                    "RequestedTheme" => match value {
                        "Default" | "Light" | "Dark" => {
                            name = "RequestedThemeVariant".into();
                            let prefix = self.prefix(FERRO_XML_NAMESPACE);
                            let xaml_prefix = self.prefix(XAML_NAMESPACE);
                            result = format!("{{{xaml_prefix}:Static {prefix}:ThemeVariant.{value}}}");
                        },
                        _ => self.diagnostic("WXAML1004", true, attribute.value.clone(),
                            "A dynamic RequestedTheme value needs an explicit ElementTheme-to-ThemeVariant adapter"),
                    },
                    "Style" => {
                        if let Some(key) = simple_resource(value, "StaticResource") {
                            if self.options.style_resources.iter().any(|known| known == key) {
                                name = "Theme".into();
                                self.diagnostic("WXAML2002", false, attribute.name.clone(),
                                    format!("Style '{key}' uses the explicitly registered ported ControlTheme"));
                            } else {
                                self.diagnostic("WXAML1005", true, attribute.value.clone(),
                                    format!("Style '{key}' is not an audited ControlTheme resource; add it to style_resources after porting it"));
                            }
                        } else {
                            self.diagnostic("WXAML1005", true, attribute.value.clone(),
                                "A WinUI Style value requires an audited ControlTheme mapping");
                        }
                    },
                    "CloseButtonStyle" if owner == "InfoBar" && is_control_namespace(namespace) => {
                        if simple_resource(value, "StaticResource").is_some_and(|key|
                            self.options.style_resources.iter().any(|known| known == key)) {
                            self.diagnostic("WXAML2002", false, attribute.value.clone(),
                                "InfoBar.CloseButtonStyle uses an explicitly audited native ControlTheme resource");
                        } else {
                            self.diagnostic("WXAML1005", true, attribute.value.clone(),
                                "InfoBar.CloseButtonStyle requires an audited ControlTheme resource key");
                        }
                    },
                    _ => {},
                }
            }
        }
        if context && value.starts_with('{') && !value.starts_with("{}") {
            // Semantic conversions above own their value. Otherwise rewrite the markup syntax.
            if result == value {
                result = self.extension(node, value, attribute.value.clone(), 0, Some(local));
            }
        }
        if context && matches!(local, "Source" | "UriSource" | "NavigateUri") {
            if let Some(path) = value.strip_prefix("ms-appx:///") {
                if let Some(base) = &self.options.asset_base_uri {
                    result = format!("{base}{path}");
                    self.diagnostic(
                        "WXAML2003",
                        false,
                        attribute.value.clone(),
                        "Packaged URI uses the application's explicit asset base",
                    );
                } else {
                    self.diagnostic("WXAML1006", true, attribute.value.clone(),
                        "ms-appx assets require an explicit asset_base_uri and registered framework assets");
                }
            }
        }
        self.edit(attribute.name.clone(), name);
        // Preserve original entities and quote style unless a semantic value actually changed.
        if result != value {
            self.edit(attribute.value.clone(), escape_attribute(&result));
        }
    }

    fn symbol_icon_attribute(
        &mut self,
        node: Node<'_, '_>,
        namespace: &str,
        owner: &str,
        attribute_namespace: Option<&str>,
        local: &str,
        attribute: &LexicalAttribute,
        value: &str,
    ) -> bool {
        if !is_control_namespace(namespace)
            || attribute_namespace.is_some()
            || !matches!((owner, local), ("NavigationViewItem", "Icon") | ("AutoSuggestBox", "QueryIcon"))
            || self.explicit_type(namespace, owner)
            || self.options.members.iter().any(|mapping| {
                mapping.source_namespace == namespace
                    && mapping.source_type == owner
                    && mapping.source_member == local
            })
            || value.trim_start().starts_with('{')
        {
            return false;
        }
        let opening = opening_name(self.source, node.range().start);
        let tag = start_tag(self.source, opening.end);
        let mapped_owner = self.type_name(namespace, owner, &self.source[opening.clone()]);
        let icon_name = self.type_name(namespace, "SymbolIcon", "SymbolIcon");
        let symbol_member = self.member_name(namespace, "SymbolIcon", "Symbol");
        let child = format!(
            "<{mapped_owner}.{local}><{icon_name} {symbol_member}=\"{}\"/></{mapped_owner}.{local}>",
            escape_attribute(value)
        );
        self.edit(attribute.name.start..attribute.value.end + 1, String::new());
        if self.source.as_bytes().get(tag.end.wrapping_sub(1)) == Some(&b'/') {
            self.edit(tag.end - 1..tag.end + 1, format!(">{child}</{mapped_owner}>"));
        } else {
            self.edit(tag.end + 1..tag.end + 1, child);
        }
        self.diagnostic(
            "WXAML2005",
            false,
            attribute.value.clone(),
            "An icon literal becomes an instance-owned SymbolIcon; the native Symbol parser validates the literal. Resource and binding values retain their native icon-object contract.",
        );
        true
    }

    fn qualified_type(&mut self, node: Node<'_, '_>, value: &str) -> String {
        let (prefix, local) = value
            .split_once(':')
            .map_or((None, value), |(prefix, name)| (Some(prefix), name));
        match node.lookup_namespace_uri(prefix) {
            Some(namespace) => self.type_name(namespace, local, value),
            None => value.to_string(),
        }
    }

    fn extension(
        &mut self,
        node: Node<'_, '_>,
        value: &str,
        span: Range<usize>,
        depth: usize,
        target_member: Option<&str>,
    ) -> String {
        if value.starts_with("{}") {
            return value.to_string();
        }
        if depth >= 64 {
            self.diagnostic(
                "WXAML0002",
                true,
                span,
                "Markup extensions are nested too deeply",
            );
            return value.to_string();
        }
        let Some((name_range, nested, parts)) = extension_parts(value) else {
            self.diagnostic("WXAML0002", true, span, "Malformed markup extension");
            return value.to_string();
        };
        let name = &value[name_range.clone()];
        let (prefix, local) = name
            .split_once(':')
            .map_or((None, name), |(prefix, name)| (Some(prefix), name));
        let namespace = node.lookup_namespace_uri(prefix).unwrap_or("");
        if namespace == XAML_NAMESPACE && local == "Bind" {
            return self.x_bind(node, value, &parts, span, depth, target_member);
        }
        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        for nested_range in nested {
            let nested_value = &value[nested_range.clone()];
            let replacement = self.extension(node, nested_value, span.clone(), depth + 1, None);
            if nested_value != replacement {
                edits.push((nested_range, replacement));
            }
        }
        if namespace == XAML_NAMESPACE && local == "Type" {
            let argument = value[name_range.end..value.len() - 1].trim();
            if !argument.contains([' ', ',', '=']) {
                let mapped = self.qualified_type(node, argument);
                if mapped != argument {
                    let start =
                        name_range.end + value[name_range.end..].find(argument).unwrap_or(0);
                    edits.push((start..start + argument.len(), mapped));
                }
            }
        } else if is_winui_namespace(namespace) {
            match local {
                "ThemeResource" => match self.options.theme_resources {
                    WinUIThemeResourcePolicy::Reject => self.diagnostic("WXAML1008", true, span.clone(),
                        "ThemeResource needs the ported dictionaries and an explicit DynamicResource migration policy"),
                    WinUIThemeResourcePolicy::DynamicResource => {
                        let mapped = if prefix.is_none() { "DynamicResource".to_string() }
                            else { format!("{}:DynamicResource", self.prefix(FERRO_XML_NAMESPACE)) };
                        edits.push((name_range.clone(), mapped));
                        self.diagnostic("WXAML2001", false, span.clone(),
                            "ThemeResource uses the framework's live resource lookup; dictionary contents and theme scope remain application responsibilities");
                    },
                },
                "Binding" => {
                    let mode = parts.iter().find_map(|part| {
                        let text = value[part.clone()].trim();
                        let (name, value) = text.split_once('=')?;
                        (name.trim() == "Mode").then_some(value.trim())
                    });
                    if mode.is_none() {
                        let separator = if value[name_range.end..value.len() - 1].trim().is_empty() { " " } else { ", " };
                        edits.push((value.len() - 1..value.len() - 1, format!("{separator}Mode=OneWay")));
                    } else if mode == Some("Default") {
                        self.diagnostic("WXAML1009", true, span.clone(),
                            "Binding Mode=Default must be made explicit as OneWay to preserve WinUI's binding default");
                    }
                },
                _ => {},
            }
        }
        edits.sort_by_key(|(range, _)| range.start);
        let mut output = value.to_string();
        for (range, replacement) in edits.into_iter().rev() {
            output.replace_range(range, &replacement);
        }
        output
    }

    fn x_bind(
        &mut self,
        node: Node<'_, '_>,
        value: &str,
        parts: &[Range<usize>],
        span: Range<usize>,
        depth: usize,
        target_member: Option<&str>,
    ) -> String {
        if self.options.x_bind == WinUIXBindPolicy::Reject {
            self.diagnostic("WXAML1007", true, span,
                "x:Bind has a typed root source, OneTime default, functions and lifecycle; it cannot be renamed to Binding. The explicit property-path policy requires a WinUIBindings host lifecycle");
            return value.into();
        }
        let unsupported = |this: &mut Self, reason: String| {
            this.diagnostic("WXAML1007", true, span.clone(), reason);
            value.to_string()
        };
        if depth != 0 || target_member.is_none() {
            return unsupported(
                self,
                "Nested x:Bind is outside the implemented property-target contract".into(),
            );
        }
        let member = target_member.unwrap_or_default();
        if super::event_targets::is_event(
            node.tag_name().namespace().unwrap_or(""),
            node.tag_name().name(),
            member,
        ) {
            return unsupported(self, format!("x:Bind on event '{member}' requires generated event-handler dispatch; only property paths are implemented"));
        }
        let parsed = match parse_x_bind(value, parts) {
            Ok(parsed) => parsed,
            Err(message) => return unsupported(self, message),
        };
        let mut template = None;
        for ancestor in node.ancestors().skip(1).filter(Node::is_element) {
            if !ancestor
                .tag_name()
                .namespace()
                .is_some_and(is_winui_namespace)
            {
                continue;
            }
            match ancestor.tag_name().name() {
                "DataTemplate" => { template = Some(ancestor); break; },
                "ControlTemplate" | "ItemsPanelTemplate" => return unsupported(self,
                    "x:Bind inside this template requires an explicit source contract; only typed DataTemplate scopes are implemented".into()),
                _ => {},
            }
        }
        let data_type = if let Some(template) = template {
            let Some(type_name) = template.attribute((XAML_NAMESPACE, "DataType")) else {
                return unsupported(
                    self,
                    "A DataTemplate x:Bind requires a declared x:DataType".into(),
                );
            };
            if type_name.starts_with('{') || type_name.contains([' ', ',', '=']) {
                return unsupported(
                    self,
                    "The DataTemplate x:DataType must be one qualified type name".into(),
                );
            }
            let mapped = self.qualified_type(template, type_name);
            let qualified = if mapped == type_name {
                // The directive's prefix might be shadowed between the template and binding.
                // Capture the declaration's namespace under a new collision-free prefix.
                let (prefix, local) = type_name
                    .split_once(':')
                    .map_or((None, type_name), |(p, n)| (Some(p), n));
                let Some(namespace) = template.lookup_namespace_uri(prefix) else {
                    return unsupported(
                        self,
                        "The DataTemplate x:DataType prefix is not declared".into(),
                    );
                };
                let namespace = self
                    .namespace(namespace)
                    .unwrap_or_else(|| namespace.into());
                format!("{}:{local}", self.prefix(&namespace))
            } else {
                mapped
            };
            let xaml = self.prefix(XAML_NAMESPACE);
            Some(format!("{{{xaml}:Type {qualified}}}"))
        } else {
            None
        };
        let prefix = self.prefix("using:FerroUI.Markup.Xaml.WinUI");
        let location = self.span(span.clone());
        let mut arguments = vec![
            format!("Path={}", parsed.path),
            format!("Mode={}", parsed.mode),
            format!(
                "Scope={}",
                if template.is_some() {
                    "Template"
                } else {
                    "Root"
                }
            ),
            format!("SourceLine={}", location.line),
            format!("SourceColumn={}", location.column),
        ];
        if let Some(data_type) = data_type {
            arguments.push(format!("DataType={data_type}"));
        }
        if let Some(trigger) = parsed.trigger {
            arguments.push(format!("UpdateSourceTrigger={trigger}"));
        }
        // Empty Path is best represented by the parameterless extension constructor: the XAML
        // extension grammar does not accept a missing value after an equals sign.
        if parsed.path.is_empty() {
            arguments.remove(0);
        }
        self.diagnostic("WXAML2004", false, span,
            "x:Bind uses registered property metadata and the root/template WinUIBindings lifecycle; the host must explicitly Initialize after construction, Update as needed and choose unload stopping");
        format!("{{{prefix}:WinUIBind {}}}", arguments.join(", "))
    }
}

struct XBindParts<'a> {
    path: &'a str,
    mode: &'a str,
    trigger: Option<&'a str>,
}

fn parse_x_bind<'a>(value: &'a str, parts: &[Range<usize>]) -> Result<XBindParts<'a>, String> {
    let mut result = XBindParts {
        path: "",
        mode: "OneTime",
        trigger: None,
    };
    let mut seen = HashSet::new();
    for (index, part) in parts.iter().enumerate() {
        let text = value[part.clone()].trim();
        if text.is_empty() && index == 0 {
            continue;
        }
        let (name, value) = match text.split_once('=') {
            Some((name, value)) => (name.trim(), value.trim()),
            None if index == 0 => ("Path", text),
            None => return Err("Malformed x:Bind parameter".into()),
        };
        if !seen.insert(name) {
            return Err(format!("Duplicate x:Bind parameter '{name}'"));
        }
        let value = if value.len() >= 2
            && ((value.starts_with('\'') && value.ends_with('\''))
                || (value.starts_with('"') && value.ends_with('"')))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };
        match name {
            "Path" => result.path = value,
            "Mode" => result.mode = value,
            "UpdateSourceTrigger" => result.trigger = Some(value),
            _ => {
                return Err(format!(
                    "x:Bind parameter '{name}' is not implemented by the property-path adapter"
                ))
            }
        }
    }
    if !result.path.is_empty()
        && !result.path.split('.').all(|part| {
            let mut chars = part.chars();
            chars.next().is_some_and(|c| c == '_' || c.is_alphabetic())
                && chars.all(|c| c == '_' || c.is_alphanumeric())
        })
    {
        return Err("Only dotted x:Bind property paths are implemented; functions, casts, indexers and operators require generated binding support".into());
    }
    if !matches!(result.mode, "OneTime" | "OneWay" | "TwoWay") {
        return Err("x:Bind Mode must be OneTime, OneWay or TwoWay".into());
    }
    if result.mode == "TwoWay" && result.path.is_empty() {
        return Err("A TwoWay x:Bind needs a writable property path".into());
    }
    if let Some(trigger) = result.trigger {
        if result.mode != "TwoWay"
            || !matches!(trigger, "Default" | "PropertyChanged" | "LostFocus")
        {
            return Err("x:Bind UpdateSourceTrigger accepts Default, PropertyChanged or LostFocus only with TwoWay".into());
        }
    }
    Ok(result)
}

fn ignored_namespace(node: Node<'_, '_>, namespace: &str) -> bool {
    node.ancestors().filter(Node::is_element).any(|ancestor| {
        ancestor
            .attribute((MC_NAMESPACE, "Ignorable"))
            .is_some_and(|prefixes| {
                prefixes
                    .split_whitespace()
                    .any(|prefix| ancestor.lookup_namespace_uri(Some(prefix)) == Some(namespace))
            })
    })
}

fn is_winui_namespace(namespace: &str) -> bool {
    namespace == WINUI_PRESENTATION_NAMESPACE
        || namespace == "using:Microsoft.UI.Xaml"
        || namespace.starts_with("using:Microsoft.UI.Xaml.")
}

fn is_control_namespace(namespace: &str) -> bool {
    namespace == WINUI_PRESENTATION_NAMESPACE || namespace == "using:Microsoft.UI.Xaml.Controls"
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn opening_name(source: &str, start: usize) -> Range<usize> {
    let start = start + 1;
    let end = source[start..]
        .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
        .map_or(source.len(), |length| start + length);
    start..end
}

fn closing_name(source: &str, range: Range<usize>) -> Option<Range<usize>> {
    let text = &source[range.clone()];
    if text.ends_with("/>") {
        return None;
    }
    let start = range.start + text.rfind("</")? + 2;
    let end = source[start..range.end].find(|c: char| c.is_ascii_whitespace() || c == '>')? + start;
    Some(start..end)
}

fn start_tag(source: &str, after_name: usize) -> Range<usize> {
    let mut quote = None;
    for (offset, byte) in source.as_bytes()[after_name..].iter().copied().enumerate() {
        match (quote, byte) {
            (Some(expected), found) if expected == found => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (None, b'>') => return after_name..after_name + offset,
            _ => {}
        }
    }
    after_name..source.len()
}

struct LexicalAttribute {
    name: Range<usize>,
    value: Range<usize>,
}

/// The input has already passed XML validation. Unlike roxmltree's compact attribute ranges,
/// this scan also handles very long names/whitespace and namespace declaration source ranges.
fn lexical_attributes(source: &str, range: Range<usize>) -> Vec<LexicalAttribute> {
    let bytes = source.as_bytes();
    let mut cursor = range.start;
    let mut attributes = Vec::new();
    while cursor < range.end {
        while cursor < range.end && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= range.end || bytes[cursor] == b'/' {
            break;
        }
        let start = cursor;
        while cursor < range.end && !bytes[cursor].is_ascii_whitespace() && bytes[cursor] != b'=' {
            cursor += 1;
        }
        let name = start..cursor;
        while cursor < range.end && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b'=') {
            cursor += 1;
        }
        if cursor >= range.end {
            break;
        }
        let quote = bytes[cursor];
        cursor += 1;
        let value_start = cursor;
        while cursor < range.end && bytes[cursor] != quote {
            cursor += 1;
        }
        attributes.push(LexicalAttribute {
            name,
            value: value_start..cursor,
        });
        cursor += 1;
    }
    attributes
}

type ExtensionParts = (Range<usize>, Vec<Range<usize>>, Vec<Range<usize>>);

/// Lexes the outer extension and direct nested extensions. Braces/commas in quoted arguments
/// are data. Full type/member validation still belongs to the normal XAML parser/compiler.
fn extension_parts(value: &str) -> Option<ExtensionParts> {
    let bytes = value.as_bytes();
    if bytes.first() != Some(&b'{') || bytes.last() != Some(&b'}') {
        return None;
    }
    let name_start = 1 + value[1..].len() - value[1..].trim_start().len();
    let name_end = value[name_start..]
        .find(|c: char| c.is_ascii_whitespace() || c == ',' || c == '}')?
        + name_start;
    if name_start == name_end {
        return None;
    }
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0;
    let mut nested_start = 0;
    let mut nested = Vec::new();
    let mut parts = Vec::new();
    let mut part_start = name_end;
    for (offset, byte) in bytes[name_end..bytes.len() - 1].iter().copied().enumerate() {
        let index = name_end + offset;
        if escaped {
            escaped = false;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            continue;
        }
        if let Some(expected) = quote {
            if byte == expected {
                quote = None;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'{' => {
                if depth == 0 {
                    nested_start = index;
                }
                depth += 1;
            }
            b'}' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    nested.push(nested_start..index + 1);
                }
            }
            b',' if depth == 0 => {
                parts.push(part_start..index);
                part_start = index + 1;
            }
            _ => {}
        }
    }
    if quote.is_some() || depth != 0 || escaped {
        return None;
    }
    parts.push(part_start..bytes.len() - 1);
    Some((name_start..name_end, nested, parts))
}

fn simple_resource<'a>(value: &'a str, expected: &str) -> Option<&'a str> {
    let (name, _, _) = extension_parts(value)?;
    if &value[name.clone()] != expected {
        return None;
    }
    let key = value[name.end..value.len() - 1].trim();
    (!key.is_empty() && !key.contains([',', '{', '}', '='])).then_some(key)
}

fn policy_error(message: impl Into<String>) -> WinUIXamlMigrationError {
    WinUIXamlMigrationError {
        diagnostics: vec![WinUIXamlDiagnostic {
            code: "WXAML0003",
            severity: WinUIXamlDiagnosticSeverity::Error,
            message: message.into(),
            span: WinUIXamlSourceSpan {
                bytes: 0..0,
                line: 1,
                column: 1,
            },
        }],
    }
}

fn error_offset(source: &str, line: u32, column: u32) -> usize {
    let mut current_line = 1;
    let mut current_column = 1;
    for (offset, character) in source.char_indices() {
        if current_line == line && current_column == column {
            return offset;
        }
        if character == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
    }
    source.len()
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c == '_' || c.is_alphabetic())
        && chars.all(|c| c == '_' || c.is_alphanumeric())
}

fn validate_options(options: &WinUIXamlOptions) -> Result<(), WinUIXamlMigrationError> {
    let mut namespaces = HashSet::new();
    for mapping in &options.namespaces {
        if mapping.source.is_empty()
            || mapping.target.is_empty()
            || !namespaces.insert(&mapping.source)
        {
            return Err(policy_error(
                "Namespace mappings must have nonempty, unique source URIs and nonempty targets",
            ));
        }
    }
    let mut types = HashSet::new();
    for mapping in &options.types {
        if mapping.source_namespace.is_empty()
            || mapping.target_namespace.is_empty()
            || !valid_name(&mapping.source_name)
            || !valid_name(&mapping.target_name)
            || !types.insert((&mapping.source_namespace, &mapping.source_name))
        {
            return Err(policy_error("Type mappings require valid local names, nonempty namespaces and unique source identities"));
        }
    }
    let mut members = HashSet::new();
    for mapping in &options.members {
        if mapping.source_namespace.is_empty()
            || !valid_name(&mapping.source_type)
            || !valid_name(&mapping.source_member)
            || !valid_name(&mapping.target_member)
            || !members.insert((
                &mapping.source_namespace,
                &mapping.source_type,
                &mapping.source_member,
            ))
        {
            return Err(policy_error(
                "Member mappings require valid local names and unique source identities",
            ));
        }
    }
    if options.asset_base_uri.as_ref().is_some_and(|base| {
        ferroui_base::utilities::Uri::try_create(base, ferroui_base::utilities::UriKind::Absolute)
            .is_none()
            || !base.ends_with('/')
            || base.contains(['?', '#'])
            || base.chars().any(char::is_whitespace)
    }) {
        return Err(policy_error(
            "asset_base_uri must be an absolute URI ending in '/' without a query or fragment",
        ));
    }
    Ok(())
}
