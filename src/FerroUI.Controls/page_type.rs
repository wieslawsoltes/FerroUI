//! The type of a page a [`Frame`](crate::Frame) navigates to: the `TypeName` that WinUI's
//! `Frame.Navigate` takes (`System.Type` in C#), as a typed Rust value (B3 of #612).

use crate::navigation::NavigationError;
use crate::FramePage;
use ferroui_base::{DefaultConstructible, ObjectType, Ref, TypeIdentity, TypeInfo, Upcast};
use std::fmt;
use std::hash::{Hash, Hasher};

/// The type of a page a [`Frame`](crate::Frame) navigates to (WinUI's `SourcePageType`): the
/// identity of a class deriving from [`FramePage`] and how the frame creates a page of it.
///
/// Made with [`PageType::of`], which names the page class statically: the compiler checks that
/// the class is a page with a default constructor, and the constructor is reached through the
/// class, not through its [`TypeInfo`], so an application links the constructors of the pages it
/// navigates to and no others. A page class that has no default constructor and whose pages a page
/// factory of the frame creates (`Frame::register_page_factory`) is named with
/// [`PageType::for_factory`].
///
/// ```ignore
/// frame.navigate(PageType::of::<SettingsPage>(), None)?;
/// ```
///
/// Two page types are equal when they name the same class, whichever way they were made; the hash
/// follows. The conversion from a `&'static TypeInfo` ([`From`], for the run-time XAML loader, the
/// designer and the tools) creates a page by reflection, through the default constructor the
/// [`TypeInfo`] carries; it stays in 0.5 for compatibility and is removed with the other deprecated
/// forms (#625).
#[derive(Clone, Copy)]
pub struct PageType {
    form: Form,
}

#[derive(Clone, Copy)]
enum Form {
    /// Named statically ([`PageType::of`], [`PageType::for_factory`]): the identity of the class
    /// and its default constructor, if the page type has one.
    Static { identity: &'static TypeIdentity, create: Option<fn() -> Ref<FramePage>> },
    /// Converted from the runtime type of a class: created by reflection.
    Reflective(&'static TypeInfo),
}

/// Creates a page of `T` with its default constructor.
fn create_page<T: DefaultConstructible + Upcast<FramePage>>() -> Ref<FramePage> {
    T::new_instance().upcast::<FramePage>()
}

impl PageType {
    /// The page type of the page class `T`: a class deriving from [`FramePage`] that declares a
    /// default constructor (`new` of `ferro_class_info!`).
    pub const fn of<T: ObjectType + DefaultConstructible + Upcast<FramePage>>() -> Self {
        Self { form: Form::Static { identity: T::IDENTITY, create: Some(create_page::<T> as fn() -> Ref<FramePage>) } }
    }

    /// The page type of the page class `T` without a constructor: the pages of `T` are created
    /// by the page factory a frame registers for it (`Frame::register_page_factory`), and a
    /// navigation of a frame that has none fails with [`NavigationError::MissingConstructor`].
    pub const fn for_factory<T: ObjectType + Upcast<FramePage>>() -> Self {
        Self { form: Form::Static { identity: T::IDENTITY, create: None } }
    }

    /// The page type of the runtime type `type_`, created by reflection; see
    /// [`from_type_info`](Self::from_type_info).
    #[inline]
    const fn reflective(type_: &'static TypeInfo) -> Self {
        Self { form: Form::Reflective(type_) }
    }

    /// The page type of the runtime type `type_`. A page of it is created through the default
    /// constructor its [`TypeInfo`] carries, which builds without the type tables may not keep;
    /// a type that is not a page fails when the frame navigates to it
    /// ([`NavigationError::NotAPage`]).
    #[deprecated(
        since = "0.5.0",
        note = "use `PageType::of::<T>()`, which reaches the constructor of the page statically"
    )]
    pub const fn from_type_info(type_: &'static TypeInfo) -> Self {
        Self::reflective(type_)
    }

