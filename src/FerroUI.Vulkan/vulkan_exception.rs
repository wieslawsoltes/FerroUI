use ash::vk;
use std::fmt;

/// A failure reported by Vulkan or by the code driving it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VulkanError {
    message: String,
    result: Option<vk::Result>,
}

/// The name of [`VulkanError`] before 0.5.
#[deprecated(since = "0.5.0", note = "use `VulkanError`")]
pub type VulkanException = VulkanError;

impl VulkanError {
    /// A failure without a result of Vulkan.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), result: None }
    }

    /// The failure of the function `func_name` with the numeric result
    /// `res`.
    pub fn from_code(func_name: &str, res: i32) -> Self {
        Self::from_result(func_name, vk::Result::from_raw(res))
    }

    /// The failure of the function `func_name` with the result `res`.
    pub fn from_result(func_name: &str, res: vk::Result) -> Self {
        Self { message: format!("{func_name} returned {}", result_name(res)), result: Some(res) }
    }

    /// The message of the failure.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The result of Vulkan, when the failure was reported by Vulkan.
    pub fn result(&self) -> Option<vk::Result> {
        self.result
    }

    /// The failure of the function `func_name`, unless its numeric result
    /// `res` is `VK_SUCCESS`.
    pub fn throw_on_error(func_name: &str, res: i32) -> Result<(), VulkanError> {
        vk::Result::from_raw(res).throw_on_error(func_name)
    }
}

impl fmt::Display for VulkanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for VulkanError {}

/// The name of a result as the Vulkan headers spell it.
fn result_name(res: vk::Result) -> String {
    let name = format!("{res:?}");
    // A value the bindings have no name for prints as its number.
    if name.starts_with(|c: char| c.is_ascii_uppercase()) {
        format!("VK_{name}")
    } else {
        res.as_raw().to_string()
    }
}

/// Turns every result but `VK_SUCCESS` into a failure.
pub trait VulkanExceptionExtensions {
    /// The failure of the function `func_name`, unless the result is
    /// `VK_SUCCESS`.
    fn throw_on_error(self, func_name: &str) -> Result<(), VulkanError>;
}

impl VulkanExceptionExtensions for vk::Result {
    fn throw_on_error(self, func_name: &str) -> Result<(), VulkanError> {
        if self != vk::Result::SUCCESS {
            return Err(VulkanError::from_result(func_name, self));
        }
        Ok(())
    }
}

/// Names the function whose result a failure is.
pub trait VulkanResultExtensions<T> {
    /// The value, or the failure of the function `func_name`.
    fn throw_on_error(self, func_name: &str) -> Result<T, VulkanError>;
}

impl<T> VulkanResultExtensions<T> for Result<T, vk::Result> {
    fn throw_on_error(self, func_name: &str) -> Result<T, VulkanError> {
        self.map_err(|res| VulkanError::from_result(func_name, res))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_is_no_failure() {
        assert_eq!(vk::Result::SUCCESS.throw_on_error("vkCreateFence"), Ok(()));
        assert_eq!(VulkanError::throw_on_error("vkCreateFence", 0), Ok(()));
    }

    #[test]
    fn a_result_is_named_as_in_the_headers() {
        let error = vk::Result::ERROR_OUT_OF_DATE_KHR.throw_on_error("vkAcquireNextImageKHR").unwrap_err();
        assert_eq!(error.message(), "vkAcquireNextImageKHR returned VK_ERROR_OUT_OF_DATE_KHR");
        assert_eq!(error.result(), Some(vk::Result::ERROR_OUT_OF_DATE_KHR));
        assert_eq!(error.to_string(), error.message());
    }

    #[test]
    fn a_positive_result_is_a_failure_too() {
        // The reference compares with VK_SUCCESS, so VK_SUBOPTIMAL_KHR and VK_INCOMPLETE fail.
        let error = VulkanError::throw_on_error("vkQueuePresentKHR", 1000001003).unwrap_err();
        assert_eq!(error.message(), "vkQueuePresentKHR returned VK_SUBOPTIMAL_KHR");
    }

    #[test]
    fn a_result_without_a_name_prints_its_number() {
        let error = VulkanError::from_code("vkFoo", -123456);
        assert_eq!(error.message(), "vkFoo returned -123456");
    }

    #[test]
    fn a_plain_message_has_no_result() {
        let error = VulkanError::new("No devices found");
        assert_eq!(error.message(), "No devices found");
        assert_eq!(error.result(), None);
    }
}
