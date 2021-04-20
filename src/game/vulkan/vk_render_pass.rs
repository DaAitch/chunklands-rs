use super::error::{to_vulkan, Result};
use vk_sys as vk;
use vulkanic::DevicePointers;

pub fn create_render_pass(
    dp: &DevicePointers,
    device: vk::Device,
    format: &vk::SurfaceFormatKHR,
) -> Result<vk::RenderPass> {
    let color_attachment_desc = vk::AttachmentDescription {
        flags: 0,
        format: format.format,
        samples: vk::SAMPLE_COUNT_1_BIT,
        loadOp: vk::ATTACHMENT_LOAD_OP_CLEAR,
        storeOp: vk::ATTACHMENT_STORE_OP_STORE,
        stencilLoadOp: vk::ATTACHMENT_LOAD_OP_DONT_CARE,
        stencilStoreOp: vk::ATTACHMENT_STORE_OP_DONT_CARE,
        initialLayout: vk::IMAGE_LAYOUT_UNDEFINED,
        finalLayout: vk::IMAGE_LAYOUT_PRESENT_SRC_KHR,
    };

    let color_attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
    };

    let subpass_desc = vk::SubpassDescription {
        flags: 0,
        pipelineBindPoint: vk::PIPELINE_BIND_POINT_GRAPHICS,
        inputAttachmentCount: 0,
        pInputAttachments: std::ptr::null(),
        colorAttachmentCount: 1,
        pColorAttachments: &color_attachment_ref,
        pResolveAttachments: std::ptr::null(),
        pDepthStencilAttachment: std::ptr::null(),
        preserveAttachmentCount: 0,
        pPreserveAttachments: std::ptr::null(),
    };

    let subpass_dep = vk::SubpassDependency {
        srcSubpass: vk::SUBPASS_EXTERNAL,
        dstSubpass: 0,
        srcStageMask: vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
        dstStageMask: vk::PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
        srcAccessMask: 0,
        dstAccessMask: vk::ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
        dependencyFlags: 0,
    };

    let render_pass_info = vk::RenderPassCreateInfo {
        sType: vk::STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
        pNext: std::ptr::null(),
        flags: 0,
        attachmentCount: 1,
        pAttachments: &color_attachment_desc,
        subpassCount: 1,
        pSubpasses: &subpass_desc,
        dependencyCount: 1,
        pDependencies: &subpass_dep,
    };

    unsafe { dp.create_render_pass(device, &render_pass_info) }.map_err(to_vulkan)
}
