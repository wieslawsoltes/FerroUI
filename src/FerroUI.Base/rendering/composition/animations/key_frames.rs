use crate::animation::easings::IEasing;
use crate::rendering::composition::expressions::Expression;
use std::any::Any;
use std::rc::Rc;

/// Collection of composition animation key frames
pub struct KeyFrames<T> {
    items: Vec<KeyFrame<T>>,
}

impl<T> Default for KeyFrames<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T: Copy + Default + 'static> KeyFrames<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }

    pub fn items(&self) -> &[KeyFrame<T>] {
        &self.items
    }

    fn validate(&self, key: f32) {
        if key < 0.0 || key > 1.0 {
            panic!("Key frame key");
        }
        if let Some(last) = self.items.last() {
            if last.normalized_progress_key > key {
                panic!("Key frame key {key} is less than the previous one");
            }
        }
    }

    pub fn insert_expression_key_frame(&mut self, normalized_progress_key: f32, value: &str, easing_function: Rc<dyn IEasing>) {
        self.validate(normalized_progress_key);
        let expression = match Expression::parse(value) {
            Ok(expression) => expression,
            Err(error) => panic!("{error}"),
        };
        self.items.push(KeyFrame {
            normalized_progress_key,
            value: T::default(),
            expression: Some(std::sync::Arc::new(expression)),
            easing_function,
        });
    }

    pub fn insert(&mut self, normalized_progress_key: f32, value: T, easing_function: Rc<dyn IEasing>) {
        self.validate(normalized_progress_key);
        self.items.push(KeyFrame { normalized_progress_key, value, expression: None, easing_function });
    }

    pub fn snapshot(&self) -> Vec<ServerKeyFrame<T>> {
        self.items
            .iter()
            .map(|f| ServerKeyFrame {
                expression: f.expression.clone(),
                value: f.value,
                easing_function: Some(f.easing_function.to_shared()),
                key: f.normalized_progress_key,
            })
            .collect()
    }
}

/// Composition animation key frame
#[derive(Clone)]
pub struct KeyFrame<T> {
    pub normalized_progress_key: f32,
    pub value: T,
    /// The expression of an expression key frame (`null` upstream for a
    /// value key frame).
    pub expression: Option<std::sync::Arc<Expression>>,
    pub easing_function: Rc<dyn IEasing>,
}

/// Server-side composition animation key frame
#[derive(Clone, Default)]
pub struct ServerKeyFrame<T> {
    pub value: T,
    pub expression: Option<std::sync::Arc<Expression>>,
    /// Always set on a key frame of a snapshot; `None` only on the frame an
    /// instance builds for the starting value, whose easing is never used.
    /// The easing in the form the render thread evaluates.
    pub easing_function: Option<std::sync::Arc<crate::animation::easings::SharedEasing>>,
    pub key: f32,
}

/// The key frames of a key frame animation, whatever their value type.
pub trait IKeyFrames: Any {
    fn insert_expression_key_frame(&mut self, normalized_progress_key: f32, value: &str, easing_function: Rc<dyn IEasing>);

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Copy + Default + 'static> IKeyFrames for KeyFrames<T> {
    fn insert_expression_key_frame(&mut self, normalized_progress_key: f32, value: &str, easing_function: Rc<dyn IEasing>) {
        KeyFrames::insert_expression_key_frame(self, normalized_progress_key, value, easing_function)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
