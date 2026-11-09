//! Geometry operations and value properties for reusable XAML consumers.
use ferroboard_core::layouts::KeyRect;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
#[doc(hidden)]
pub struct KeyRectMetadata;
ferroui_base::ferro_markup_type!(struct KeyRectMetadata as "KeyRect" {
 namespace:"FerroBoard.Layouts",this:KeyRect,handles:[KeyRect],
 constructors:[()=>KeyRect::default,(X:f64,Y:f64,Width:f64,Height:f64)=>KeyRect::new],
 properties:[X:f64{get:|r:&KeyRect|r.x},Y:f64{get:|r:&KeyRect|r.y},Width:f64{get:|r:&KeyRect|r.width},Height:f64{get:|r:&KeyRect|r.height},
 Left:f64{get:|r:&KeyRect|r.left()},Top:f64{get:|r:&KeyRect|r.top()},Right:f64{get:|r:&KeyRect|r.right()},Bottom:f64{get:|r:&KeyRect|r.bottom()},CenterX:f64{get:|r:&KeyRect|r.center_x()},CenterY:f64{get:|r:&KeyRect|r.center_y()},IsEmpty:bool{get:|r:&KeyRect|r.is_empty()}],
 methods:[fn Contains(f64,f64)->bool=>(|r:&KeyRect,x,y|r.contains(x,y)),fn Deflate(f64,f64,f64,f64)->KeyRect=>(|r:&KeyRect,l,t,rr,b|r.deflate(l,t,rr,b)),fn CenterDistanceSquared(f64,f64)->f64=>(|r:&KeyRect,x,y|r.center_distance_squared(x,y))]
});
pub(crate) fn register() {
    MarkupType::register_all(&[KeyRectMetadata::MARKUP]);
    ValueTypes::register_global(ValueTypes::register_nullable::<KeyRect>);
}
