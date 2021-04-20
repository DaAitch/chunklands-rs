use super::util::{copy_extent_2d, copy_surface_format_khr};
use super::{
    error::{to_other, to_vulkan, Error, Result},
    vk_queue::QueueFamilyIndices,
};
use glfw::Window;
use vk_sys as vk;
use vulkanic::{DevicePointers, InstancePointers};

pub fn create_swapchain(
    ip: &InstancePointers,
    dp: &DevicePointers,
    physical_device: vk::PhysicalDevice,
    device: vk::Device,
    surface: vk::SurfaceKHR,
    window: &Window,
    queue_family_indices: &QueueFamilyIndices,
) -> Result<(
    vk::SwapchainKHR,
    vk::SurfaceFormatKHR,
    vk::PresentModeKHR,
    vk::Extent2D,
)> {
    let formats = ip
        .get_physical_device_surface_formats_khr(physical_device, surface)
        .map_err(to_vulkan)?;
    let modes = ip
        .get_physical_device_surface_present_modes_khr(physical_device, surface)
        .map_err(to_vulkan)?;

    let good_format = formats
        .iter()
        .find(|format| {
            format.format == vk::FORMAT_B8G8R8A8_SRGB
                && format.colorSpace == vk::COLOR_SPACE_SRGB_NONLINEAR_KHR
        })
        .or_else(|| formats.iter().next()) // first
        .ok_or_else(|| to_other(Error::Other("no good format found".to_owned())))?;
    let good_mode = modes
        .iter()
        .find(|mode| **mode == vk::PRESENT_MODE_MAILBOX_KHR)
        .unwrap_or(&vk::PRESENT_MODE_FIFO_KHR);

    let capabilities = ip
        .get_physical_device_surface_capabilities_khr(physical_device, surface)
        .map_err(to_vulkan)?;
    let extent = choose_swap_extent(&capabilities, window);

    let image_count = (capabilities.minImageCount + 1).min(capabilities.maxImageCount);
    let (image_sharing_mode, queue_families) =
        if queue_family_indices.graphics != queue_family_indices.present {
            (
                vk::SHARING_MODE_CONCURRENT,
                vec![queue_family_indices.graphics, queue_family_indices.present],
            )
        } else {
            (vk::SHARING_MODE_EXCLUSIVE, vec![])
        };

    let info = vk::SwapchainCreateInfoKHR {
        sType: vk::STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,
        pNext: std::ptr::null(),
        flags: 0,
        surface,
        minImageCount: image_count,
        imageFormat: good_format.format,
        imageColorSpace: good_format.colorSpace,
        imageExtent: copy_extent_2d(&extent),
        imageArrayLayers: 1,
        imageUsage: vk::IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
        imageSharingMode: image_sharing_mode,
        queueFamilyIndexCount: queue_families.len() as u32,
        pQueueFamilyIndices: queue_families.as_ptr(),
        preTransform: capabilities.currentTransform,
        compositeAlpha: vk::COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
        presentMode: *good_mode,
        clipped: vk::TRUE,
        oldSwapchain: vk::NULL_HANDLE,
    };

    let swapchain = unsafe { dp.create_swapchain_khr(device, &info) }.map_err(to_vulkan)?;
    let good_format: vk::SurfaceFormatKHR = copy_surface_format_khr(good_format);

    Ok((swapchain, good_format, *good_mode, extent))
}

fn choose_swap_extent(caps: &vk::SurfaceCapabilitiesKHR, window: &glfw::Window) -> vk::Extent2D {
    if caps.currentExtent.width != u32::MAX {
        return vk::Extent2D {
            width: caps.currentExtent.width,
            height: caps.currentExtent.height,
        };
    }

    let (w, h) = window.get_framebuffer_size();
    let w = w as u32;
    let h = h as u32;

    vk::Extent2D {
        width: w.clamp(caps.minImageExtent.width, caps.maxImageExtent.width),
        height: h.clamp(caps.minImageExtent.height, caps.maxImageExtent.height),
    }
}
