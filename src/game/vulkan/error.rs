use std::fmt;
use vk_sys as vk;

pub fn maybe_vulkan_error(error_code: u32) -> Result<()> {
    if error_code != vk::SUCCESS {
        Err(Error::VulkanError(error_code))
    } else {
        Ok(())
    }
}

pub fn to_other<E: fmt::Display>(err: E) -> Error {
    Error::Other(format!("{}", err))
}

pub fn to_vulkan(error_result: vk::Result) -> Error {
    Error::VulkanError(error_result)
}

pub enum Error {
    VulkanError(u32),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::VulkanError(error_code) => {
                let name = match *error_code {
                    vk::NOT_READY => "NOT_READY",
                    vk::TIMEOUT => "TIMEOUT",
                    vk::EVENT_SET => "EVENT_SET",
                    vk::EVENT_RESET => "EVENT_RESET",
                    vk::INCOMPLETE => "INCOMPLETE",
                    vk::ERROR_OUT_OF_HOST_MEMORY => "ERROR_OUT_OF_HOST_MEMORY",
                    vk::ERROR_OUT_OF_DEVICE_MEMORY => "ERROR_OUT_OF_DEVICE_MEMORY",
                    vk::ERROR_INITIALIZATION_FAILED => "ERROR_INITIALIZATION_FAILED",
                    vk::ERROR_DEVICE_LOST => "ERROR_DEVICE_LOST",
                    vk::ERROR_MEMORY_MAP_FAILED => "ERROR_MEMORY_MAP_FAILED",
                    vk::ERROR_LAYER_NOT_PRESENT => "ERROR_LAYER_NOT_PRESENT",
                    vk::ERROR_EXTENSION_NOT_PRESENT => "ERROR_EXTENSION_NOT_PRESENT",
                    vk::ERROR_FEATURE_NOT_PRESENT => "ERROR_FEATURE_NOT_PRESENT",
                    vk::ERROR_INCOMPATIBLE_DRIVER => "ERROR_INCOMPATIBLE_DRIVER",
                    vk::ERROR_TOO_MANY_OBJECTS => "ERROR_TOO_MANY_OBJECTS",
                    vk::ERROR_FORMAT_NOT_SUPPORTED => "ERROR_FORMAT_NOT_SUPPORTED",
                    vk::ERROR_SURFACE_LOST_KHR => "ERROR_SURFACE_LOST_KHR",
                    vk::ERROR_NATIVE_WINDOW_IN_USE_KHR => "ERROR_NATIVE_WINDOW_IN_USE_KHR",
                    vk::SUBOPTIMAL_KHR => "SUBOPTIMAL_KHR",
                    vk::ERROR_OUT_OF_DATE_KHR => "ERROR_OUT_OF_DATE_KHR",
                    vk::ERROR_INCOMPATIBLE_DISPLAY_KHR => "ERROR_INCOMPATIBLE_DISPLAY_KHR",
                    vk::ERROR_VALIDATION_FAILED_EXT => "ERROR_VALIDATION_FAILED_EXT",
                    vk::ERROR_INVALID_SHADER_NV => "ERROR_INVALID_SHADER_NV",
                    vk::ERROR_OUT_OF_POOL_MEMORY_KHR => "ERROR_OUT_OF_POOL_MEMORY_KHR",
                    vk::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT => {
                        "ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT"
                    }
                    _ => "unknown vulkan error",
                };

                write!(f, "Vulkan error: {}", name)
            }
            Error::Other(text) => {
                write!(f, "Other error: {}", text)
            }
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
