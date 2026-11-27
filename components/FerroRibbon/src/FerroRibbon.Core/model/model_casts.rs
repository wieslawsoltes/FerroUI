//! Declared projections between ribbon model protocols. General casts use Rust.
use super::*;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
type CastFn<Target> = dyn Fn(Rc<dyn Any>) -> Rc<Target>;
struct TypedCast<Target: ?Sized> {
    cast: Rc<CastFn<Target>>,
}
struct ModelType {
    casts: HashMap<TypeId, Rc<dyn Any>>,
}
#[derive(Default)]
struct Registry {
    types: HashMap<TypeId, ModelType>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelCastError {
    UnregisteredSource(TypeId),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelRegistrationError {
    pub concrete_type: TypeId,
}
impl std::fmt::Display for ModelCastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnregisteredSource(source) => {
                write!(f, "unregistered ribbon model type: {source:?}")
            }
        }
    }
}
impl std::error::Error for ModelCastError {}
impl std::fmt::Display for ModelRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "conflicting ribbon model views for type: {:?}",
            self.concrete_type
        )
    }
}
impl std::error::Error for ModelRegistrationError {}
ferroui_system::ferro_thread_local! {
    static TYPES:RefCell<Registry>=RefCell::default();
    static BUILTINS:Cell<bool>=const{Cell::new(false)};
}
fn entry<T: Any, Target: ?Sized + RibbonModelView>(
    cast: fn(Rc<T>) -> Rc<Target>,
) -> (TypeId, Rc<dyn Any>) {
    let typed = TypedCast::<Target> {
        cast: Rc::new(move |value| cast(value.downcast::<T>().expect("registered concrete type"))),
    };
    (TypeId::of::<Rc<Target>>(), Rc::new(typed))
}
fn register<T: Any>(casts: Vec<(TypeId, Rc<dyn Any>)>) -> Result<(), ModelRegistrationError> {
    ensure_builtins();
    let casts = casts.into_iter().collect::<HashMap<_, _>>();
    TYPES.with(|registry| {
        let mut registry = registry.borrow_mut();
        if let Some(previous) = registry.types.get(&TypeId::of::<T>()) {
            if previous.casts.len() != casts.len()
                || previous.casts.keys().any(|key| !casts.contains_key(key))
            {
                return Err(ModelRegistrationError {
                    concrete_type: TypeId::of::<T>(),
                });
            }
        } else {
            registry
                .types
                .insert(TypeId::of::<T>(), ModelType { casts });
        }
        Ok(())
    })
}
/// Whether a concrete type has a declared Core hierarchy. Non-model payloads
/// are not errors; callers must not guess a hierarchy from a Rust type name.
pub fn is_registered_model_type(source: TypeId) -> bool {
    ensure_builtins();
    TYPES.with(|registry| registry.borrow().types.contains_key(&source))
}

/// Source is the object allocation erased to Any, not a boxed Rc handle. Unknown
/// source declarations fail explicitly; a registered but unrelated model view returns None.
/// Concrete recovery uses `Rc::downcast`; inherited protocols use trait upcasting.
///
/// General runtime casts are outside this model boundary:
/// ```compile_fail
/// use ferroribbon_core::model::{RibbonButtonModel, try_cast_model};
/// let model = RibbonButtonModel::new(None, None);
/// let _ = try_cast_model::<String>(Some(model));
/// ```
pub fn try_cast_model<Target: ?Sized + RibbonModelView>(
    source: Option<Rc<dyn Any>>,
) -> Result<Option<Rc<Target>>, ModelCastError> {
    let Some(source) = source else {
        return Ok(None);
    };
    ensure_builtins();
    let id = source.as_ref().type_id();
    let cast = TYPES.with(|registry| {
        let registry = registry.borrow();
        let model = registry
            .types
            .get(&id)
            .ok_or(ModelCastError::UnregisteredSource(id))?;
        let target = TypeId::of::<Rc<Target>>();
        Ok(model.casts.get(&target).cloned())
    })?;
    Ok(cast.map(|cast| {
        (cast
            .downcast_ref::<TypedCast<Target>>()
            .expect("registered target converter")
            .cast)(source)
    }))
}
pub fn try_cast_node<Target: ?Sized + RibbonModelView>(
    source: Option<Rc<dyn RibbonNodeModelImpl>>,
) -> Result<Option<Rc<Target>>, ModelCastError> {
    try_cast_model(source.map(|value| value as Rc<dyn Any>))
}

