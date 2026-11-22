//! Authoring values for the native ordered transform list.

mod operation;

use crate::{AnimationError, Result, require};
use ferroui_base::animation::{Animatable, animators::TransformListAnimator};
use ferroui_base::media::{ITransform, transformation::TransformOperations};
use ferroui_base::{BoxedValue, FerroProperty, FerroPropertyRegistry};
use serde::{Deserialize, Serialize};
use std::{any::TypeId, rc::Rc};

pub use operation::{AnimationTransformOperation, TransformOperationKind};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationTransformList {
    operations: Vec<AnimationTransformOperation>,
    #[serde(skip)]
    spelling: Option<String>,
}

impl PartialEq for AnimationTransformList {
    fn eq(&self, other: &Self) -> bool {
        self.operations == other.operations
    }
}

impl AnimationTransformList {
    pub fn is_property(property: &FerroProperty) -> bool {
        property.property_type() == TypeId::of::<Option<Rc<dyn ITransform>>>()
    }

    pub fn base_value(target: &Animatable, property: &'static FerroProperty) -> Option<Self> {
        if !FerroPropertyRegistry::instance().is_registered_for(target, property) {
            return None;
        }
        let value = TransformListAnimator::base_value(target, property.as_styled()?);
        Self::from_native(value.as_deref()).ok()
    }

    pub fn parse(source: &str) -> Result<Self> {
        let native = TransformOperations::parse(source)
            .map_err(|error| AnimationError(error.to_string()))?;
        let mut list = Self::from_native(Some(&*native))?;
        list.spelling = Some(source.to_owned());
        Ok(list)
    }

    pub fn from_native(transform: Option<&dyn ITransform>) -> Result<Self> {
        let operations = match transform {
            None => Vec::new(),
            Some(value) => {
                if let Some(list) = value.as_any().downcast_ref::<TransformOperations>() {
                    list.operations()
                        .iter()
                        .map(AnimationTransformOperation::from_operation)
                        .collect::<Result<Vec<_>>>()?
                } else {
                    vec![AnimationTransformOperation::from_matrix(value.value())?]
                }
            }
        };
        let list = Self {
            operations,
            spelling: None,
        };
        list.validate()?;
        Ok(list)
    }

    pub fn from_boxed(value: &BoxedValue) -> Option<Result<Self>> {
        let value = value.downcast_ref::<Option<Rc<dyn ITransform>>>()?;
        Some(Self::from_native(value.as_deref()))
    }

    pub fn operations(&self) -> &[AnimationTransformOperation] {
        &self.operations
    }

    pub fn to_native(&self) -> Rc<TransformOperations> {
        if self.operations.is_empty() {
            return TransformOperations::identity();
        }
        let mut builder = TransformOperations::create_builder(self.operations.len());
        for operation in &self.operations {
            operation.append(&mut builder);
        }
        builder.build()
    }

    pub fn to_transform(&self) -> Option<Rc<dyn ITransform>> {
        Some(self.to_native())
    }

    pub(crate) fn validate(&self) -> Result<()> {
        require(
            self.operations
                .iter()
                .all(AnimationTransformOperation::is_finite),
            "Transform operation values must be finite",
        )?;
        require(
            operation::matrix_is_finite(self.to_native().value()),
            "The combined transform matrix must be finite",
        )
    }

    pub(crate) fn interpolate(&self, next: &Self, progress: f64) -> Result<Self> {
        let value = TransformListAnimator::interpolate_value(
            progress,
            &self.to_transform(),
            &next.to_transform(),
        );
        Self::from_native(value.as_deref())
    }

    pub fn to_xaml(&self) -> String {
        if let Some(spelling) = &self.spelling {
            return spelling.clone();
        }
        if self.operations.is_empty() {
            return "none".into();
        }
        self.operations
            .iter()
            .map(AnimationTransformOperation::to_xaml)
            .collect::<Vec<_>>()
            .join(" ")
    }
}