    /// The identity of the class ([`TypeIdentity`]); `None` for a runtime type that is not a class
    /// (the conversion from a `&'static TypeInfo` of a static type).
    #[inline]
    pub fn identity(&self) -> Option<&'static TypeIdentity> {
        match self.form {
            Form::Static { identity, .. } => Some(identity),
            Form::Reflective(type_) => type_.identity(),
        }
    }

    /// The name of the class, without namespace.
    pub fn name(&self) -> &'static str {
        match self.form {
            Form::Static { identity, .. } => identity.name(),
            Form::Reflective(type_) => type_.name(),
        }
    }

    /// Whether this is the page type of the class `T`.
    #[inline]
    pub fn is<T: ObjectType>(&self) -> bool {
        self.identity().is_some_and(|identity| std::ptr::eq(identity, T::IDENTITY))
    }

    /// Whether `object_type`, the runtime type of an object, is this class or derives from it.
    pub fn is_assignable_from(&self, object_type: &'static TypeInfo) -> bool {
        match self.identity() {
            Some(identity) => object_type.derives_from(identity),
            None => match self.form {
                Form::Reflective(type_) => type_.is_assignable_from(object_type),
                Form::Static { .. } => false,
            },
        }
    }

    /// The name used in the errors of a navigation: the namespace-qualified name for a page type
    /// converted from a runtime type, the name of the class otherwise.
    pub(crate) fn display_name(&self) -> String {
        match self.form {
            Form::Static { identity, .. } => identity.name().to_string(),
            Form::Reflective(type_) => type_.full_name(),
        }
    }

    /// Whether the type derives from [`FramePage`]: always for [`PageType::of`], checked for a
    /// page type converted from a runtime type.
    pub(crate) fn check_page(&self) -> Result<(), NavigationError> {
        match self.form {
            Form::Static { .. } => Ok(()),
            Form::Reflective(type_) if type_.derives_from(<FramePage as ObjectType>::IDENTITY) => Ok(()),
            Form::Reflective(_) => Err(NavigationError::NotAPage(self.display_name())),
        }
    }

    /// Creates a page of the type with its default constructor. A page type of
    /// [`PageType::of`] always creates one, one of [`PageType::for_factory`] never does
    /// ([`NavigationError::MissingConstructor`]); one converted from a runtime type fails with
    /// [`NavigationError::NotAPage`] for a type that is not a page and with
    /// [`NavigationError::MissingConstructor`] for a type without a default constructor.
    pub fn create(&self) -> Result<Ref<FramePage>, NavigationError> {
        match self.form {
            Form::Static { create: Some(create), .. } => Ok(create()),
            Form::Static { create: None, .. } => Err(NavigationError::MissingConstructor(self.display_name())),
            Form::Reflective(type_) => {
                self.check_page()?;
                type_
                    .create_instance()
                    .ok_or_else(|| NavigationError::MissingConstructor(self.display_name()))?
                    .cast::<FramePage>()
                    .ok_or_else(|| NavigationError::NotAPage(self.display_name()))
            }
        }
    }

    /// What equality and the hash compare: the identity of the class, or the runtime type of a
    /// type that has none.
    #[inline]
    fn key(&self) -> *const () {
        match (self.identity(), self.form) {
            (Some(identity), _) => identity as *const TypeIdentity as *const (),
            (None, Form::Reflective(type_)) => type_ as *const TypeInfo as *const (),
            (None, Form::Static { identity, .. }) => identity as *const TypeIdentity as *const (),
        }
    }
}

/// The page type of a runtime type, created by reflection ([`PageType::from_type_info`]); what the
/// run-time XAML loader, the designer and the tools give a frame for `{x:Type local:Page}`.
impl From<&'static TypeInfo> for PageType {
    #[inline]
    fn from(type_: &'static TypeInfo) -> Self {
        Self::reflective(type_)
    }
}

impl PartialEq for PageType {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for PageType {}

impl Hash for PageType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl fmt::Debug for PageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PageType").field(&self.name()).finish()
    }
}

/// The name of the class.
impl fmt::Display for PageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
