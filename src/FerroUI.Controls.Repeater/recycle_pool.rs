// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::element_factory::ElementFactoryError;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{Control, Panel};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, AttachedProperty, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, WeakRef,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

#[derive(Clone)]
struct OriginTemplate(Rc<OriginTemplateData>);
struct OriginTemplateData {
    weak: Weak<dyn IDataTemplate>,
    retained: RefCell<Option<Rc<dyn IDataTemplate>>>,
}
impl PartialEq for OriginTemplate {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

struct ElementInfo {
    element: Ref<Control>,
    owner: Option<WeakRef<Panel>>,
}

struct TemplatePool {
    template: Weak<dyn IDataTemplate>,
    pool: Ref<RecyclePool>,
}
ferroui_base::ferro_thread_local! {
    // Deviation (winui-repeater/DEVIATIONS.md, WR-06): a weak template-identity registry in
    // place of the attached `PoolInstanceProperty`.
    // FerroUI data templates are interface handles, not dependency objects.
    // The attached pool uses weak template keys; stale entries are released at
    // each template operation and when a wrapper relinquishes its templates.
    static TEMPLATE_POOLS: RefCell<Vec<TemplatePool>> = const { RefCell::new(Vec::new()) };
}

/// Keyed element pool that preserves the owner when the same panel reuses an
/// element and detaches it when a different panel requests it.
#[repr(C)]
pub struct RecyclePool {
    base: FerroObject,
    elements: RefCell<HashMap<String, Vec<ElementInfo>>>,
}

ferro_class! {
    RecyclePool: FerroObject, virtuals RecyclePoolImpl: FerroObjectImpl {
        fn put_element_core(this, element: Ref<Control>, key: &str, owner: Option<Ref<Control>>) -> Result<(), ElementFactoryError>;
        fn try_get_element_core(this, key: &str, owner: Option<Ref<Control>>) -> Result<Option<Ref<Control>>, ElementFactoryError>;
    }
}
ferro_class_info!(RecyclePool {
    new: RecyclePool::new,
    markup: {
        properties: [Count: i32 { get: |pool: &Ref<RecyclePool>| pool.count() as i32 }],
        methods: [
            try fn PutElement(Ref<Control>, String) =>
                |pool: &Ref<RecyclePool>, element: Ref<Control>, key: String| pool.put_element(element, &key),
            try fn PutElement(Ref<Control>, String, Option<Ref<Control>>) =>
                |pool: &Ref<RecyclePool>, element: Ref<Control>, key: String, owner: Option<Ref<Control>>| pool.put_element_with_owner(element, &key, owner),
            try fn TryGetElement(String) -> Option<Ref<Control>> =>
                |pool: &Ref<RecyclePool>, key: String| pool.try_get_element(&key),
            try fn TryGetElement(String, Option<Ref<Control>>) -> Option<Ref<Control>> =>
                |pool: &Ref<RecyclePool>, key: String, owner: Option<Ref<Control>>| pool.try_get_element_with_owner(&key, owner),
            static fn GetReuseKey(Ref<Control>) -> String => |element: Ref<Control>| RecyclePool::get_reuse_key(&element),
            static fn SetReuseKey(Ref<Control>, String) => |element: Ref<Control>, key: String| RecyclePool::set_reuse_key(&element, key),
            static fn GetOriginTemplate(Ref<Control>) -> Option<Rc<dyn IDataTemplate>> =>
                |element: Ref<Control>| RecyclePool::get_origin_template(&element),
            static fn SetOriginTemplate(Ref<Control>, Option<Rc<dyn IDataTemplate>>) =>
                |element: Ref<Control>, template: Option<Rc<dyn IDataTemplate>>| RecyclePool::set_origin_template(&element, template),
            static fn GetPoolInstance(Rc<dyn IDataTemplate>) -> Option<Ref<RecyclePool>> =>
                |template: Rc<dyn IDataTemplate>| RecyclePool::get_pool_instance(&template),
            static fn SetPoolInstance(Rc<dyn IDataTemplate>, Option<Ref<RecyclePool>>) =>
                |template: Rc<dyn IDataTemplate>, pool: Option<Ref<RecyclePool>>| RecyclePool::set_pool_instance(&template, pool),
        ],
    },
});
impl FerroObjectImpl for RecyclePool {}

ferro_properties! {
    impl RecyclePool {
        pub fn reuse_key_property() -> AttachedProperty<String> {
            FerroProperty::register_attached::<RecyclePool, Control, _>("ReuseKey", String::new())
        }
        fn origin_template_property() -> AttachedProperty<Option<OriginTemplate>> {
            FerroProperty::register_attached::<RecyclePool, Control, _>("OriginTemplate", None)
        }
    }
}

impl RecyclePool {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            elements: RefCell::new(HashMap::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn put_element(&self, element: Ref<Control>, key: &str) -> Result<(), ElementFactoryError> {
        self.put_element_core(element, key, None)
    }
    pub fn put_element_with_owner(
        &self,
        element: Ref<Control>,
        key: &str,
        owner: Option<Ref<Control>>,
    ) -> Result<(), ElementFactoryError> {
        self.put_element_core(element, key, owner)
    }
    pub fn try_get_element(&self, key: &str) -> Result<Option<Ref<Control>>, ElementFactoryError> {
        self.try_get_element_core(key, None)
    }
    pub fn try_get_element_with_owner(
        &self,
        key: &str,
        owner: Option<Ref<Control>>,
    ) -> Result<Option<Ref<Control>>, ElementFactoryError> {
        self.try_get_element_core(key, owner)
    }
    pub fn get_reuse_key(element: &Control) -> String {
        element.get_value(Self::reuse_key_property())
    }
    pub fn set_reuse_key(element: &Control, key: impl Into<String>) {
        element.set_value(Self::reuse_key_property(), key.into());
    }
    pub fn get_origin_template(element: &Control) -> Option<Rc<dyn IDataTemplate>> {
        element
            .get_value(Self::origin_template_property())
            .and_then(|template| {
                template
                    .0
                    .retained
                    .borrow()
                    .clone()
                    .or_else(|| template.0.weak.upgrade())
            })
    }
    pub fn set_origin_template(element: &Control, template: Option<Rc<dyn IDataTemplate>>) {
        element.set_value(
            Self::origin_template_property(),
            template.map(|template| {
                OriginTemplate(Rc::new(OriginTemplateData {
                    weak: Rc::downgrade(&template),
                    retained: RefCell::new(Some(template)),
                }))
            }),
        );
    }
    fn release_origin_template(element: &Control) {
        // A realized element owns its origin, including templates created on
        // demand by selectors. Once pooled, retain only the weak identity to
        // break template -> pool -> element -> template cycles.
        if let Some(origin) = element.get_value(Self::origin_template_property()) {
            let retained = origin.0.retained.borrow_mut().take();
            drop(retained);
        }
    }
    pub fn get_pool_instance(template: &Rc<dyn IDataTemplate>) -> Option<Ref<Self>> {
        Self::prune_template_pools();
        TEMPLATE_POOLS.with(|pools| {
            pools
                .borrow()
                .iter()
                .find(|entry| std::ptr::addr_eq(entry.template.as_ptr(), Rc::as_ptr(template)))
                .map(|entry| entry.pool.clone())
        })
    }
    pub fn set_pool_instance(template: &Rc<dyn IDataTemplate>, pool: Option<Ref<Self>>) {
        Self::prune_template_pools();
        let removed = TEMPLATE_POOLS
            .try_with(|pools| {
                let mut pools = pools.borrow_mut();
                let old = pools
                    .iter()
                    .position(|entry| {
                        std::ptr::addr_eq(entry.template.as_ptr(), Rc::as_ptr(template))
                    })
                    .map(|index| pools.remove(index));
                if let Some(pool) = pool {
                    pools.push(TemplatePool {
                        template: Rc::downgrade(template),
                        pool,
                    });
                }
                old
            })
            .unwrap_or_default();
        drop(removed);
    }
    pub(crate) fn prune_template_pools() {
        let removed = TEMPLATE_POOLS
            .try_with(|pools| {
                let mut pools = pools.borrow_mut();
                let mut removed = Vec::new();
                let mut index = 0;
                while index < pools.len() {
                    if pools[index].template.strong_count() == 0 {
                        removed.push(pools.remove(index));
                    } else {
                        index += 1;
                    }
                }
                removed
            })
            .unwrap_or_default();
        drop(removed);
    }
    pub fn count(&self) -> usize {
        self.elements.borrow().values().map(Vec::len).sum()
    }
    fn owner_as_panel(
        owner: Option<Ref<Control>>,
    ) -> Result<Option<Ref<Panel>>, ElementFactoryError> {
        owner
            .map(|owner| {
                owner
                    .cast::<Panel>()
                    .ok_or(ElementFactoryError::InvalidOwner)
            })
            .transpose()
    }
}

impl RecyclePoolImpl for RecyclePool {
    fn put_element_core(
        this: &Self,
        element: Ref<Control>,
        key: &str,
        owner: Option<Ref<Control>>,
    ) -> Result<(), ElementFactoryError> {
        let owner = Self::owner_as_panel(owner)?.map(|panel| panel.downgrade());
        Self::release_origin_template(&element);
        this.elements
            .borrow_mut()
            .entry(key.to_owned())
            .or_default()
            .push(ElementInfo { element, owner });
        Ok(())
    }
    fn try_get_element_core(
        this: &Self,
        key: &str,
        owner: Option<Ref<Control>>,
    ) -> Result<Option<Ref<Control>>, ElementFactoryError> {
        let owner = Self::owner_as_panel(owner)?;
        let info = {
            let mut map = this.elements.borrow_mut();
            let Some(elements) = map.get_mut(key) else {
                return Ok(None);
            };
            if elements.is_empty() {
                return Ok(None);
            }
            // Keep upstream's first matching element rule: an unowned element
            // and an element under this owner have equal preference.
            let index = elements
                .iter()
                .position(|info| {
                    let old_owner = info.owner.as_ref().and_then(WeakRef::upgrade);
                    old_owner.is_none() || old_owner == owner
                })
                .unwrap_or(elements.len() - 1);
            let info = elements.remove(index);
            if elements.is_empty() {
                map.remove(key);
            }
            info
        };
        if let Some(old_owner) = info.owner.as_ref().and_then(WeakRef::upgrade) {
            if owner.as_ref() != Some(&old_owner)
                && !old_owner.children().remove(info.element.clone())
            {
                return Err(ElementFactoryError::MissingChild);
            }
        }
        Ok(Some(info.element))
    }
}
