use super::{
    json::{self, Node},
    *,
};
use crate::collections::RibbonItems;
use crate::{
    collections::RibbonList,
    text::{ordinal::equals_ascii, trim, whitespace},
    *,
};
use std::{fmt, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonStateErrorKind {
    ArgumentNull,
    Argument,
    Io,
    Unauthorized,
    Canceled,
}
#[derive(Clone, Debug)]
pub struct RibbonStateError {
    pub kind: RibbonStateErrorKind,
    pub parameter: Option<&'static str>,
    pub message: String,
}
impl RibbonStateError {
    pub(super) fn new(
        kind: RibbonStateErrorKind,
        parameter: Option<&'static str>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            parameter,
            message: message.into(),
        }
    }
}
impl fmt::Display for RibbonStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for RibbonStateError {}
pub struct RibbonStateSerializer;
impl RibbonStateSerializer {
    pub fn serialize(
        state: Option<&RibbonState>,
        indented: bool,
    ) -> Result<RibbonString, RibbonStateError> {
        let state = state.ok_or_else(|| {
            RibbonStateError::new(
                RibbonStateErrorKind::ArgumentNull,
                Some("state"),
                "Value cannot be null. (state)",
            )
        })?;
        let node = state_node(state)?;
        Ok(json::render(&node, indented).into())
    }
    pub fn serialize_default(
        state: Option<&RibbonState>,
    ) -> Result<RibbonString, RibbonStateError> {
        Self::serialize(state, true)
    }
    pub fn deserialize(
        text: Option<&RibbonString>,
    ) -> Result<Option<Rc<RibbonState>>, RibbonStateError> {
        let Some(text) = text else { return Ok(None) };
        if text.as_utf16().iter().all(|&c| whitespace(c)) {
            return Ok(None);
        }
        text.to_utf8().map_err(|_| {
            RibbonStateError::new(
                RibbonStateErrorKind::Argument,
                None,
                "Cannot transcode invalid UTF-16 string to UTF-8 JSON text.",
            )
        })?;
        let state = json::parse(text)
            .and_then(|node| match node {
                Node::Null => Ok(None),
                node => read_state(node).map(Some),
            })
            .ok()
            .flatten();
        let Some(state) = state else { return Ok(None) };
        if state.schema_version() > RibbonState::CURRENT_SCHEMA_VERSION {
            return Ok(None);
        }
        state.normalize();
        state.set_schema_version(RibbonState::CURRENT_SCHEMA_VERSION);
        Ok(Some(state))
    }
}
fn str_node(v: Option<RibbonString>) -> Node {
    v.map_or(Node::Null, Node::String)
}
fn put(rows: &mut Vec<(RibbonString, Node)>, key: &str, value: Node) {
    if !matches!(value, Node::Null) {
        rows.push((key.into(), value));
    }
}
fn list_node<T: Clone + PartialEq + Default + 'static>(
    list: Option<RibbonList<T>>,
    map: impl Fn(T) -> Result<Node, RibbonStateError>,
) -> Result<Node, RibbonStateError> {
    list.map_or(Ok(Node::Null), |list| {
        list.iter()
            .map(map)
            .collect::<Result<Vec<_>, _>>()
            .map(Node::Array)
    })
}
fn strings_node(list: Option<RibbonStateStrings>) -> Node {
    list_node(list, |v| Ok(str_node(v))).unwrap()
}
fn enum_node(value: i32, names: &[&str]) -> Node {
    names.get(value as usize).map_or_else(
        || Node::Number(value.to_string()),
        |s| Node::String((*s).into()),
    )
}
const DISPLAY: &[&str] = &["Classic", "Simplified"];
const VISIBILITY: &[&str] = &[
    "AlwaysShow",
    "TabsOnly",
    "FullScreen",
    "PanelButtons",
    "PanelTitles",
];
const DENSITY: &[&str] = &["Comfortable", "Compact", "Touch"];
const POSITION: &[&str] = &["AboveRibbon", "BelowRibbon"];
const MINIMIZE: &[&str] = &["Tabs", "PanelTitles", "PanelButtons", "CycleAll"];
fn state_node(s: &RibbonState) -> Result<Node, RibbonStateError> {
    let mut r = vec![];
    put(
        &mut r,
        "schemaVersion",
        Node::Number(s.schema_version().to_string()),
    );
    put(&mut r, "selectedTabId", str_node(s.selected_tab_id()));
    put(
        &mut r,
        "displayMode",
        enum_node(s.display_mode().raw(), DISPLAY),
    );
    put(
        &mut r,
        "visibilityMode",
        enum_node(s.visibility_mode().raw(), VISIBILITY),
    );
    put(&mut r, "density", enum_node(s.density().raw(), DENSITY));
    put(
        &mut r,
        "quickAccessPosition",
        enum_node(s.quick_access_position().raw(), POSITION),
    );
    put(
        &mut r,
        "isQuickAccessVisible",
        Node::Bool(s.is_quick_access_visible()),
    );
    put(
        &mut r,
        "showQuickAccessLabels",
        Node::Bool(s.show_quick_access_labels()),
    );
    put(
        &mut r,
        "minimizeBehavior",
        enum_node(s.minimize_behavior().raw(), MINIMIZE),
    );
    put(
        &mut r,
        "showGroupCaptions",
        Node::Bool(s.show_group_captions()),
    );
    put(
        &mut r,
        "floatingGroups",
        list_node(s.floating_groups(), |g| {
            g.map_or(Ok(Node::Null), |g| {
                let mut r = vec![];
                put(&mut r, "groupId", str_node(g.group_id()));
                put(&mut r, "x", float_node(g.x())?);
                put(&mut r, "y", float_node(g.y())?);
                Ok(Node::Object(r))
            })
        })?,
    );
    put(
        &mut r,
        "quickAccessItemIds",
        strings_node(s.quick_access_item_ids()),
    );
    put(
        &mut r,
        "customization",
        s.customization()
            .map_or(Ok(Node::Null), |c| custom_node(&c))?,
    );
    put(
        &mut r,
        "recentSearchIds",
        strings_node(s.recent_search_ids()),
    );
    put(&mut r, "recentColors", strings_node(s.recent_colors()));
    Ok(Node::Object(r))
}
fn float_node(value: f64) -> Result<Node, RibbonStateError> {
    if !value.is_finite() {
        return Err(RibbonStateError::new(
            RibbonStateErrorKind::Argument,
            None,
            ".NET number values such as positive and negative infinity cannot be written as valid JSON.",
        ));
    }
    Ok(Node::Number(
        ferroui_system::invariant_numbers::InvariantF64(value).to_string(),
    ))
}
fn custom_node(c: &RibbonCustomization) -> Result<Node, RibbonStateError> {
    let mut r = vec![];
    put(&mut r, "tabOrder", strings_node(c.tab_order()));
    put(
        &mut r,
        "groupOrder",
        c.group_order().map_or(Node::Null, |d| {
            Node::Object(
                d.entries_snapshot()
                    .into_iter()
                    .map(|(k, v)| (k, strings_node(v)))
                    .collect(),
            )
        }),
    );
    put(&mut r, "hiddenTabIds", strings_node(c.hidden_tab_ids()));
    put(&mut r, "hiddenGroupIds", strings_node(c.hidden_group_ids()));
    put(
        &mut r,
        "labels",
        c.labels().map_or(Node::Null, |d| {
            Node::Object(
                d.entries_snapshot()
                    .into_iter()
                    .map(|(k, v)| (k, str_node(v)))
                    .collect(),
            )
        }),
    );
    put(
        &mut r,
        "customTabs",
        list_node(c.custom_tabs(), |t| {
            t.map_or(Ok(Node::Null), |t| {
                let mut r = vec![];
                put(&mut r, "id", str_node(t.id()));
                put(&mut r, "label", str_node(t.label()));
                put(
                    &mut r,
                    "groups",
                    list_node(t.groups(), |g| g.map_or(Ok(Node::Null), |g| group_node(&g)))?,
                );
                Ok(Node::Object(r))
            })
        })?,
    );
    put(
        &mut r,
        "customGroups",
        list_node(c.custom_groups(), |g| {
            g.map_or(Ok(Node::Null), |g| group_node(&g))
        })?,
    );
    Ok(Node::Object(r))
}
fn group_node(g: &RibbonCustomGroup) -> Result<Node, RibbonStateError> {
    let mut r = vec![];
    put(&mut r, "id", str_node(g.id()));
    put(&mut r, "label", str_node(g.label()));
    put(&mut r, "tabId", str_node(g.tab_id()));
    put(&mut r, "itemIds", strings_node(g.item_ids()));
    Ok(Node::Object(r))
}
fn read_list<T: Clone + PartialEq + Default + 'static>(
    value: Node,
    read: impl Fn(Node) -> Result<T, ()>,
) -> Result<Option<RibbonList<T>>, ()> {
    if matches!(value, Node::Null) {
        return Ok(None);
    }
    Ok(Some(RibbonItems::from_values(
        json::array(value)?
            .into_iter()
            .map(read)
            .collect::<Result<Vec<_>, _>>()?,
    )))
}
fn read_strings(value: Node) -> Result<Option<RibbonStateStrings>, ()> {
    read_list(value, json::string)
}
fn read_map<V: Clone>(
    value: Node,
    read: impl Fn(Node) -> Result<V, ()>,
) -> Result<Option<Rc<RibbonStateEntries<V>>>, ()> {
    if matches!(value, Node::Null) {
        return Ok(None);
    }
    let map = RibbonStateEntries::new();
    for (k, v) in json::object(value)? {
        k.to_utf8().map_err(|_| ())?;
        map.insert(k, read(v)?);
    }
    Ok(Some(map))
}
fn field_name(key: &RibbonString) -> Result<String, ()> {
    key.to_utf8().map_err(|_| ())
}
fn read_enum(value: Node, names: &[&str]) -> Result<i32, ()> {
    match value {
        Node::Number(s) => s.parse().map_err(|_| ()),
        Node::String(s) => {
            let s = RibbonString::from(trim(s.as_utf16()));
            s.to_utf8().map_err(|_| ())?;
            if let Some(n) = ferroui_system::invariant_numbers::try_parse_int(
                &s.to_utf8().map_err(|_| ())?,
                ferroui_system::invariant_numbers::NumberStyles::INTEGER,
            ) {
                return Ok(n);
            }
            let mut bits = 0;
            for part in s.as_utf16().split(|&c| c == 44) {
                let part = RibbonString::from(trim(part));
                let index = names
                    .iter()
                    .position(|name| equals_ascii(&part, name))
                    .ok_or(())?;
                bits |= index as i32;
            }
            Ok(bits)
        }
        _ => Err(()),
    }
}
fn read_state(value: Node) -> Result<Rc<RibbonState>, ()> {
    let s = RibbonState::new();
    for (k, v) in json::object(value)? {
        match field_name(&k)?.as_str() {
            "schemaVersion" => s.set_schema_version(json::integer(v)?),
            "selectedTabId" => s.set_selected_tab_id(json::string(v)?),
            "displayMode" => {
                s.set_display_mode(RibbonDisplayMode::from_raw(read_enum(v, DISPLAY)?))
            }
            "visibilityMode" => {
                s.set_visibility_mode(RibbonVisibilityMode::from_raw(read_enum(v, VISIBILITY)?))
            }
            "density" => s.set_density(RibbonDensity::from_raw(read_enum(v, DENSITY)?)),
            "quickAccessPosition" => s.set_quick_access_position(
                RibbonQuickAccessPosition::from_raw(read_enum(v, POSITION)?),
            ),
            "isQuickAccessVisible" => s.set_is_quick_access_visible(json::boolean(v)?),
            "showQuickAccessLabels" => s.set_show_quick_access_labels(json::boolean(v)?),
            "minimizeBehavior" => {
                s.set_minimize_behavior(RibbonMinimizeBehavior::from_raw(read_enum(v, MINIMIZE)?))
            }
            "showGroupCaptions" => s.set_show_group_captions(json::boolean(v)?),
            "floatingGroups" => s.set_floating_groups(read_list(v, |v| {
                if matches!(v, Node::Null) {
                    Ok(None)
                } else {
                    read_float(v).map(Some)
                }
            })?),
            "quickAccessItemIds" => s.set_quick_access_item_ids(read_strings(v)?),
            "customization" => s.set_customization(if matches!(v, Node::Null) {
                None
            } else {
                Some(read_custom(v)?)
            }),
            "recentSearchIds" => s.set_recent_search_ids(read_strings(v)?),
            "recentColors" => s.set_recent_colors(read_strings(v)?),
            _ => {}
        }
    }
    Ok(s)
}
fn read_custom(value: Node) -> Result<Rc<RibbonCustomization>, ()> {
    let c = RibbonCustomization::new();
    for (k, v) in json::object(value)? {
        match field_name(&k)?.as_str() {
            "tabOrder" => c.set_tab_order(read_strings(v)?),
            "groupOrder" => c.set_group_order(read_map(v, read_strings)?),
            "hiddenTabIds" => c.set_hidden_tab_ids(read_strings(v)?),
            "hiddenGroupIds" => c.set_hidden_group_ids(read_strings(v)?),
            "labels" => c.set_labels(read_map(v, json::string)?),
            "customTabs" => c.set_custom_tabs(read_list(v, |v| {
                if matches!(v, Node::Null) {
                    Ok(None)
                } else {
                    read_tab(v).map(Some)
                }
            })?),
            "customGroups" => c.set_custom_groups(read_list(v, |v| {
                if matches!(v, Node::Null) {
                    Ok(None)
                } else {
                    read_group(v).map(Some)
                }
            })?),
            _ => {}
        }
    }
    Ok(c)
}
fn read_tab(value: Node) -> Result<Rc<RibbonCustomTab>, ()> {
    let t = RibbonCustomTab::new();
    for (k, v) in json::object(value)? {
        match field_name(&k)?.as_str() {
            "id" => t.set_id(json::string(v)?),
            "label" => t.set_label(json::string(v)?),
            "groups" => t.set_groups(read_list(v, |v| {
                if matches!(v, Node::Null) {
                    Ok(None)
                } else {
                    read_group(v).map(Some)
                }
            })?),
            _ => {}
        }
    }
    Ok(t)
}
fn read_group(value: Node) -> Result<Rc<RibbonCustomGroup>, ()> {
    let g = RibbonCustomGroup::new();
    for (k, v) in json::object(value)? {
        match field_name(&k)?.as_str() {
            "id" => g.set_id(json::string(v)?),
            "label" => g.set_label(json::string(v)?),
            "tabId" => g.set_tab_id(json::string(v)?),
            "itemIds" => g.set_item_ids(read_strings(v)?),
            _ => {}
        }
    }
    Ok(g)
}
fn read_float(value: Node) -> Result<Rc<RibbonFloatingGroupState>, ()> {
    let g = RibbonFloatingGroupState::new();
    for (k, v) in json::object(value)? {
        match field_name(&k)?.as_str() {
            "groupId" => g.set_group_id(json::string(v)?),
            "x" => g.set_x(json::double(v)?),
            "y" => g.set_y(json::double(v)?),
            _ => {}
        }
    }
    Ok(g)
}
