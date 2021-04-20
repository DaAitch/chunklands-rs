use super::error::{to_vulkan, Result};
use vk_sys as vk;
use vulkanic::DevicePointers;

pub fn create_image_view(
    dp: &DevicePointers,
    device: vk::Device,
    image: vk::Image,
    format: vk::Format,
) -> Result<vk::ImageView> {
    let info = vk::ImageViewCreateInfo {
        sType: vk::STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        image,
        viewType: vk::IMAGE_VIEW_TYPE_2D,
        format,
        components: vk::ComponentMapping {
            r: vk::COMPONENT_SWIZZLE_IDENTITY,
            g: vk::COMPONENT_SWIZZLE_IDENTITY,
            b: vk::COMPONENT_SWIZZLE_IDENTITY,
            a: vk::COMPONENT_SWIZZLE_IDENTITY,
        },
        subresourceRange: vk::ImageSubresourceRange {
            aspectMask: vk::IMAGE_ASPECT_COLOR_BIT,
            baseMipLevel: 0,
            levelCount: 1,
            baseArrayLayer: 0,
            layerCount: 1,
        },
    };

    unsafe { dp.create_image_view(device, &info) }.map_err(to_vulkan)
}
