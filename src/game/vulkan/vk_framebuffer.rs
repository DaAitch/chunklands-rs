use super::error::{to_vulkan, Result};
use vk_sys as vk;
use vulkanic::DevicePointers;

pub fn create_framebuffer(
    dp: &DevicePointers,
    device: vk::Device,
    render_pass: vk::RenderPass,
    image_view: vk::ImageView,
    extent: &vk::Extent2D,
) -> Result<vk::Framebuffer> {
    let attachments = [image_view];

    let create_info = vk::FramebufferCreateInfo {
        sType: vk::STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        renderPass: render_pass,
        attachmentCount: attachments.len() as u32,
        pAttachments: attachments.as_ptr(),
        width: extent.width,
        height: extent.height,
        layers: 1,
    };

    unsafe { dp.create_framebuffer(device, &create_info) }.map_err(to_vulkan)
}
