//! Native Rust declarations consumed by the source fact reader, not runtime models.
pub struct Plain;
struct Private;
pub(crate) struct Internal;
pub enum Choice { First, Second }
pub trait IRoot {}
pub trait IDependency: source_dependency::Facade {}
pub trait IChild<T>: IRoot where T: Clone {}
pub struct Generic<'a, T: Clone + 'a, const N: usize = 4> where T: Default {
    value: &'a T,
}
impl<'a,T:Clone+Default+'a,const N:usize> IRoot for Generic<'a,T,N> {}
impl<'a,T:Clone+Default+'a,const N:usize> IChild<T> for Generic<'a,T,N> {}
pub type Alias = Plain;
#[cfg(feature="outer")]
#[path="elsewhere.rs"]
pub mod moved;
pub mod nested {
    pub(super) struct Parent;
    pub(in crate) struct Visible;
    pub mod r#type {pub struct Raw;}
    #[cfg(feature="a")] pub struct Conditional;
    #[cfg(not(feature="a"))] pub struct Conditional;
}
#[cfg(test)] mod tests {struct NotProduction;}
#[cfg(all(feature="x",test))] mod test_path_that_does_not_exist;
macro_rules! generated {()=>{pub struct NotExpanded;};}
generated!();

#[repr(transparent)] pub struct NumericTuple(i32);
#[repr(transparent)] pub struct NumericNamed { storage: i32 }
#[repr(transparent)] pub struct NumericUnsigned(u32);
#[repr(C)] pub struct NumericC(i32);
pub struct NumericDefault(i32);
#[repr(transparent)] pub struct NumericMarker(i32, std::marker::PhantomData<()>);
#[repr(transparent)] pub struct NumericConditional(#[cfg(feature="storage")] i32);
#[cfg_attr(feature="layout", repr(transparent))] pub struct NumericConditionalLayout(i32);
