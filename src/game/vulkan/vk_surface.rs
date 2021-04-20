use super::error::{maybe_vulkan_error, Result};
use std::mem;

use vk_sys as vk;

pub fn create_surface(window: &glfw::Window, instance: vk::Instance) -> Result<vk::SurfaceKHR> {
    let mut surface = mem::MaybeUninit::<vk::SurfaceKHR>::uninit();
    let result = window.create_window_surface(instance, std::ptr::null(), surface.as_mut_ptr());
    maybe_vulkan_error(result)?;

    Ok(unsafe { surface.assume_init() })
}