mod model_view_private {
    pub trait Sealed {}
}
/// A declared ribbon-model protocol accepted by model projection helpers.
/// Use standard Rust upcasting/downcasting for other reference views.
pub trait RibbonModelView: model_view_private::Sealed + 'static {}
macro_rules! model_views { ($($view:ty),+ $(,)?) => {$(impl model_view_private::Sealed for $view {} impl RibbonModelView for $view {})+}; }
model_views!(
    dyn RibbonBackstageItemModelImpl,
    dyn RibbonBackstageModelImpl,
    dyn RibbonButtonGroupModelImpl,
    dyn RibbonButtonModelImpl,
    dyn RibbonCheckBoxModelImpl,
    dyn RibbonColorPickerModelImpl,
    dyn RibbonComboBoxModelImpl,
    dyn RibbonContextualGroupModelImpl,
    dyn RibbonCustomItemModelImpl,
    dyn RibbonDropDownButtonModelImpl,
    dyn RibbonGalleryItemModelImpl,
    dyn RibbonGalleryModelImpl,
    dyn RibbonGridPickerModelImpl,
    dyn RibbonGroupModelImpl,
    dyn RibbonItemModelImpl,
    dyn RibbonLabelModelImpl,
    dyn RibbonMenuItemModelImpl,
    dyn RibbonModelImpl,
    dyn RibbonNodeModelImpl,
    dyn RibbonRowModelImpl,
    dyn RibbonSegmentModelImpl,
    dyn RibbonSegmentedModelImpl,
    dyn RibbonSeparatorModelImpl,
    dyn RibbonSliderModelImpl,
    dyn RibbonSpinnerModelImpl,
    dyn RibbonSplitButtonModelImpl,
    dyn RibbonTabModelImpl,
    dyn RibbonTextBoxModelImpl,
    dyn RibbonToggleButtonModelImpl
);
pub fn register_ribbon_backstage_item_model<T: RibbonBackstageItemModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonBackstageItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_backstage_model<T: RibbonBackstageModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![entry::<T, dyn RibbonBackstageModelImpl>(|v| v)])
}
pub fn register_ribbon_button_group_model<T: RibbonButtonGroupModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonButtonGroupModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_button_model<T: RibbonButtonModelImpl>() -> Result<(), ModelRegistrationError>
{
    register::<T>(vec![
        entry::<T, dyn RibbonButtonModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_check_box_model<T: RibbonCheckBoxModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonCheckBoxModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_color_picker_model<T: RibbonColorPickerModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonColorPickerModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_combo_box_model<T: RibbonComboBoxModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonComboBoxModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_contextual_group_model<T: RibbonContextualGroupModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonContextualGroupModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_custom_item_model<T: RibbonCustomItemModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonCustomItemModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_drop_down_button_model<T: RibbonDropDownButtonModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonDropDownButtonModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_gallery_item_model<T: RibbonGalleryItemModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonGalleryItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_gallery_model<T: RibbonGalleryModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonGalleryModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_grid_picker_model<T: RibbonGridPickerModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonGridPickerModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_group_model<T: RibbonGroupModelImpl>() -> Result<(), ModelRegistrationError>
{
    register::<T>(vec![
        entry::<T, dyn RibbonGroupModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_item_model<T: RibbonItemModelImpl>() -> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_label_model<T: RibbonLabelModelImpl>() -> Result<(), ModelRegistrationError>
{
    register::<T>(vec![
        entry::<T, dyn RibbonLabelModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_menu_item_model<T: RibbonMenuItemModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonMenuItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_model<T: RibbonModelImpl>() -> Result<(), ModelRegistrationError> {
    register::<T>(vec![entry::<T, dyn RibbonModelImpl>(|v| v)])
}
pub fn register_ribbon_node_model<T: RibbonNodeModelImpl>() -> Result<(), ModelRegistrationError> {
    register::<T>(vec![entry::<T, dyn RibbonNodeModelImpl>(|v| v)])
}
pub fn register_ribbon_row_model<T: RibbonRowModelImpl>() -> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonRowModelImpl>(|v| v),
        entry::<T, dyn RibbonButtonGroupModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_segment_model<T: RibbonSegmentModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonSegmentModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_segmented_model<T: RibbonSegmentedModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonSegmentedModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_separator_model<T: RibbonSeparatorModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonSeparatorModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_slider_model<T: RibbonSliderModelImpl>() -> Result<(), ModelRegistrationError>
{
    register::<T>(vec![
        entry::<T, dyn RibbonSliderModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_spinner_model<T: RibbonSpinnerModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonSpinnerModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_split_button_model<T: RibbonSplitButtonModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonSplitButtonModelImpl>(|v| v),
        entry::<T, dyn RibbonDropDownButtonModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_tab_model<T: RibbonTabModelImpl>() -> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonTabModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_text_box_model<T: RibbonTextBoxModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonTextBoxModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
pub fn register_ribbon_toggle_button_model<T: RibbonToggleButtonModelImpl>()
-> Result<(), ModelRegistrationError> {
    register::<T>(vec![
        entry::<T, dyn RibbonToggleButtonModelImpl>(|v| v),
        entry::<T, dyn RibbonButtonModelImpl>(|v| v),
        entry::<T, dyn RibbonItemModelImpl>(|v| v),
        entry::<T, dyn RibbonNodeModelImpl>(|v| v),
    ])
}
fn ensure_builtins() {
    BUILTINS.with(|initialized| {
        if initialized.replace(true) {
            return;
        }
        register_ribbon_node_model::<RibbonMenuHeaderModel>().expect("consistent sealed node");
        register_ribbon_node_model::<RibbonMenuSeparatorModel>().expect("consistent sealed node");
        register_ribbon_backstage_item_model::<RibbonBackstageItemModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_backstage_model::<RibbonBackstageModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_button_group_model::<RibbonButtonGroupModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_button_model::<RibbonButtonModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_check_box_model::<RibbonCheckBoxModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_color_picker_model::<RibbonColorPickerModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_combo_box_model::<RibbonComboBoxModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_contextual_group_model::<RibbonContextualGroupModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_custom_item_model::<RibbonCustomItemModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_drop_down_button_model::<RibbonDropDownButtonModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_gallery_item_model::<RibbonGalleryItemModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_gallery_model::<RibbonGalleryModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_grid_picker_model::<RibbonGridPickerModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_group_model::<RibbonGroupModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_label_model::<RibbonLabelModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_menu_item_model::<RibbonMenuItemModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_model::<RibbonModel>().expect("consistent built-in model declaration");
        register_ribbon_row_model::<RibbonRowModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_segment_model::<RibbonSegmentModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_segmented_model::<RibbonSegmentedModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_separator_model::<RibbonSeparatorModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_slider_model::<RibbonSliderModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_spinner_model::<RibbonSpinnerModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_split_button_model::<RibbonSplitButtonModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_tab_model::<RibbonTabModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_text_box_model::<RibbonTextBoxModel>()
            .expect("consistent built-in model declaration");
        register_ribbon_toggle_button_model::<RibbonToggleButtonModel>()
            .expect("consistent built-in model declaration");
    });
}
// A class reference viewed through any derived protocol keeps virtual Object.Equals.
macro_rules! node_view_equality { ($($view:ident),+ $(,)?)=>{$(impl PartialEq for dyn $view { fn eq(&self,other:&Self)->bool {self.equals_node(other)} })+}; }
node_view_equality!(
    RibbonButtonGroupModelImpl,
    RibbonButtonModelImpl,
    RibbonCheckBoxModelImpl,
    RibbonComboBoxModelImpl,
    RibbonCustomItemModelImpl,
    RibbonDropDownButtonModelImpl,
    RibbonGridPickerModelImpl,
    RibbonLabelModelImpl,
    RibbonMenuItemModelImpl,
    RibbonRowModelImpl,
    RibbonSegmentedModelImpl,
    RibbonSeparatorModelImpl,
    RibbonSliderModelImpl,
    RibbonSpinnerModelImpl,
    RibbonSplitButtonModelImpl,
    RibbonTextBoxModelImpl,
    RibbonToggleButtonModelImpl
);
