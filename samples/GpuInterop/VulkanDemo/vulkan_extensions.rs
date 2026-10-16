//! Port of `VulkanDemo/VulkanExtensions.cs` (namespace `SilkNetDemo` upstream): a result of
//! Vulkan that is not a success becomes an error.
//!
//! Upstream throws an exception, which `VulkanContext.TryCreate` and the initialization of the
//! demos catch and show, and which ends the application where nothing catches it (the drawing of
//! a frame). Here the error is a value that the same places turn into the text of the demo, and
//! the drawing of a frame panics with it.

use ash::vk;
use std::fmt;

/// An error of the Vulkan demos: the message of the exception upstream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VulkanDemoError(String);

impl VulkanDemoError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VulkanDemoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for VulkanDemoError {}

/// What the demos return: a value, or the error upstream throws.
pub type VulkanDemoResult<T> = Result<T, VulkanDemoError>;

/// `ThrowOnError`: a result of Vulkan other than `VK_SUCCESS` is an error.
pub trait ThrowOnError {
    type Output;

    fn throw_on_error(self) -> VulkanDemoResult<Self::Output>;
}

fn unexpected(result: vk::Result) -> VulkanDemoError {
    VulkanDemoError::new(format!("Unexpected API error \"{result:?}\"."))
}

impl ThrowOnError for vk::Result {
    type Output = ();

    fn throw_on_error(self) -> VulkanDemoResult<()> {
        if self != vk::Result::SUCCESS {
            return Err(unexpected(self));
        }
        Ok(())
    }
}

impl<T> ThrowOnError for ash::prelude::VkResult<T> {
    type Output = T;

    fn throw_on_error(self) -> VulkanDemoResult<T> {
        self.map_err(unexpected)
    }
}
