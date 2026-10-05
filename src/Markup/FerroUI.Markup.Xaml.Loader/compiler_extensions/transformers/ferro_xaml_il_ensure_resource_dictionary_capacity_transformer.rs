//! Port of
//! `CompilerExtensions/Transformers/FerroXamlIlEnsureResourceDictionaryCapacityTransformer.cs`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstManipulationNode, IXamlAstNode, IXamlLineInfo, XamlAstNode, XamlAstNodeExtensions,
    XamlManipulationGroupNode, XamlPropertyAssignmentNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::IXamlMethod;
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use super::{FerroXamlIlWellKnownTypes, FerroXamlIlWellKnownTypesExtensions};

/// Adds a call to EnsureCapacity before adding items to a ResourceDictionary.
#[derive(Default)]
pub struct FerroXamlIlEnsureResourceDictionaryCapacityTransformer {
    /// `HashSet<XamlManipulationGroupNode>` (reference identity): keyed by the node's address,
    /// the node is kept alive so that the address stays unique.
    processed_groups: RefCell<HashMap<usize, Rc<XamlManipulationGroupNode>>>,
}

impl IXamlAstTransformer for FerroXamlIlEnsureResourceDictionaryCapacityTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(group) = node.cast::<XamlManipulationGroupNode>() {
            self.apply(context, &group)?;
        }

        Ok(node)
    }
}

impl FerroXamlIlEnsureResourceDictionaryCapacityTransformer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(
        &self,
        context: &AstTransformationContext,
        group: &Rc<XamlManipulationGroupNode>,
    ) -> XamlResult<()> {
        let types = context.try_get_ferro_types()?;
        let info = self.get_resources_info_group(group, &types);
        if info.mode != ResourcesMode::None && info.count >= 2 {
            let node = EnsureCapacityNode::new(&**group, info.count, info.resources_getter);
            group.children.borrow_mut().insert(0, node);
        }
        Ok(())
    }

    fn get_resources_info(
        &self,
        node: &Rc<dyn IXamlAstManipulationNode>,
        types: &FerroXamlIlWellKnownTypes,
    ) -> ResourcesInfo {
        if let Some(property_assignment) = node.cast::<XamlPropertyAssignmentNode>() {
            Self::get_resources_info_assignment(&property_assignment, types)
        } else if let Some(group) = node.cast::<XamlManipulationGroupNode>() {
            self.get_resources_info_group(&group, types)
        } else {
            ResourcesInfo::default()
        }
    }

    fn get_resources_info_group(
        &self,
        node: &Rc<XamlManipulationGroupNode>,
        types: &FerroXamlIlWellKnownTypes,
    ) -> ResourcesInfo {
        let key = Rc::as_ptr(node) as usize;
        if self
            .processed_groups
            .borrow_mut()
            .insert(key, node.clone())
            .is_some()
        {
            return ResourcesInfo::default();
        }

        let mut group_info = ResourcesInfo::default();

        let children = node.children.borrow().clone();
        for child in &children {
            let child_info = self.get_resources_info(child, types);
            if child_info.mode == ResourcesMode::None {
                continue;
            }

            if group_info.mode == ResourcesMode::None {
                group_info = ResourcesInfo::new(child_info.mode, child_info.resources_getter.clone());
            } else if group_info.mode != child_info.mode
                || !same_getter(&group_info.resources_getter, &child_info.resources_getter)
            {
                return ResourcesInfo::default();
            }

            group_info.count += child_info.count;
        }

        group_info
    }

    fn get_resources_info_assignment(
        node: &XamlPropertyAssignmentNode,
        types: &FerroXamlIlWellKnownTypes,
    ) -> ResourcesInfo {
        match node.property.name().as_str() {
            "Content"
                if node
                    .property
                    .declaring_type()
                    .equals(&*types.resource_dictionary) =>
            {
                ResourcesInfo {
                    count: 1,
                    ..ResourcesInfo::new(ResourcesMode::ResourceDictionaryContent, None)
                }
            }
            "Resources" => match node.property.getter() {
                Some(getter)
                    if types
                        .i_resource_dictionary
                        .is_assignable_from(&*getter.return_type()) =>
                {
                    ResourcesInfo {
                        count: 1,
                        ..ResourcesInfo::new(ResourcesMode::ElementResources, Some(getter))
                    }
                }
                _ => ResourcesInfo::default(),
            },
            _ => ResourcesInfo::default(),
        }
    }
}

/// `groupInfo.ResourcesGetter == childInfo.ResourcesGetter`. Upstream compares the method
/// objects by reference, relying on the type system handing out one object per method; here
/// two handles to an equal method also match.
fn same_getter(a: &Option<Rc<dyn IXamlMethod>>, b: &Option<Rc<dyn IXamlMethod>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b) || a.equals(&**b),
        _ => false,
    }
}

#[derive(Default, Clone)]
struct ResourcesInfo {
    mode: ResourcesMode,
    resources_getter: Option<Rc<dyn IXamlMethod>>,
    count: i32,
}

impl ResourcesInfo {
    fn new(mode: ResourcesMode, resources_getter: Option<Rc<dyn IXamlMethod>>) -> Self {
        Self {
            mode,
            resources_getter,
            count: 0,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum ResourcesMode {
    #[default]
    None,
    ResourceDictionaryContent,
    ElementResources,
}

/// Reserves room in a resource dictionary for the entries the following manipulations add.
///
/// # What a back end has to do (upstream IL)
///
/// `Emit` (stack: the object being initialized; consumes it, leaves nothing):
///
/// 1. when [`EnsureCapacityNode::resources_getter`] is set, call it on the object (the getter
///    of the element's `Resources` property), leaving the resources object; otherwise the
///    object on the stack is the dictionary itself;
/// 2. cast it to `ResourceDictionary` with a type test (`isinst`) and keep the result in a
///    temporary; when it is `null` (not a `ResourceDictionary`) do nothing;
/// 3. otherwise call `types.resource_dictionary_get_count` (`get_Count`) on it, add
///    [`EnsureCapacityNode::capacity`], and call `types.resource_dictionary_ensure_capacity`
///    (`EnsureCapacity(int)`) on it with the sum.
///
/// That is `if (value is ResourceDictionary d) d.EnsureCapacity(d.Count + capacity);`.
pub struct EnsureCapacityNode {
    base: XamlAstNode,
    pub capacity: i32,
    pub resources_getter: Option<Rc<dyn IXamlMethod>>,
}

impl EnsureCapacityNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        capacity: i32,
        resources_getter: Option<Rc<dyn IXamlMethod>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            capacity,
            resources_getter,
        })
    }
}

xaml_line_info_impl!(EnsureCapacityNode, base);

impl IXamlAstNode for EnsureCapacityNode {
    xaml_ast_node_members!("EnsureCapacityNode", manipulation);
}

impl IXamlAstManipulationNode for EnsureCapacityNode {}
