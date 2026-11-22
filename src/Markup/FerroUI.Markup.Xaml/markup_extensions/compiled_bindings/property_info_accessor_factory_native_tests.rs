use super::*;
use ferroui_base::{FerroObject,FerroObjectImpl,FerroProperty,Ref,StyledProperty,instantiate};
use ferroui_base::data::core::{ClrPropertyInfo,Value};
#[repr(C)]
struct NativeBacking {base:FerroObject}
ferroui_base::ferro_class!(NativeBacking:FerroObject);
ferroui_base::ferro_impl_classes!(NativeBacking:FerroObjectImpl);
ferroui_base::ferro_properties! {impl NativeBacking {
    fn number_property()->StyledProperty<i32>{FerroProperty::register::<NativeBacking,_>("Number",1)}
}}
fn source()->(Ref<NativeBacking>,BoxedValue){
    let source=instantiate(NativeBacking{base:FerroObject::construct()});let boxed:BoxedValue=Rc::new(source.clone());(source,boxed)
}
fn property()->Rc<dyn IPropertyInfo>{Rc::new(ClrPropertyInfo::read_write::<Ref<NativeBacking>,Value<i32>>(
    "Number",|v|v.get_value(NativeBacking::number_property())+100,|v,n|v.set_value(NativeBacking::number_property(),n-100),
))}
#[test]
fn compiled_accessor_follows_native_changes_and_keeps_declared_write_publication_order(){
    let (source,boxed)=source();let baseline=source.property_changed_subscriber_count();
    let accessor=PropertyInfoAccessorFactory::create_inpc_property_accessor(&WeakValue::new(&boxed),property());
    let values=Rc::new(RefCell::new(Vec::new()));let capture=values.clone();
    accessor.subscribe(Rc::new(move|v|capture.borrow_mut().push(*v.unwrap().downcast_ref::<i32>().unwrap())));
    source.set_value(NativeBacking::number_property(),2);
    assert_eq!(&*values.borrow(),&[101,102]);
    let input:BoxedValue=Rc::new(109i32);accessor.set_value(Some(&input),BindingPriority::LocalValue).unwrap();
    assert_eq!(&*values.borrow(),&[101,102,109,109]);assert_eq!(source.get_value(NativeBacking::number_property()),9);
    accessor.unsubscribe();assert_eq!(source.property_changed_subscriber_count(),baseline);
    accessor.subscribe(Rc::new(|_|{}));drop(accessor);assert_eq!(source.property_changed_subscriber_count(),baseline);
}
#[test]
fn compiled_native_accessor_keeps_source_weak_and_getter_errors_visible(){
    let (source,boxed)=source();let weak=source.downgrade();
    let property=Rc::new(ClrPropertyInfo::read_only_fallible::<Ref<NativeBacking>,Value<i32>>("Number",|v|{
        let n=v.get_value(NativeBacking::number_property());if n==3{Err(BindingError::message("getter failure"))}else{Ok(n+100)}
    }));
    let accessor=PropertyInfoAccessorFactory::create_inpc_property_accessor(&WeakValue::new(&boxed),property);
    let values=Rc::new(RefCell::new(Vec::new()));let capture=values.clone();
    accessor.subscribe(Rc::new(move|v|capture.borrow_mut().push(v)));
    source.set_value(NativeBacking::number_property(),3);
    assert!(values.borrow().last().unwrap().as_ref().unwrap().downcast_ref::<BindingNotification>().is_some());
    drop(boxed);drop(source);assert!(weak.upgrade().is_none());assert!(accessor.value().is_none());accessor.unsubscribe();
}
